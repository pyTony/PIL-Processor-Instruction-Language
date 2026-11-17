//! `pil --dos-console`: a DOS session in this process with its own keyboard
//! and screen, for every program alike.
//!
//! Two front ends share one loop. `--tty` draws the text page in the host
//! terminal with ANSI sequences and reads keys in raw mode; on Windows a
//! graphics mode opens the console window too. `--window` (Windows) is a
//! Win32 window that draws the text page at VGA resolution (dos_vga_text), the graphics
//! modes through `desktop_vga_frame`, and takes keyboard and mouse input.
//!
//! Host keys become the words the IBM BIOS stores (scan code high, ASCII
//! low) and go through the BIOS keyboard ring, or IRQ1 when the program owns
//! INT 09h, exactly as `--dos-session` keys do. Line editing and history are
//! therefore the program's own: a shell's arrows, command recall and
//! Ctrl+C work as they would on a PC. Nothing here knows which program runs.
use super::session::Session;
pub(crate) use crate::dos_config::HostPointer;
use super::*;
use std::sync::mpsc;
use std::time::{Duration,Instant};

/// A host key, before the BIOS mapping.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum Key{Char(char),Enter,Backspace,Tab,Escape,Up,Down,Left,Right,Home,End,PageUp,PageDown,Insert,Delete,F(u8)}
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
pub(crate) struct Mods{pub(crate) shift:bool,pub(crate) ctrl:bool,pub(crate) alt:bool}

/// What a front end hands the loop.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
#[cfg_attr(not(windows),allow(dead_code))]
pub(crate) enum Event{
    Key(Key,Mods),
    /// Physical set-1 identity plus optional layout-translated BIOS key.
    NativeKey{scan:u8,extended:bool,down:bool,key:Option<(Key,Mods)>},
    /// Ctrl+Break.
    Break,
    /// The pointer at a fraction of the drawn screen: x/width, y/height;
    /// `shift` when Shift is held (a drag then selects host text).
    Mouse{x:i32,y:i32,width:i32,height:i32,buttons:u16,shift:bool},
    /// Ctrl+Shift+C: copy the selection, or the whole screen.
    Copy,
    /// Ctrl+Shift+V or Shift+Insert: type the host clipboard.
    Paste,
    HostMenu,
    HostCommand(u16),
    Quit,
}

#[cfg(any(windows,test))]
#[path="dos_host_keys.rs"]
mod host_keys;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum PendingKey{Tap(u16),Native{scan:u8,extended:bool,down:bool,word:Option<u16>}}
impl From<u16> for PendingKey{fn from(word:u16)->Self{Self::Tap(word)}}
impl PendingKey{
    fn send(self,session:&mut Session)->Result<bool,String>{
        match self{
            Self::Tap(word)=>session.key(word as u8,(word>>8)as u8),
            Self::Native{scan,extended,down,word}=>session.native_key(scan,extended,down,word),
        }
    }
}

// US-layout set 1 scan codes of the keys that type ASCII.
const ROWS:[(&str,u8);4]=[("1234567890-=",0x02),("qwertyuiop[]",0x10),("asdfghjkl;'`",0x1e),("zxcvbnm,./",0x2c)];
const SHIFTED:&str="!@#$%^&*()_+{}:\"~<>?|";
const UNSHIFTED:&str="1234567890-=[];'`,./\\";
fn scan_of(c:char)->Option<u8>{
    let c=c.to_ascii_lowercase();
    let c=SHIFTED.find(c).and_then(|i|UNSHIFTED[i..].chars().next()).unwrap_or(c);
    match c{'\\'=>return Some(0x2b),' '=>return Some(0x39),_=>{}}
    ROWS.iter().find_map(|(row,start)|row.find(c).map(|i|start+i as u8))
}
/// CP437 byte of a host character, for 80h-FFh and the symbols at 14h/15h.
fn cp437(c:char)->Option<u8>{
    if c.is_ascii(){return Some(c as u8);}
    let bytes=crate::dos_encoding::command_line(&c.to_string()).ok()?;
    if let [b]=bytes[..]{return Some(b);}
    match c{'¶'=>Some(0x14),'§'=>Some(0x15),_=>None}
}

