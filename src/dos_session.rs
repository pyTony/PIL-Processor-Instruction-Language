//! Interactive DOS session over stdin/stdout for a frontend: one command per
//! line, one JSON reply per line. The frontend draws text and graphics buffers
//! and sends keys; all execution stays in the PIL interpreter and Dine.
use super::*;
use std::io::{BufRead,Write as _};
const MAX_LINE:usize=1024;
fn parse_drive_letter(text:&str)->Option<u8>{
    match text.as_bytes(){
        [l]|[l,b':'] if l.is_ascii_alphabetic()=>Some(l.to_ascii_uppercase()),
        _=>None,
    }
}
pub(crate) fn open_mount_source(path:&str)->Result<Source,String>{
    let normalized=crate::dos_config::host_path(path);let p=normalized.as_path();
    let is_image=p.extension().and_then(|x|x.to_str()).is_some_and(|x|x.eq_ignore_ascii_case("img")||x.eq_ignore_ascii_case("ima"));
    if is_image&&p.is_file(){
        let data=super::super::containers::read_image(p)?;
        return Ok(Source::Image(std::sync::Arc::new(super::super::containers::fat16_tree(&data)?)));
    }
    if p.is_dir(){return Ok(Source::Host(p.to_path_buf()));}
    Err(format!("mount {path} is not a folder or .img/.ima image"))
}
pub struct Session{loaded:Option<Loaded>,stop:Stop,steps:usize,started:bool,exceptions:Vec<ExceptionSite>,exception_count:usize,idle_hint:bool,backend:Option<Box<dyn InstructionBackend>>}
impl Session{
    pub fn new(loaded:Loaded)->Self{Self{loaded:Some(loaded),stop:Stop::Budget,steps:0,started:false,exceptions:Vec::new(),exception_count:0,idle_hint:false,backend:None}}
    /// Keep the executor for the whole session, including across frontend slices.
    #[allow(dead_code)]
    pub(crate) fn with_backend(loaded:Loaded,backend:Box<dyn InstructionBackend>)->Self{
        let mut session=Self::new(loaded);session.backend=Some(backend);session
    }
    fn live(&self)->bool{matches!(self.stop,Stop::Budget|Stop::WaitingForInput)}
    /// Runs up to `budget` steps; output already reaches the screen through Dine.
    pub fn run(&mut self,budget:usize)->Result<Vec<u8>,String>{
        if !self.live(){return Ok(Vec::new());}
        self.started=true;
        let loaded=self.loaded.take().ok_or("session state lost")?;
        let mut output=crate::host_io::CaptureOutput::new(crate::host_io::DOS_OUTPUT_LIMIT);
        let mut probe=match self.backend.as_mut(){
            Some(backend)=>run_with_backend(loaded,budget,&mut output,Some(backend.as_mut()))?,
            None=>run_with_backend(loaded,budget,&mut output,None)?,
        };
        probe.output=output.into_bytes();
        // A hint only: polling programs continue running, but a frontend may
        // reduce its execution rate while the screen is unchanged.
        let polls=probe.instructions.get("DOS INT16/AH01").copied().unwrap_or(0)+probe.instructions.get("DOS INT16/AH11").copied().unwrap_or(0);
        self.idle_hint=probe.completed>=1000&&polls.saturating_mul(200)>probe.completed&&probe.output.is_empty();
        self.exception_count+=probe.exception_count;
        self.exceptions.extend(probe.exceptions.into_iter().take(64-self.exceptions.len()));
        self.steps+=probe.completed;self.stop=probe.stop;self.loaded=Some(probe.loaded);
        Ok(probe.output)
    }
    #[cfg(test)]
    pub fn loaded(&self)->Option<&Loaded>{self.loaded.as_ref()}
    /// Queues one key as the BIOS stores it: scan code in the high byte, ASCII low.
    pub fn key(&mut self,ascii:u8,scan:u8)->Result<bool,String>{
        let p=self.loaded.as_mut().ok_or("session state lost")?;
        let word=u16::from(scan)<<8|u16::from(ascii);
        // A program's own INT 09h handler sees the key arrive through IRQ1.
        if keyboard::hooked(p)?{return keyboard::key(p,word);}
        // The BIOS ring stands behind INT 09h only; IRQ1 moved elsewhere
        // with no program handler there is not modelled.
        let vector=p.speaker.pic.keyboard_vector();
        if vector!=0x09{return Err(format!("IRQ1 through INT {vector:02X}h (PIC base moved), which has no program handler, is not modelled"));}
        console::enqueue(p,word)
    }
    fn key_edge(&mut self,ascii:u8,scan:u8,down:bool)->Result<bool,String>{
        let p=self.loaded.as_mut().ok_or("session state lost")?;
        if keyboard::hooked(p)?{return keyboard::edge(p,u16::from(scan)<<8|u16::from(ascii),down);}
        if down{self.key(ascii,scan)}else{Ok(true)}
    }
    /// The active text page as character/attribute pairs, its width and the cursor.
    pub fn screen(&self)->Result<(Vec<u8>,u32,(u32,u32)),String>{
        let p=self.loaded.as_ref().ok_or("session state lost")?;
        let byte=|at|p.ram.load(at,MemWidth::Byte).map_err(|e|format!("screen read: {e:?}"));
        let word=|at|p.ram.load(at,MemWidth::Word).map_err(|e|format!("screen read: {e:?}"));
        let(columns,page)=(word(0x44a)?.clamp(1,80),byte(0x462)?&7);
        let origin=if p.vga.start_override{u32::from(p.vga.start())*2}else{page*word(0x44c)?};
        let base=0xb8000;
        let cells=(0..columns*25*2).map(|i|byte(base+(origin+i)%32768).map(|b|b as u8)).collect::<Result<Vec<_>,_>>()?;
        let cursor=if p.vga.cursor_override{let cell=u32::from(p.vga.cursor()).wrapping_sub(origin/2)&0x3fff;((cell/columns)<<8)|(cell%columns)}else{word(0x450+page*2)?};
        Ok((cells,columns,(cursor>>8,cursor&0xff)))
    }
    fn status(&self)->(&'static str,String){
        match &self.stop{
            Stop::Budget=>("running",String::new()),Stop::WaitingForInput=>("waiting_input",String::new()),
            Stop::Exit(code)=>("exit",format!("exit code {code}")),
            Stop::MissingEncoding{key,detail}=>("stopped",format!("missing encoding {key}: {detail}")),
            Stop::Service(m)=>("stopped",format!("unimplemented service: {m}")),
            Stop::Fault(m)=>("stopped",format!("fault: {m}")),Stop::Boundary(m)=>("stopped",format!("boundary: {m}")),
            Stop::Monitor(m)=>("stopped",format!("monitor stop: {m}")),
        }
    }
    /// The reply line; it hands the frontend what the program sent to COM1
    /// since the last reply.
    fn reply(&mut self,seq:u64,ok:bool,text:&str,output:&[u8])->Result<String,String>{
        let com1:String=self.loaded.as_mut().and_then(|p|p.com1.as_mut()).map_or(Vec::new(),|s|std::mem::take(&mut s.tx)).iter().map(|b|format!("{b:02x}")).collect();
        let(cells,columns,(row,column))=self.screen()?;let(status,detail)=self.status();
        let hex:String=cells.iter().map(|b|format!("{b:02x}")).collect();
        let cs=self.loaded.as_ref().map(|p|p.selectors[SegmentRegister::Cs.index()]).unwrap_or(0);
        let diagnostic=self.loaded.as_ref().map(|p|{
            let mut bytes=Vec::new();
            for n in 0..15{match p.image.fetch_signature(&p.ram,p.cpu.eip,n+1){Ok(b)=>bytes.push(b[n as usize]),Err(_)=>break}}
            format!("\"instruction_bytes\":\"{}\",\"registers\":{:?},\"selectors\":{:?},\"eflags\":{},\"msw\":{}",bytes.iter().map(|b|format!("{b:02X}")).collect::<Vec<_>>().join(" "),p.cpu.regs,p.selectors,p.cpu.eflags,p.image.msw)
        }).unwrap_or_else(||"\"instruction_bytes\":\"\"".into());
        let graphics=self.loaded.as_ref().and_then(super::planar::mode13_snapshot).map_or(String::new(),|g|format!(",\"graphics13\":\"{g}\",\"palette\":\"{}\"",self.loaded.as_ref().unwrap().vga.palette_hex()));
        let cga=self.loaded.as_ref().and_then(|p|match p.ram.load(0x449,MemWidth::Byte).ok(){
            Some(bios_mode @ (4|5|6))=>{let mode=if bios_mode==6{6}else if p.vga.cga_mode()&4!=0{5}else{4};Some(format!(",\"graphics_cga\":\"{}\",\"cga_mode\":{mode},\"cga_color\":{}",
                p.ram.bytes()[0xb8000..0xbc000].iter().map(|b|format!("{b:02x}")).collect::<String>(),p.vga.cga_color()))},
            _=>None,
        }).unwrap_or_default();
        // 16-colour planar modes: each plane's used bytes, then the 16 RGB colours.
        let planar=self.loaded.as_ref().and_then(super::planar::snapshot).map(|(mode,g,planes,rgb)|
            format!(",\"graphics_planar\":\"{planes}\",\"planar_mode\":{mode},\"planar_width\":{},\"planar_height\":{},\"planar_rgb\":\"{rgb}\"",g.width,g.height)).unwrap_or_default();
        let raw_keyboard=self.loaded.as_ref().is_some_and(|p|keyboard::hooked(p).unwrap_or(false));
        let eip=self.loaded.as_ref().map(|p|p.cpu.eip).unwrap_or(0);
        // The speaker: times it was switched on so far, and its tone now (0 off).
        let(beeps,tone)=self.loaded.as_ref().map_or((0,0),|p|(p.speaker.beeps,p.speaker.tone()));
        let(blocks,native)=self.backend.as_ref().map_or((0,0),|b|b.block_stats());
        let mode_control=self.loaded.as_ref().map_or(0x29,|p|p.vga.cga_mode());
        let diagnostic=format!("\"video_enabled\":{},\"text_blink\":{},\"cga_mode_control\":{mode_control},{diagnostic},\"native_blocks\":{blocks},\"native_block_instructions\":{native}",mode_control&8!=0,mode_control&0x20!=0);
        let fpu_present=self.loaded.as_ref().is_some_and(|p|p.fpu.available());
        let diagnostic=format!("{diagnostic},\"fpu_present\":{fpu_present}");
        Ok(format!("{{\"protocol\":1,\"buffered_run\":true,\"key_edges\":true,\"raw_keyboard\":{raw_keyboard},\"seq\":{seq},\"ok\":{ok},\"text\":{},\"status\":\"{status}\",\"detail\":{},\"steps\":{},\"idle_hint\":{},\"cs\":{cs},\"eip\":{eip},{diagnostic},\"columns\":{columns},\"cursor\":[{row},{column}],\"screen\":\"{hex}\"{graphics}{cga}{planar},\"beeps\":{beeps},\"tone\":{tone},\"com1\":\"{com1}\",\"output\":{}}}",
            quote(text),quote(&detail),self.steps,self.idle_hint,quote(&crate::dos_encoding::decode(output))))
    }
    /// One protocol command: `run N`, `key ASCII SCAN`, `break`, `clock OFFSET`, `snapshot` or `quit`.
    pub fn command(&mut self,seq:u64,line:&str)->Result<(String,bool),String>{
        let words:Vec<&str>=line.split_whitespace().collect();
        // Execute without serializing a framebuffer. The frontend requests a
        // full snapshot when it is ready to present the latest buffered state.
        if let ["run",n,"buffered"]=words.as_slice(){
            if let Ok(n)=n.parse::<usize>(){if (1..=MAX_STEPS).contains(&n){
                let output=self.run(n)?;let(status,detail)=self.status();
                let p=self.loaded.as_mut().ok_or("session state lost")?;
                let com1= p.com1.as_mut().map_or(Vec::new(),|s|std::mem::take(&mut s.tx));
                let hex:String=com1.iter().map(|b|format!("{b:02x}")).collect();
                return Ok((format!("{{\"protocol\":1,\"seq\":{seq},\"ok\":true,\"frame_pending\":true,\"status\":\"{status}\",\"detail\":{},\"steps\":{},\"idle_hint\":{},\"beeps\":{},\"tone\":{},\"com1\":\"{hex}\",\"output\":{}}}",quote(&detail),self.steps,self.idle_hint,p.speaker.beeps,p.speaker.tone(),quote(&crate::dos_encoding::decode(&output))),false));
            }}
        }
        let mut output=Vec::new();
        let(ok,text,quit)=match words.as_slice(){
            []|["snapshot"]=>(true,String::new(),false),
            ["fpu",mode @ ("none"|"80387")]=>{
                if self.started{(false,"FPU selection requires a fresh session before the first run".into(),false)}
                else{
                    let p=self.loaded.as_mut().ok_or("session state lost")?;
                    p.select_fpu(*mode=="80387")?;
                    (true,format!("FPU: {}",p.fpu.name()),false)
                }
            },
            ["fpu",..]=>(false,"fpu requires none or 80387".into(),false),
            ["run",n]=>match n.parse::<usize>(){
                Ok(n) if (1..=MAX_STEPS).contains(&n)=>{output=self.run(n)?;(true,String::new(),false)},
                _=>(false,"run budget must be 1..1000000".into(),false),
            },
            [direction @ ("keydown"|"keyup"),a,s]=>match(a.parse::<u8>(),s.parse::<u8>()){
                (Ok(a),Ok(s))=>if self.key_edge(a,s,*direction=="keydown")?{(true,String::new(),false)}else{(false,"keyboard scan codes still waiting for the program".into(),false)},
                _=>(false,"key edge needs ASCII and scan code bytes".into(),false),
            },
            ["key",a,s]=>match(a.parse::<u8>(),s.parse::<u8>()){
                (Ok(a),Ok(s))=>if self.key(a,s)?{(true,String::new(),false)}else{(false,"BIOS keyboard buffer full".into(),false)},
                _=>(false,"key needs ASCII and scan code bytes".into(),false),
            },
            // `shift FLAGS`: the shift keys held now, as BIOS 0040:0017 bits 0-3
            // (right Shift, left Shift, Ctrl, Alt); the toggle bits are kept.
            ["shift",flags]=>match flags.parse::<u8>(){
                Ok(flags) if flags<16=>{
                    let p=self.loaded.as_mut().ok_or("session state lost")?;
                    let old=p.ram.load(0x417,MemWidth::Byte).map_err(|e|format!("keyboard flags: {e:?}"))?as u8;
                    p.ram.store(0x417,MemWidth::Byte,u32::from(old&0xf0|flags)).map_err(|e|format!("keyboard flags: {e:?}"))?;
                    (true,String::new(),false)
                },
                _=>(false,"shift needs flags 0..15".into(),false),
            },
            // `clock OFFSET`: run the clock on host time, OFFSET seconds east of UTC.
            ["clock",offset]=>match offset.parse::<i64>(){
                Ok(offset) if offset.abs()<=14*3600=>{self.loaded.as_mut().ok_or("session state lost")?.clock_offset=Some(offset);(true,String::new(),false)},
                _=>(false,"clock needs a UTC offset in seconds (-50400..50400)".into(),false),
            },
            // `country CODE`: the DOS country, as AH38 with DX=FFFFh would set it.
            ["country",code]=>match code.parse::<u16>(){
                Ok(code) if COUNTRIES.contains(&code)=>{self.loaded.as_mut().ok_or("session state lost")?.country=code;(true,String::new(),false)},
                _=>(false,"country needs 1 or 358".into(),false),
            },
            // `ansi on`: load the ANSI.SYS-style CON driver, as DEVICE=ANSI.SYS would.
            ["ansi","on"]=>{super::console::ansi::install(self.loaded.as_mut().ok_or("session state lost")?)?;(true,String::new(),false)},
            ["console","com1"]=>{
                let p=self.loaded.as_mut().ok_or("session state lost")?;
                if self.steps!=0{return Err("console com1 must be selected before execution".into());}
                if p.com1.is_none(){return Err("COM1 is not attached".into());}
                let table=u32::from(p.psp)*16+0x18;
                p.ram.check_write(table,MemWidth::Word).map_err(|e|format!("console handles: {e:?}"))?;
                p.ram.store(table,MemWidth::Word,0x0303).map_err(|e|format!("console handles: {e:?}"))?;
                p.com1.as_mut().unwrap().info|=0x23;
                (true,String::new(),false)
            },
            // `mount DRIVE PATH`: mount or replace drive A..E with a host folder or .img/.ima image.
            ["mount",drive,..]=>{
                let path=line.trim_start().strip_prefix("mount").unwrap_or("").trim_start().strip_prefix(drive).unwrap_or("").trim();
                match parse_drive_letter(drive){
                    Some(letter) if !path.is_empty()=>{
                        let p=self.loaded.as_mut().ok_or("session state lost")?;
                        match open_mount_source(path).and_then(|src|p.drive.mount(letter,src)){
                            Ok(())=>(true,format!("{}: {path}",char::from(letter)),false),
                            Err(e)=>(false,e,false),
                        }
                    },
                    _=>(false,"mount needs drive A..E and a folder or .img/.ima path".into(),false),
                }
            },
            ["mount"]=>(false,"mount needs drive A..E and a folder or .img/.ima path".into(),false),
            // `unmount DRIVE`: detach drive A..E when no files are open on it.
            ["unmount",drive]=>match parse_drive_letter(drive){
                Some(letter)=>{
                    let p=self.loaded.as_mut().ok_or("session state lost")?;
                    match p.drive.unmount(letter){
                        Ok(())=>(true,format!("{}: unmounted",char::from(letter)),false),
                        Err(e)=>(false,e,false),
                    }
                },
                None=>(false,"unmount needs drive A..E".into(),false),
            },
            ["unmount"]=>(false,"unmount needs drive A..E".into(),false),
            // `com1 HEX`: bytes arriving at COM1 from the host terminal.
            ["com1",hex]=>match(0..hex.len()).step_by(2).map(|i|hex.get(i..i+2).and_then(|b|u8::from_str_radix(b,16).ok())).collect::<Option<Vec<u8>>>(){
                Some(bytes) if !bytes.is_empty()=>match self.loaded.as_mut().and_then(|p|p.com1.as_mut()){
                    Some(port)=>{port.rx.extend(bytes);(true,String::new(),false)},
                    None=>(false,"COM1 is not attached".into(),false),
                },
                _=>(false,"com1 needs bytes in hex".into(),false),
            },
            // `break`: Ctrl+Break, as the BIOS and DOS's INT 1Bh handler take it.
            // When the program owns INT 09h or INT 1Bh, Ctrl and Scroll Lock
            // arrive through IRQ1 and the BIOS keyboard handler calls INT 1Bh.
            ["break"]=>{
                let p=self.loaded.as_mut().ok_or("session state lost")?;
                let through_irq=keyboard::hooked(p)?||(super::vectors::answered(p,p.speaker.pic.keyboard_vector())?&&super::vectors::reachable(p,0x1b)?);
                if through_irq{
                    match keyboard::ctrl_break(p)?{
                        true=>(true,String::new(),false),
                        false=>(false,"keyboard scan codes still waiting for the program".into(),false),
                    }
                }else{match brk::ctrl_break(p){
                    Ok(())=>(true,String::new(),false),
                    Err(message)=>(false,message,false),
                }}
            },
            ["quit"]=>(true,String::new(),true),
            _=>(false,format!("unknown command: {line}"),false),
        };
        Ok((self.reply(seq,ok,&text,&output)?,quit))
    }
}
pub(crate) fn quote(text:&str)->String{
    let mut out=String::from("\"");
    for c in text.chars(){match c{'"'=>out.push_str("\\\""),'\\'=>out.push_str("\\\\"),'\n'=>out.push_str("\\n"),'\r'=>out.push_str("\\r"),'\t'=>out.push_str("\\t"),c if (c as u32)<0x20=>out.push_str(&format!("\\u{:04x}",c as u32)),c=>out.push(c)}}
    out.push('"');out
}
/// `--dos-session [--mount DIR] [--args TEXT] PATH`: PATH is a file or
/// IMAGE::INNER inside a container. A FAT16 image becomes read-only drive C:
/// and DIR the next drive; TEXT is the program's command line.
pub fn serve(path:&str,mount:Option<&str>,args:Option<&str>,log_path:&str,config:Option<&str>,fpu_mode:Option<&str>)->Result<(),String>{
    let mut logger=stoplog::Logger::new(log_path,path);
    let containers=super::super::containers::split_container_path(path);
    let mut drives=Vec::new();let mut inner=None;
    let bytes=match containers{
        Some((container,entry))=>{
            let is_image=Path::new(container).extension().and_then(|x|x.to_str()).is_some_and(|x|x.eq_ignore_ascii_case("img")||x.eq_ignore_ascii_case("ima"));
            if is_image{
                let data=super::super::containers::read_image(Path::new(container))?;
                drives.push(Source::Image(std::sync::Arc::new(super::super::containers::fat16_tree(&data)?)));inner=Some(entry);
            }
            super::super::containers::read_entry(Path::new(container),entry)?
        },
        None=>read_file(Path::new(path))?,
    };
    if let Some(mount)=mount{drives.push(open_mount_source(mount)?);}
    // Z: holds files in memory for the session, and TEMP points there.
    drives.push(Source::Memory(MemDir::default()));
    let mut loaded=load_with_drives(&bytes,path,drives,inner)?;
    let skip=mount.map(|_|if inner.is_some(){b'D'}else{b'C'});
    crate::dos_config::apply(&mut loaded,config,skip)?;
    if let Some(mode)=fpu_mode{
        match mode{"none"=>loaded.select_fpu(false)?,"80387"=>loaded.select_fpu(true)?,_=>return Err("FPU must be none or 80387".into())}
    }
    if let Some(args)=args{set_command_tail(&mut loaded,args)?;}
    serial::attach(&mut loaded)?;
    #[cfg(pil_shadow_backend)]
    let mut session=Session::with_backend(loaded,Box::new(crate::shadow_backend::Backend::default()));
    #[cfg(not(pil_shadow_backend))]
    let mut session=Session::new(loaded);
    // Opt-in native graphical scanout. Do not redirect stdout, which is
    // the JSON session protocol used by the Python terminal frontend.
    // Keep the normal Python/Tk path unchanged unless explicitly requested.
    #[cfg(windows)]
    let mut desktop=std::env::var("DINE_VIDEO").ok()
        .filter(|mode|matches!(mode.to_ascii_lowercase().as_str(),"window"|"desktop"))
        .map(|_|crate::dine_display::Display::new());
    let stdin=std::io::stdin();let mut stdout=std::io::stdout();
    let(first,_)=session.command(0,"snapshot")?;writeln!(stdout,"{first}").map_err(|e|e.to_string())?;stdout.flush().map_err(|e|e.to_string())?;
    for(seq,line)in stdin.lock().lines().enumerate(){
        let line=line.map_err(|e|e.to_string())?;
        let(reply,quit)=if line.len()>MAX_LINE{(session.reply(seq as u64+1,false,"command line too long",&[])?,false)}else{session.command(seq as u64+1,&line)?};
        #[cfg(windows)]
        if let Some(display)=desktop.as_mut(){
            if let Some(p)=session.loaded.as_ref(){
                display.update(p,session.status().0=="waiting_input");
            }
        }
        if session.status().0=="stopped"{if let Some(p)=session.loaded.as_ref(){logger.record(p,"stopped",&session.status().1,session.steps,&session.exceptions,session.exception_count);}}
        writeln!(stdout,"{reply}").map_err(|e|e.to_string())?;stdout.flush().map_err(|e|e.to_string())?;
        if quit{break;}
    }
    Ok(())
}

