//! Interactive DOS session over stdin/stdout for a frontend: one command per
//! line, one JSON reply per line. The frontend only draws the B800 text page
//! and sends keys; all execution stays in the PIL interpreter and Dine.
use super::*;
use std::io::{BufRead,Write as _};
const MAX_LINE:usize=256;
pub struct Session{loaded:Option<Loaded>,stop:Stop,steps:usize}
impl Session{
    pub fn new(loaded:Loaded)->Self{Self{loaded:Some(loaded),stop:Stop::Budget,steps:0}}
    fn live(&self)->bool{matches!(self.stop,Stop::Budget|Stop::WaitingForInput)}
    /// Runs up to `budget` steps; output already reaches the screen through Dine.
    pub fn run(&mut self,budget:usize)->Result<Vec<u8>,String>{
        if !self.live(){return Ok(Vec::new());}
        let loaded=self.loaded.take().ok_or("session state lost")?;
        let probe=run(loaded,budget)?;
        self.steps+=probe.completed;self.stop=probe.stop;self.loaded=Some(probe.loaded);
        Ok(probe.output)
    }
    #[cfg(test)]
    pub fn loaded(&self)->Option<&Loaded>{self.loaded.as_ref()}
    /// Queues one key as the BIOS stores it: scan code in the high byte, ASCII low.
    pub fn key(&mut self,ascii:u8,scan:u8)->Result<bool,String>{
        let p=self.loaded.as_mut().ok_or("session state lost")?;
        console::enqueue(p,u16::from(scan)<<8|u16::from(ascii))
    }
    /// The active text page as character/attribute pairs, its width and the cursor.
    pub fn screen(&self)->Result<(Vec<u8>,u32,(u32,u32)),String>{
        let p=self.loaded.as_ref().ok_or("session state lost")?;
        let byte=|at|p.ram.load(at,MemWidth::Byte).map_err(|e|format!("screen read: {e:?}"));
        let word=|at|p.ram.load(at,MemWidth::Word).map_err(|e|format!("screen read: {e:?}"));
        let(columns,page)=(word(0x44a)?.clamp(1,80),byte(0x462)?&7);
        let base=0xb8000+page*word(0x44c)?;
        let cells=(0..columns*25*2).map(|i|byte(base+i).map(|b|b as u8)).collect::<Result<Vec<_>,_>>()?;
        let cursor=word(0x450+page*2)?;
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
    fn reply(&self,seq:u64,ok:bool,text:&str,output:&[u8])->Result<String,String>{
        let(cells,columns,(row,column))=self.screen()?;let(status,detail)=self.status();
        let hex:String=cells.iter().map(|b|format!("{b:02x}")).collect();
        let cs=self.loaded.as_ref().map(|p|p.selectors[SegmentRegister::Cs.index()]).unwrap_or(0);
        let eip=self.loaded.as_ref().map(|p|p.cpu.eip).unwrap_or(0);
        // The speaker: times it was switched on so far, and its tone now (0 off).
        let(beeps,tone)=self.loaded.as_ref().map_or((0,0),|p|(p.speaker.beeps,p.speaker.tone()));
        Ok(format!("{{\"protocol\":1,\"seq\":{seq},\"ok\":{ok},\"text\":{},\"status\":\"{status}\",\"detail\":{},\"steps\":{},\"cs\":{cs},\"eip\":{eip},\"columns\":{columns},\"cursor\":[{row},{column}],\"screen\":\"{hex}\",\"beeps\":{beeps},\"tone\":{tone},\"output\":{}}}",
            quote(text),quote(&detail),self.steps,quote(&String::from_utf8_lossy(output))))
    }
    /// One protocol command: `run N`, `key ASCII SCAN`, `clock OFFSET`, `snapshot` or `quit`.
    pub fn command(&mut self,seq:u64,line:&str)->Result<(String,bool),String>{
        let words:Vec<&str>=line.split_whitespace().collect();
        let mut output=Vec::new();
        let(ok,text,quit)=match words.as_slice(){
            []|["snapshot"]=>(true,String::new(),false),
            ["run",n]=>match n.parse::<usize>(){
                Ok(n) if (1..=MAX_STEPS).contains(&n)=>{output=self.run(n)?;(true,String::new(),false)},
                _=>(false,"run budget must be 1..1000000".into(),false),
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
            ["quit"]=>(true,String::new(),true),
            _=>(false,format!("unknown command: {line}"),false),
        };
        Ok((self.reply(seq,ok,&text,&output)?,quit))
    }
}
fn quote(text:&str)->String{
    let mut out=String::from("\"");
    for c in text.chars(){match c{'"'=>out.push_str("\\\""),'\\'=>out.push_str("\\\\"),'\n'=>out.push_str("\\n"),'\r'=>out.push_str("\\r"),'\t'=>out.push_str("\\t"),c if (c as u32)<0x20=>out.push_str(&format!("\\u{:04x}",c as u32)),c=>out.push(c)}}
    out.push('"');out
}
/// `--dos-session [--mount DIR] [--args TEXT] PATH`: PATH is a file or
/// IMAGE::INNER inside a container. A FAT16 image becomes read-only drive C:
/// and DIR the next drive; TEXT is the program's command line.
pub fn serve(path:&str,mount:Option<&str>,args:Option<&str>)->Result<(),String>{
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
    if let Some(mount)=mount{drives.push(Source::Host(std::path::PathBuf::from(mount)));}
    // Z: holds files in memory for the session, and TEMP points there.
    drives.push(Source::Memory(MemDir::default()));
    let mut loaded=load_with_drives(&bytes,path,drives,inner)?;
    if let Some(args)=args{set_command_tail(&mut loaded,args)?;}
    let mut session=Session::new(loaded);
    let stdin=std::io::stdin();let mut stdout=std::io::stdout();
    let(first,_)=session.command(0,"snapshot")?;writeln!(stdout,"{first}").map_err(|e|e.to_string())?;stdout.flush().map_err(|e|e.to_string())?;
    for(seq,line)in stdin.lock().lines().enumerate(){
        let line=line.map_err(|e|e.to_string())?;
        let(reply,quit)=if line.len()>MAX_LINE{(session.reply(seq as u64+1,false,"command line too long",&[])?,false)}else{session.command(seq as u64+1,&line)?};
        writeln!(stdout,"{reply}").map_err(|e|e.to_string())?;stdout.flush().map_err(|e|e.to_string())?;
        if quit{break;}
    }
    Ok(())
}