/// The modifiers of a typed character. On Windows AltGr arrives as Ctrl+Alt
/// together with the character it typed (@, \, |, {, [, ], } and $ on a
/// Finnish keyboard): that character is typed plainly, as a DOS keyboard
/// driver would give it, not as a Ctrl or Alt combination. (The euro sign
/// has no code page 437 character and types nothing.)
pub(crate) fn typed_mods(c:char,m:Mods)->Mods{
    if m.ctrl&&m.alt&&!c.is_control(){Mods{ctrl:false,alt:false,..m}}else{m}
}
/// The BIOS keyboard word for a host key, as INT 16h AH=10h returns it
/// (IBM PC/AT BIOS tables), or None for keys DOS never sees.
pub(crate) fn bios_word(key:Key,m:Mods)->Option<u16>{
    let w=|scan:u8,ascii:u8|Some(u16::from(scan)<<8|u16::from(ascii));
    // (plain, Ctrl, Alt) scan codes of the keys that type nothing.
    let block=|plain:u8,ctrl:u8,alt:u8|w(if m.alt{alt}else if m.ctrl{ctrl}else{plain},0);
    match key{
        Key::Up=>block(0x48,0x8d,0x98),Key::Down=>block(0x50,0x91,0xa0),
        Key::Left=>block(0x4b,0x73,0x9b),Key::Right=>block(0x4d,0x74,0x9d),
        Key::Home=>block(0x47,0x77,0x97),Key::End=>block(0x4f,0x75,0x9f),
        Key::PageUp=>block(0x49,0x84,0x99),Key::PageDown=>block(0x51,0x76,0xa1),
        Key::Insert=>block(0x52,0x92,0xa2),Key::Delete=>block(0x53,0x93,0xa3),
        Key::Enter=>if m.alt{w(0x1c,0)}else if m.ctrl{w(0x1c,0x0a)}else{w(0x1c,0x0d)},
        Key::Backspace=>if m.alt{w(0x0e,0)}else if m.ctrl{w(0x0e,0x7f)}else{w(0x0e,0x08)},
        Key::Tab=>if m.alt{w(0xa5,0)}else if m.ctrl{w(0x94,0)}else if m.shift{w(0x0f,0)}else{w(0x0f,0x09)},
        Key::Escape=>if m.alt{w(0x01,0)}else{w(0x01,0x1b)},
        Key::F(n @ 1..=10)=>{
            let n=n-1;
            w(if m.alt{0x68+n}else if m.ctrl{0x5e+n}else if m.shift{0x54+n}else{0x3b+n},0)
        },
        Key::F(n @ 11..=12)=>{
            let n=n-11;
            w(if m.alt{0x8b+n}else if m.ctrl{0x89+n}else if m.shift{0x87+n}else{0x85+n},0)
        },
        Key::F(_)=>None,
        Key::Char(c)=>{
            if m.alt{
                // Alt with a letter or a top-row key: ASCII 0, the key's scan
                // code (78h-83h for 1-0, - and =).
                let lower=c.to_ascii_lowercase();
                if lower.is_ascii_lowercase(){return w(scan_of(lower)?,0);}
                if let Some(i)="1234567890-=".find(c){return w(0x78+i as u8,0);}
                return None;
            }
            if m.ctrl{
                if c.is_ascii_alphabetic(){let l=c.to_ascii_lowercase();return w(scan_of(l)?,l as u8-0x60);}
                return match c{'['=>w(0x1a,0x1b),'\\'=>w(0x2b,0x1c),']'=>w(0x1b,0x1d),'6'|'^'=>w(0x07,0x1e),'-'|'_'=>w(0x0c,0x1f),'2'|'@'|' '=>w(0x03,0),_=>None};
            }
            // A control code typed as such (a terminal sends Ctrl+letter so).
            if (1..=26).contains(&(c as u32)){
                let letter=char::from(c as u8+0x60);return w(scan_of(letter)?,c as u8);
            }
            let byte=cp437(c)?;
            if byte<0x20&&!matches!(byte,0x14|0x15){return None;}
            // Letters beyond ASCII take the scan code of the key they sit on
            // in the Nordic layouts; other characters get 0.
            let scan=scan_of(c).unwrap_or(match c.to_lowercase().next().unwrap_or(c){'å'=>0x1a,'ä'|'æ'=>0x28,'ö'|'ø'=>0x27,'§'|'½'=>0x29,_=>0});
            w(scan,byte)
        },
    }
}

/// Terminal input in raw mode: bytes to keys. ESC alone is the Esc key only
/// after a short pause (`flush`); ESC [ and ESC O start VT sequences, ESC
/// and another key is Alt with that key. Ctrl+] (1Dh) ends the console.
#[derive(Default)]
#[cfg(test)]
#[cfg_attr(windows,allow(dead_code))]
pub(crate) struct VtParser{pending:Vec<u8>}
#[cfg(test)]
#[cfg_attr(windows,allow(dead_code))]
impl VtParser{
    pub(crate) fn feed(&mut self,byte:u8,out:&mut Vec<Event>){
        self.pending.push(byte);
        loop{
            match self.parse(){
                Parsed::Done(event,used)=>{
                    self.pending.drain(..used);
                    if let Some(event)=event{out.push(event);}
                    if self.pending.is_empty(){return;}
                },
                Parsed::More=>return,
            }
        }
    }
    /// The pause after a lone ESC (or an unfinished sequence) has passed.
    pub(crate) fn flush(&mut self,out:&mut Vec<Event>){
        if self.pending.is_empty(){return;}
        let rest=std::mem::take(&mut self.pending);
        // An unfinished ESC sequence ends as Esc and the bytes after it.
        out.push(Event::Key(Key::Escape,Mods::default()));
        for b in rest.into_iter().skip(1){self.feed(b,out);}
        if self.pending==[0x1b]{self.pending.clear();out.push(Event::Key(Key::Escape,Mods::default()));}
    }
    pub(crate) fn waiting(&self)->bool{!self.pending.is_empty()}
    fn parse(&self)->Parsed{
        let p=&self.pending;
        let plain=Mods::default();
        let key=|k:Key,m:Mods,n:usize|Parsed::Done(Some(Event::Key(k,m)),n);
        match p[0]{
            0x1b=>{
                if p.len()<2{return Parsed::More;}
                match p[1]{
                    b'['|b'O'=>self.sequence(),
                    0x1b=>Parsed::Done(Some(Event::Key(Key::Escape,plain)),1),
                    _=>{
                        // Alt with the key after ESC.
                        let inner=VtParser{pending:p[1..].to_vec()};
                        match inner.parse(){
                            Parsed::More=>Parsed::More,
                            Parsed::Done(Some(Event::Key(k,mut m)),n)=>{m.alt=true;key(k,m,n+1)},
                            Parsed::Done(e,n)=>Parsed::Done(e,n+1),
                        }
                    },
                }
            },
            0x0d|0x0a=>key(Key::Enter,plain,1),
            0x7f|0x08=>key(Key::Backspace,plain,1),
            0x09=>key(Key::Tab,plain,1),
            0x1d=>Parsed::Done(Some(Event::Quit),1),
            0x00=>key(Key::Char(' '),Mods{ctrl:true,..plain},1),
            b @ 1..=26=>key(Key::Char(char::from(b+0x60)),Mods{ctrl:true,..plain},1),
            b @ 0x1c..=0x1f=>key(Key::Char(char::from(b+0x40)),Mods{ctrl:true,..plain},1),
            b if b<0x80=>key(Key::Char(char::from(b)),plain,1),
            b=>{
                let len=if b>=0xf0{4}else if b>=0xe0{3}else if b>=0xc0{2}else{return Parsed::Done(None,1)};
                if p.len()<len{return Parsed::More;}
                match std::str::from_utf8(&p[..len]).ok().and_then(|s|s.chars().next()){
                    Some(c)=>key(Key::Char(c),plain,len),
                    None=>Parsed::Done(None,1),
                }
            },
        }
    }
    fn sequence(&self)->Parsed{
        let p=&self.pending;
        // ESC [ params final, or ESC O final.
        let Some(end)=p[2..].iter().position(|b|(0x40..=0x7e).contains(b)).map(|i|i+2) else {
            return if p.len()>16{Parsed::Done(None,p.len())}else{Parsed::More};
        };
        let params:Vec<u32>=std::str::from_utf8(&p[2..end]).unwrap_or("").split(';').map(|x|x.parse().unwrap_or(0)).collect();
        // xterm modifier parameter: 1 + Shift(1) + Alt(2) + Ctrl(4).
        let bits=params.get(1).copied().unwrap_or(1).saturating_sub(1);
        let m=Mods{shift:bits&1!=0,alt:bits&2!=0,ctrl:bits&4!=0};
        let k=match p[end]{
            b'A'=>Some(Key::Up),b'B'=>Some(Key::Down),b'C'=>Some(Key::Right),b'D'=>Some(Key::Left),
            b'H'=>Some(Key::Home),b'F'=>Some(Key::End),b'Z'=>return Parsed::Done(Some(Event::Key(Key::Tab,Mods{shift:true,..Mods::default()})),end+1),
            b'P'=>Some(Key::F(1)),b'Q'=>Some(Key::F(2)),b'R'=>Some(Key::F(3)),b'S'=>Some(Key::F(4)),
            b'~'=>match params.first().copied().unwrap_or(0){
                1|7=>Some(Key::Home),2=>Some(Key::Insert),3=>Some(Key::Delete),4|8=>Some(Key::End),
                5=>Some(Key::PageUp),6=>Some(Key::PageDown),
                n @ 11..=15=>Some(Key::F((n-10) as u8)),n @ 17..=21=>Some(Key::F((n-11) as u8)),
                23=>Some(Key::F(11)),24=>Some(Key::F(12)),_=>None,
            },
            _=>None,
        };
        Parsed::Done(k.map(|k|Event::Key(k,m)),end+1)
    }
}
#[cfg_attr(windows,allow(dead_code))]
#[cfg(test)]
enum Parsed{Done(Option<Event>,usize),More}

/// IBM PC symbols for the glyphs of 00h-1Fh and 7Fh.
// Escaped so compiler diagnostics stay ASCII (Windows CI reads them as cp1252).
const LOW:[char;32]=[' ','\u{263a}','\u{263b}','\u{2665}','\u{2666}','\u{2663}','\u{2660}','\u{2022}','\u{25d8}','\u{25cb}','\u{25d9}','\u{2642}','\u{2640}','\u{266a}','\u{266b}','\u{263c}','\u{25ba}','\u{25c4}','\u{2195}','\u{203c}','\u{b6}','\u{a7}','\u{25ac}','\u{21a8}','\u{2191}','\u{2193}','\u{2192}','\u{2190}','\u{221f}','\u{2194}','\u{25b2}','\u{25bc}'];
fn glyph(b:u8)->char{
    match b{0..=31=>LOW[usize::from(b)],0x7f=>'\u{2302}',_=>crate::dos_encoding::decode(&[b]).chars().next().unwrap_or(' ')}
}
/// CGA colour index to the ANSI colour number.
const ANSI:[u8;8]=[0,4,2,6,1,5,3,7];

/// ANSI text for the rows of `cells` that differ from `previous`, ending
/// with the cursor at its place.
pub(crate) fn ansi_frame(cells:&[u8],columns:u32,cursor:(u32,u32),cursor_visible:bool,previous:Option<&[u8]>)->String{
    let columns=columns as usize;let row_bytes=columns*2;
    let mut out=String::new();
    for row in 0..cells.len()/row_bytes{
        let line=&cells[row*row_bytes..(row+1)*row_bytes];
        if previous.and_then(|p|p.get(row*row_bytes..(row+1)*row_bytes))==Some(line){continue;}
        out.push_str(&format!("\x1b[{};1H",row+1));
        let mut attr=None;
        for cell in line.chunks(2){
            if attr!=Some(cell[1]){
                let a=cell[1];
                let fg=if a&8!=0{90}else{30}+ANSI[usize::from(a&7)];
                let bg=40+ANSI[usize::from((a>>4)&7)];
                out.push_str(&format!("\x1b[0;{fg};{bg}m"));
                attr=Some(a);
            }
            out.push(glyph(cell[0]));
        }
    }
    out.push_str(&format!("\x1b[0m\x1b[{};{}H\x1b[?25{}",cursor.0+1,cursor.1+1,if cursor_visible{'h'}else{'l'}));
    out
}

#[cfg(any(windows,test))]
pub(crate) use super::vga_text::text_pixels;
/// The BIOS cursor (0040:0050 for the page, shape at 0040:0060) on a 16-line
/// cell, or None when it is off.
fn cursor_shape(p:&Loaded)->Option<(u32,u32)>{
    let byte=|at|p.ram.load(at,MemWidth::Byte).ok();
    let (end,start)=(byte(0x460)?,byte(0x461)?);
    super::vga_text::cursor_lines(start as u8,end as u8)
}
pub(crate) fn graphics_mode(p:&Loaded)->bool{
    p.ram.load(0x449,MemWidth::Byte).is_ok_and(|m|!matches!(m&0x7f,0..=3|7))
}

/// Options of `pil --dos-console`.
pub(crate) struct Options{pub(crate) window:bool,pub(crate) mouse:bool,pub(crate) ansi:bool,pub(crate) country:Option<u16>,pub(crate) utc_offset:Option<i64>,
    /// The host pointer over the window (Windows); the terminal ignores it.
    pub(crate) host_pointer:HostPointer,
    /// PC speaker volume 0..100, 0 silent (`speaker_volume`).
    pub(crate) speaker_volume:u8}

/// Run the session until the program ends, it stops, or the user quits.
/// Returns the closing text for the host terminal: when the program ended
/// or stopped, the last lines it left on the screen (the window or the
/// terminal's alternate screen is gone by then), then the closing line.
pub(crate) fn run(session:&mut Session,options:&Options)->Result<String,String>{
    let (tx,rx)=mpsc::channel();
    let mut front=Front::open(options.window,tx)?;
    let result=run_on(session,options,&mut front,&rx);
    drop(front);
    let line=result?;
    if matches!(session.status().0,"running"|"waiting_input"){return Ok(line);}
    let tail=select::screen_text(session).map(|(columns,_,text)|last_lines(&text,columns,CLOSING_LINES)).unwrap_or_default();
    Ok(if tail.is_empty(){line}else{format!("{tail}\n{line}")})
}
/// Screen lines repeated on the host terminal when a program ends.
const CLOSING_LINES:usize=4;
/// The last `count` lines of a text grid with text on them, as host
/// text (trailing blanks and blank lines at the end left out).
pub(crate) fn last_lines(text:&[u8],columns:u32,count:usize)->String{
    let all=select::copy_text(text,columns,None);
    let lines:Vec<&str>=all.split("\r\n").collect();
    let first=lines.len().saturating_sub(count);
    lines[first..].iter().skip_while(|l|l.is_empty()).copied().collect::<Vec<_>>().join("\n")
}
/// Apply the same console settings to interactive and redirected sessions.
pub(crate) fn configure(session:&mut Session,options:&Options)->Result<(),String>{
    let offset=options.utc_offset.unwrap_or_else(local_utc_offset);
    for command in [Some(format!("clock {offset}")),options.country.map(|c|format!("country {c}")),
        options.ansi.then(||"ansi on".to_string()),options.mouse.then(||"mouse on".to_string())].into_iter().flatten(){
        let (reply,_)=session.command(0,&command)?;
        if reply.contains("\"ok\":false"){return Err(format!("{command}: {reply}"));}
    }
    Ok(())
}
/// Run on the setup screen's already-open frontend.
pub(crate) fn run_on(session:&mut Session,options:&Options,front:&mut Front,rx:&mpsc::Receiver<Event>)->Result<String,String>{
    configure(session,options)?;
    front.reset();
    front.host_pointer=options.host_pointer;
    // Changes from before this run (a setup screen, an earlier program) are not sounded.
    if let Some(p)=session.machine_mut(){p.speaker.take_sound();p.sb.take_pcm();}
    let mut sound=speaker::Sound::host(options.speaker_volume);
    let mut sb_sound=sb_audio::Sound::host(options.speaker_volume);
    let result=drive(session,front,rx,&mut sound,&mut sb_sound);
    front.reset();
    result
}

fn drive(session:&mut Session,front:&mut Front,rx:&mpsc::Receiver<Event>,sound:&mut speaker::Sound,sb_sound:&mut sb_audio::Sound)->Result<String,String>{
    const SLICE:usize=20_000;
    let mut pending=std::collections::VecDeque::<PendingKey>::new();
    let mut last_present=Instant::now()-Duration::from_secs(1);
    let mut note=None::<String>;
    let mut host_menu_open=false;
    let mut right_down=false;
    loop{
        // Input first: keys wait while the BIOS ring is full.
        let mut events:Vec<Event>=rx.try_iter().collect();
        front.poll(&mut events)?;
        for event in events{
            match event{
                Event::Quit=>return Ok("DOS console closed by the user".into()),
                Event::Break=>{if let Err(refused)=session.ctrl_break()?{note=Some(refused);}},
                Event::Key(Key::F(n @ 1..=4),mods) if mods.ctrl&&mods.shift&&!mods.alt=>{
                    let letter=if n%2==1{b'A'}else{b'B'};
                    note=Some(session.disk(letter,Some(if n<=2{"next"}else{"prev"})).unwrap_or_else(|e|e));
                },
                Event::Key(key,mods)=>{front.clear_selection();if let Some(word)=bios_word(key,mods){pending.push_back(word.into());}},
                Event::NativeKey{scan,extended,down,key}=>{
                    front.clear_selection();
                    pending.push_back(PendingKey::Native{scan,extended,down,word:key.and_then(|(k,m)|bios_word(k,m))});
                },
                Event::Copy=>front.copy(session)?,
                Event::Paste=>pending.extend(front.paste().into_iter().map(PendingKey::from)),
                Event::HostMenu=>{host_menu_open=front.media_menu();},
                Event::HostCommand(id)=>{
                    host_menu_open=false;
                    if id==1{front.copy(session)?;}
                    else if id==2{pending.extend(front.paste().into_iter().map(PendingKey::from));}
                    else if id==3{note=Some(save_live_snapshot(session,pending.len()).unwrap_or_else(|e|e));}
                    else if id==4{note=Some(session.disable_live_trace().map(|_|"Dispatch history disabled".to_string()).unwrap_or_else(|e|e));}
                    else if id!=0{note=front.media_command(session,id);}
                },
                Event::Mouse{x,y,width,height,buttons,shift}=>{
                    let pressed=buttons&2!=0&&!right_down;right_down=buttons&2!=0;
                    let active=session.machine().is_some_and(|p|p.mouse.view().active);
                    if pressed&&(shift||!active){host_menu_open=front.media_menu();continue;}
                    if front.selects(active,shift){
                        match front.select(session,x,y,width,height,buttons)?{
                            select::Gesture::Copy=>front.copy(session)?,
                            select::Gesture::Paste=>pending.extend(front.paste().into_iter().map(PendingKey::from)),
                            select::Gesture::None=>{},
                        }
                    }else if width>0&&height>0{
                        let view=session.machine().map(|p|p.mouse.view()).ok_or("session state lost")?;
                        let fx=(x.clamp(0,width-1) as i64*(i64::from(view.max_x)+1)/i64::from(width)) as u16;
                        let fy=(y.clamp(0,height-1) as i64*(i64::from(view.max_y)+1)/i64::from(height)) as u16;
                        session.mouse(fx,fy,buttons)?;
                    }
                },
            }
        }
        if host_menu_open{std::thread::sleep(Duration::from_millis(10));continue;}
        while let Some(&key)=pending.front(){
            if key.send(session)?{pending.pop_front();}else{break;}
        }
        // Run for about one frame.
        let until=Instant::now()+Duration::from_millis(15);
        loop{
            session.run(SLICE)?;
            // Both PC speaker and Sound Blaster PCM follow guest execution.
            if let Some(p)=session.machine_mut(){sound.take(&mut p.speaker);sb_sound.take(&mut p.sb);}
            if session.status().0!="running"||Instant::now()>=until{break;}
        }
        if let Some(p)=session.machine_mut(){if let Some(com1)=p.com1.as_mut(){com1.tx.clear();}}
        let (status,detail)=session.status();
        let waiting=status=="waiting_input";
        if waiting||last_present.elapsed()>=Duration::from_millis(16){
            front.present(session,note.take())?;
            last_present=Instant::now();
        }
        match status{
            "running"=>{},
            "waiting_input"=>{
                // Nothing to do until a key arrives (or the timer advances).
                if pending.is_empty(){
                    if let Ok(event)=rx.recv_timeout(Duration::from_millis(10)){front.unpoll(event);}
                }
            },
            "exit"=>{front.present(session,None)?;return Ok(format!("DOS program ended: {detail}"));},
            _=>{front.present(session,None)?;return Ok(format!("DOS session stopped after {} steps: {detail}",session.steps()));},
        }
        if front.closed(){return Ok("DOS console window closed".into());}
    }
}

/// The host side: a terminal, a window, or both (Windows terminal with
/// graphics in a window).
pub(crate) struct Front{
    tty:Option<tty::Terminal>,
    #[cfg(windows)]
    window:Option<WindowFront>,
    #[cfg(windows)]
    window_tx:Option<mpsc::Sender<Event>>,
    held:Vec<Event>,
    previous:Option<Vec<u8>>,
    /// The cursor `present_text` last drew, to skip unchanged frames.
    previous_cursor:Option<(u32,u32)>,
    graphics_note:bool,
    /// Last ANSI graphics picture, to avoid repainting unchanged frames.
    graphics_cache:Option<String>,
    /// Host text selection in the window, and the (columns, rows) grid it
    /// was made on.
    selecting:select::Selecting,
    grid:(u32,u32),
    /// Whether the window hides the host pointer; the setup screen and
    /// `reset` go back to Show (the screen has no DOS cursor).
    host_pointer:HostPointer,
}
/// Append one consistent guest-state sample; never stop or step the guest here.
/// The host input event loop invokes this between its bounded run slices.
fn save_live_snapshot(session:&mut Session,host_pending:usize)->Result<String,String>{
    use std::io::Write as _;
    let (json,started)=session.live_snapshot(host_pending)?;
    let path="PILdos-snapshots.jsonl";
    let mut file=std::fs::OpenOptions::new().append(true).create(true).open(path)
        .map_err(|e|format!("cannot open {path}: {e}"))?;
    writeln!(file,"{json}").map_err(|e|format!("cannot write {path}: {e}"))?;
    Ok(if started{
        format!("Saved {path}; 32-entry trace enabled (capture again)")
    }else{format!("Saved {path}")})
}
/// A Windows Explorer path, possibly quoted by Copy as path.
#[cfg(any(windows,test))]
fn clipboard_mount_path(text:&str)->Result<&str,String>{
    let text=text.trim();
    let path=if text.starts_with('"')||text.ends_with('"'){
        text.strip_prefix('"').and_then(|s|s.strip_suffix('"'))
            .ok_or("clipboard path has unmatched quotation marks")?
    }else{text};
    let path=path.trim();
    if path.is_empty()||path.chars().any(char::is_control){
        return Err("clipboard must contain one existing host folder or .img/.ima path".into());
    }
    Ok(path)
}

impl Front{
    pub(crate) fn title(&self,title:&str){
        #[cfg(windows)]
        if let Some(window)=self.window.as_ref(){window.window.title(title);}
        let _=title;
    }
    fn media_menu(&self)->bool{
        #[cfg(windows)]
        if let Some(w)=self.window.as_ref(){
            let mut groups=vec![("Clipboard".into(),vec![(1,"Copy".into()),(2,"Paste".into())])];
            for (index,drive) in ['A','B','D','E'].iter().enumerate(){
                let base=100+index as u16*10;
                let mut items=vec![(base+1,"Mount image...".into()),(base+2,"Mount folder...".into())];
                if index<2{items.extend([(base+3,"Open disk set...".into()),(base+4,"Next disk".into()),(base+5,"Previous disk".into()),(base+6,"Current disk".into())]);}
                items.push((base+8,"Mount clipboard path".into()));
                items.push((base+7,"Unmount".into()));groups.push((format!("Drive {drive}:"),items));
            }
            groups.push(("Diagnostics".into(),vec![
                (3,"Save live guest snapshot".into()),
                (4,"Disable instruction history".into()),
            ]));
            w.window.popup(groups);return true;
        }
        false
    }
    fn media_command(&mut self,session:&mut Session,id:u16)->Option<String>{
        #[cfg(windows)]
        {
            let drive=*b"ABDE".get(usize::from(id.saturating_sub(100)/10))?;
            let action=id%10;
            let result=match action{
                1..=3=>{
                    let chosen=if action==2{media_dialog::folder()}else{media_dialog::file(action==3)};
                    match chosen{
                        Ok(Some(path))=>{
                            if action==3{session.disk_set(drive,&path)}else if action==1&&matches!(drive,b'A'|b'B'){
                                session.floppy_media(drive,&path)
                            }else{
                                media_launch::mount_path(session,drive,&path)
                            }
                        },
                        Ok(None)=>return None,Err(e)=>Err(e),
                    }
                },
                4=>session.disk(drive,Some("next")),5=>session.disk(drive,Some("prev")),6=>session.disk(drive,None),
                7=>session.unmount_media(drive),
                8=>crate::dine_window::clipboard().ok_or_else(||"clipboard has no text path; use Explorer's Copy as path".to_string())
                    .and_then(|text|{
                        let path=clipboard_mount_path(&text)?;
                        let is_image=std::path::Path::new(path).extension().and_then(|s|s.to_str())
                            .is_some_and(|s|s.eq_ignore_ascii_case("img")||s.eq_ignore_ascii_case("ima"));
                        if matches!(drive,b'A'|b'B')&&is_image{
                            session.floppy_media(drive,std::path::Path::new(path))
                        }else{media_launch::mount_path(session,drive,std::path::Path::new(path))}
                    }),
                _=>return None,
            };
            Some(result.unwrap_or_else(|e|e))
        }
        #[cfg(not(windows))]
        {let _=(session,id);None}
    }
    pub(crate) fn open(window:bool,tx:mpsc::Sender<Event>)->Result<Self,String>{
        #[cfg(not(windows))]
        if window{return Err("the DOS console window needs Windows; use --tty".into());}
        let tty=if window{None}else{Some(tty::Terminal::open(tx.clone())?)};
        #[cfg(windows)]
        let (window,window_tx)=if window{(Some(WindowFront::new(tx)),None)}else{(None,Some(tx))};
        #[cfg(not(windows))]
        let _=tx;
        Ok(Self{tty,#[cfg(windows)]window,#[cfg(windows)]window_tx,held:Vec::new(),previous:None,previous_cursor:None,graphics_note:false,graphics_cache:None,selecting:select::Selecting::default(),grid:(0,0),host_pointer:HostPointer::Show})
    }
    pub(crate) fn poll(&mut self,events:&mut Vec<Event>)->Result<(),String>{
        events.splice(0..0,self.held.drain(..));
        if let Some(t)=self.tty.as_mut(){t.poll(events)?;}
        Ok(())
    }
    fn unpoll(&mut self,event:Event){self.held.push(event);}
    /// Forget what is on the host screen, so the next frame draws it all.
    pub(crate) fn reset(&mut self){
        self.previous=None;self.previous_cursor=None;self.graphics_note=false;self.graphics_cache=None;self.held.clear();self.selecting.clear();
        self.host_pointer=HostPointer::Show;
        #[cfg(windows)]
        if let Some(w)=self.window.as_mut(){w.last=None;w.window.hide_pointer(false);}
        if let Some(t)=self.tty.as_ref(){let _=t.write("\x1b[0m\x1b[2J");}
    }
    /// Show a text page that is not a DOS session's (the setup screen):
    /// 80-column character/attribute cells and the cursor, if any.
    pub(crate) fn present_text(&mut self,cells:&[u8],cursor:Option<(u32,u32)>)->Result<(),String>{
        if let Some(t)=self.tty.as_mut(){
            if self.previous.as_deref()==Some(cells)&&self.previous_cursor==cursor{return Ok(());}
            let text=ansi_frame(cells,80,cursor.unwrap_or((24,79)),cursor.is_some(),self.previous.as_deref());
            self.previous=Some(cells.to_vec());self.previous_cursor=cursor;
            return t.write(&text);
        }
        #[cfg(windows)]
        if let Some(w)=self.window.as_mut(){w.present_text(cells,cursor);}
        Ok(())
    }
    /// Whether a pointer event selects host text instead of reaching the
    /// program: only in the window, when the program has no mouse driver
    /// active, while Shift is held, or during a selection drag.
    fn selects(&self,active:bool,shift:bool)->bool{
        #[cfg(windows)]
        if self.window.is_some(){return !active||shift||self.selecting.dragging();}
        let _=(active,shift);false
    }
    /// A pointer event for the selection, on the screen's character grid.
    fn select(&mut self,session:&Session,x:i32,y:i32,width:i32,height:i32,buttons:u16)->Result<select::Gesture,String>{
        let grid=grid(session.machine().ok_or("session state lost")?)?;
        if grid!=self.grid{self.selecting.clear();self.grid=grid;}
        Ok(self.selecting.pointer(select::cell_at(x,y,width,height,grid.0,grid.1),buttons))
    }
    fn clear_selection(&mut self){self.selecting.clear();}
    /// Copy the selection, or the whole screen, to the host clipboard.
    fn copy(&mut self,session:&Session)->Result<(),String>{
        let(columns,_,text)=select::screen_text(session)?;
        let chosen=select::copy_text(&text,columns,self.selecting.selection);
        #[cfg(windows)]
        if self.window.is_some(){crate::dine_window::set_clipboard(&chosen);}
        let _=chosen;
        self.selecting.clear();
        Ok(())
    }
    /// The keys that type the host clipboard.
    fn paste(&mut self)->Vec<u16>{
        #[cfg(windows)]
        if self.window.is_some(){return crate::dine_window::clipboard().map_or_else(Vec::new,|t|select::paste_keys(&t));}
        Vec::new()
    }
    pub(crate) fn closed(&self)->bool{
        #[cfg(windows)]
        if let Some(w)=self.window.as_ref(){return w.closed();}
        false
    }
    fn present(&mut self,session:&Session,note:Option<String>)->Result<(),String>{
        let p=session.machine().ok_or("session state lost")?;
        let graphics=graphics_mode(p);
        #[cfg(windows)]
        {
            // A terminal session shows graphics in a window that also takes keys.
            if graphics&&self.window.is_none(){
                if let Some(tx)=self.window_tx.take(){self.window=Some(WindowFront::new(tx));}
            }
            let grid=grid(p)?;
            if grid!=self.grid{self.selecting.clear();self.grid=grid;}
            let selection=self.selecting.selection.map(|s|(s,grid.0,grid.1));
            if let Some(w)=self.window.as_mut(){
                if let Some(ref note)=note{w.window.title(&format!("Dine DOS console - {note}"));}
                w.window.hide_pointer(self.host_pointer.hides(p.mouse.view().visible));
                w.present(session,selection)?;
            }
        }
        if let Some(t)=self.tty.as_mut(){
            if graphics{
                #[cfg(not(windows))]
                {
                    if let Some(frame)=desktop_vga_frame(p){
                        let (cols,rows)=crossterm::terminal::size().unwrap_or((80,25));
                        let picture=terminal_graphics::half_block_frame(&frame,cols,rows);
                        if !self.graphics_note || self.graphics_cache.as_deref()!=Some(picture.as_str()){
                            if !self.graphics_note{t.write("\x1b[0m\x1b[2J")?;}
                            t.write(&picture)?;
                            self.graphics_cache=Some(picture);
                        }
                        self.graphics_note=true;
                    }else if !self.graphics_note{
                        let mode=p.ram.load(0x449,MemWidth::Byte).unwrap_or(0);
                        t.write(&format!("\x1b[0m\x1b[2J\x1b[1;1HGraphics mode {mode:02X}h: scanout unavailable; keys still reach DOS."))?;
                        self.graphics_note=true;
                    }
                }
                #[cfg(windows)]
                if !self.graphics_note{
                    self.graphics_note=true;self.previous=None;
                    let mode=p.ram.load(0x449,MemWidth::Byte).unwrap_or(0);
                    t.write(&format!("\x1b[0m\x1b[2J\x1b[1;1HGraphics mode {mode:02X}h: shown in the Dine window; keys work there and here."))?;
                }
            }else{
                if self.graphics_note{t.write("\x1b[0m\x1b[2J")?;self.previous=None;}
                self.graphics_note=false;self.graphics_cache=None;
                let (cells,columns,cursor)=session.screen()?;
                let visible=cursor_shape(p).is_some();
                let text=ansi_frame(&cells,columns,cursor,visible,self.previous.as_deref());
                self.previous=Some(cells);
                t.write(&text)?;
            }
            if let Some(note)=note{t.write(&format!("\x1b[s\x1b[25;1H\x1b[0;7m {note} \x1b[0m\x1b[u"))?;}
        }
        Ok(())
    }
}

#[cfg(windows)]
struct WindowFront{
    window:crate::dine_window::Window,
    font:Vec<u8>,
    started:Instant,
    last:Option<(usize,usize,Vec<u32>)>,
}
#[cfg(windows)]
impl WindowFront{
    fn new(tx:mpsc::Sender<Event>)->Self{
        let (input,raw)=mpsc::channel::<crate::dine_window::Input>();
        // The window thread sends raw input; map it here, off the window thread.
        std::thread::Builder::new().name("dine-console-input".into()).spawn(move||{
            let mut keys=host_keys::WindowKeys::default();
            for input in raw{
                for event in keys.input(input){if tx.send(event).is_err(){return;}}
            }
        }).ok();
        let font=super::vga_text::font().unwrap_or_else(|_|vec![0;256*super::vga_text::CELL_HEIGHT]);
        Self{window:crate::dine_window::Window::interactive("Dine DOS console",input),font,started:Instant::now(),last:None}
    }
    fn closed(&self)->bool{self.window.closed()}
    fn present_text(&mut self,cells:&[u8],cursor:Option<(u32,u32)>){
        let on=self.started.elapsed().as_millis()/267%2==0;
        let frame=text_pixels(cells,80,&self.font,cursor.filter(|_|on).map(|(r,c)|(r,c,13,14)));
        if self.last.as_ref()==Some(&frame){return;}
        self.window.present(crate::dine_window::Frame{width:frame.0 as i32,height:frame.1 as i32,pixels:frame.2.clone()});
        self.last=Some(frame);
    }
    fn present(&mut self,session:&Session,selection:Option<(select::Selection,u32,u32)>)->Result<(),String>{
        let p=session.machine().ok_or("session state lost")?;
        let frame=if graphics_mode(p){desktop_vga_frame(p)}else{
            let (cells,columns,(row,column))=session.screen()?;
            // The hardware cursor blinks at about 3.75 Hz (16 frames of 60).
            let on=self.started.elapsed().as_millis()/267%2==0;
            let cursor=cursor_shape(p).filter(|_|on).map(|(a,b)|(row,column,a,b));
            Some(text_pixels(&cells,columns,&self.font,cursor))
        };
        let Some(mut frame)=frame else{self.window.no_frame();return Ok(());};
        if let Some((s,columns,rows))=selection{select::highlight(&mut frame,columns,rows,s);}
        if self.last.as_ref()==Some(&frame){return Ok(());}
        self.window.present(crate::dine_window::Frame{width:frame.0 as i32,height:frame.1 as i32,pixels:frame.2.clone()});
        self.last=Some(frame);
        Ok(())
    }
}
/// A window input as a console event. Ctrl+Shift+C and Ctrl+Shift+V copy
/// and paste, as Shift+Insert pastes; plain Ctrl+C and Ctrl+V reach DOS.
#[cfg(any(windows,test))]
fn window_event(input:crate::dine_window::Input)->Option<Event>{
    use crate::dine_window::Input;
    match input{
        Input::Char{ch:'\u{3}',shift:true,ctrl:true,alt:false}=>Some(Event::Copy),
        Input::Char{ch:'\u{16}',shift:true,ctrl:true,alt:false}=>Some(Event::Paste),
        Input::Char{ch:'\r',shift:true,ctrl:true,alt:false}=>Some(Event::HostMenu),
        Input::Char{ch:'\u{1b}',shift,ctrl,alt}=>Some(Event::Key(Key::Escape,Mods{shift,ctrl,alt})),
        Input::Key{vk:0x2d,shift:true,ctrl:false,alt:false}=>Some(Event::Paste),
        // Windows context-menu key (VK_APPS), and its standard Shift+F10
        // equivalent, open the host drive controls instead of reaching DOS.
        Input::Key{vk:0x5d,..}=>Some(Event::HostMenu),
        Input::Key{vk:0x79,shift:true,ctrl:false,alt:false}=>Some(Event::HostMenu),
        // Win32 delivers these keys via WM_CHAR, not untyped WM_KEYDOWN.
        // Normalize them for both PILdos setup and guest BIOS keyboard input.
        Input::Char{ch,shift,ctrl,alt}=>{
            let key=match ch{
                '\r'|'\n'=>Key::Enter,
                '\t'=>Key::Tab,
                '\u{1b}'=>Key::Escape,
                '\u{8}'=>Key::Backspace,
                _=>Key::Char(ch),
            };
            Some(Event::Key(key,Mods{shift,ctrl,alt}))
        },
        Input::Key{vk,shift,ctrl,alt}=>windows_key(vk).map(|k|if k==Event::Break{k}else if let Event::Key(k,_)=k{Event::Key(k,Mods{shift,ctrl,alt})}else{k}),
        Input::Mouse{x,y,width,height,buttons,shift}=>Some(Event::Mouse{x,y,width,height,buttons,shift}),
        Input::Menu{id}=>Some(Event::HostCommand(id)),
        Input::Physical{..}|Input::Text{..}|Input::DeadKey|Input::FocusLost=>None,
    }
}
/// Windows virtual-key codes of the keys that type nothing.
#[cfg(any(windows,test))]
fn windows_key(vk:u16)->Option<Event>{
    let m=Mods::default();
    Some(Event::Key(match vk{
        0x03=>return Some(Event::Break),
        0x21=>Key::PageUp,0x22=>Key::PageDown,0x23=>Key::End,0x24=>Key::Home,
        0x25=>Key::Left,0x26=>Key::Up,0x27=>Key::Right,0x28=>Key::Down,
        0x2d=>Key::Insert,0x2e=>Key::Delete,
        0x70..=0x7b=>Key::F((vk-0x6f) as u8),
        _=>return None,
    },m))
}

/// The host's offset from UTC in seconds, for the DOS clock.
fn local_utc_offset()->i64{
    #[cfg(windows)]
    {
        #[repr(C)]
        struct TimeZone{bias:i32,standard_name:[u16;32],standard_date:[u16;8],standard_bias:i32,daylight_name:[u16;32],daylight_date:[u16;8],daylight_bias:i32}
        #[link(name="kernel32")]
        extern "system"{fn GetTimeZoneInformation(info:*mut TimeZone)->u32;}
        let mut info=std::mem::MaybeUninit::<TimeZone>::zeroed();
        let id=unsafe{GetTimeZoneInformation(info.as_mut_ptr())};
        if id==u32::MAX{return 0;}
        let info=unsafe{info.assume_init()};
        let bias=info.bias+if id==2{info.daylight_bias}else{info.standard_bias};
        -i64::from(bias)*60
    }
    #[cfg(not(windows))]
    {
        let text=std::process::Command::new("date").arg("+%z").output().ok()
            .and_then(|o|String::from_utf8(o.stdout).ok()).unwrap_or_default();
        let t=text.trim();
        if t.len()!=5{return 0;}
        let sign=if t.starts_with('-'){-1}else{1};
        let (h,m)=(t[1..3].parse::<i64>().unwrap_or(0),t[3..5].parse::<i64>().unwrap_or(0));
        sign*(h*3600+m*60)
    }
}

#[path="dos_host_terminal.rs"]
mod tty;
// Only the Unix terminal draws graphics in text cells.
#[cfg(any(not(windows),test))]
#[path="dos_terminal_graphics.rs"]
mod terminal_graphics;
#[path="dos_host_speaker.rs"]
pub(crate) mod speaker;
#[path="dos_host_sb.rs"]
mod sb_audio;
#[path="dos_host_audio.rs"]
pub(crate) mod audio;
#[path="dos_console_select.rs"]
pub(crate) mod select;
#[cfg(windows)]
#[path="dos_media_dialog.rs"]
mod media_dialog;
#[path="dos_media_launch.rs"]
pub(crate) mod media_launch;
#[path="dos_catalog.rs"]
pub(crate) mod catalog;

/// The character grid of the screen shown now: (columns, rows).
fn grid(p:&Loaded)->Result<(u32,u32),String>{
    let read=|at,w|p.ram.load(at,w).map_err(|e|format!("BIOS screen size read: {e:?}"));
    let columns=read(0x44a,MemWidth::Word)?.clamp(1,80);
    let rows=if graphics_mode(p){read(0x484,MemWidth::Byte)?+1}else{25};
    Ok((columns,rows))
}

#[cfg(test)]
#[path="../tests/unit/host_console.rs"]
mod tests;
