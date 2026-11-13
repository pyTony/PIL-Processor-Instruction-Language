//! `pil --dos-console`: a DOS session in this process with its own keyboard
//! and screen, for every program alike.
//!
//! Two front ends share one loop. `--tty` draws the text page in the host
//! terminal with ANSI sequences and reads keys in raw mode; on Windows a
//! graphics mode opens the console window too. `--window` (Windows) is a
//! Win32 window that draws the text page with the ROM font, the graphics
//! modes through `desktop_vga_frame`, and takes keyboard and mouse input.
//!
//! Host keys become the words the IBM BIOS stores (scan code high, ASCII
//! low) and go through the BIOS keyboard ring, or IRQ1 when the program owns
//! INT 09h, exactly as `--dos-session` keys do. Line editing and history are
//! therefore the program's own: a shell's arrows, command recall and
//! Ctrl+C work as they would on a PC. Nothing here knows which program runs.
use super::session::Session;
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
    /// Ctrl+Break.
    Break,
    /// The pointer at a fraction of the drawn screen: x/width, y/height.
    Mouse{x:i32,y:i32,width:i32,height:i32,buttons:u16},
    Quit,
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
#[cfg_attr(windows,allow(dead_code))]
pub(crate) struct VtParser{pending:Vec<u8>}
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
enum Parsed{Done(Option<Event>,usize),More}

/// IBM PC symbols for the glyphs of 00h-1Fh and 7Fh.
const LOW:[char;32]=[' ','☺','☻','♥','♦','♣','♠','•','◘','○','◙','♂','♀','♪','♫','☼','►','◄','↕','‼','¶','§','▬','↨','↑','↓','→','←','∟','↔','▲','▼'];
fn glyph(b:u8)->char{
    match b{0..=31=>LOW[usize::from(b)],0x7f=>'⌂',_=>crate::dos_encoding::decode(&[b]).chars().next().unwrap_or(' ')}
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

/// The 16 colours of the CGA text attribute, as GDI 0x00RRGGBB.
#[cfg_attr(not(windows),allow(dead_code))]
const COLORS:[u32;16]=[0x000000,0x0000aa,0x00aa00,0x00aaaa,0xaa0000,0xaa00aa,0xaa5500,0xaaaaaa,
    0x555555,0x5555ff,0x55ff55,0x55ffff,0xff5555,0xff55ff,0xffff55,0xffffff];
/// The text page drawn with the 8x8 ROM font, each font line twice (8x16
/// cells, 640x400 for 80 columns). The cursor is drawn when `cursor` is set:
/// (row, column, first line, last line) on the 16-line cell.
#[cfg_attr(not(windows),allow(dead_code))]
pub(crate) fn text_pixels(cells:&[u8],columns:u32,font:&[u8],cursor:Option<(u32,u32,u32,u32)>)->(usize,usize,Vec<u32>){
    let columns=columns as usize;let rows=cells.len()/(columns*2);
    let (width,height)=(columns*8,rows*16);
    let mut pixels=vec![0u32;width*height];
    for row in 0..rows{for column in 0..columns{
        let at=(row*columns+column)*2;
        let (ch,attr)=(usize::from(cells[at]),cells[at+1]);
        let (fg,bg)=(COLORS[usize::from(attr&15)],COLORS[usize::from((attr>>4)&7)]);
        let under=cursor.filter(|c|(c.0 as usize,c.1 as usize)==(row,column));
        for y in 0..16{
            let bits=font.get(ch*8+y/2).copied().unwrap_or(0);
            let solid=under.is_some_and(|c|(c.2 as usize..=c.3 as usize).contains(&y));
            let line=&mut pixels[(row*16+y)*width+column*8..][..8];
            for (x,p) in line.iter_mut().enumerate(){*p=if solid||bits&(0x80>>x)!=0{fg}else{bg};}
        }
    }}
    (width,height,pixels)
}
/// The BIOS cursor (0040:0050 for the page, shape at 0040:0060) on a 16-line
/// cell, or None when it is off.
fn cursor_shape(p:&Loaded)->Option<(u32,u32)>{
    let byte=|at|p.ram.load(at,MemWidth::Byte).ok();
    let (end,start)=(byte(0x460)?,byte(0x461)?);
    if start&0x20!=0||start&0x1f>end&0x1f{return None;}
    let (start,end)=(start&0x1f,end&0x1f);
    // An 8-line CGA shape (6-7) doubles; a taller one is already in lines.
    Some(if end<8{(start*2,end*2+1)}else{(start.min(15),end.min(15))})
}
fn graphics_mode(p:&Loaded)->bool{
    p.ram.load(0x449,MemWidth::Byte).is_ok_and(|m|!matches!(m&0x7f,0..=3|7))
}

/// Options of `pil --dos-console`.
pub(crate) struct Options{pub(crate) window:bool,pub(crate) mouse:bool,pub(crate) ansi:bool,pub(crate) country:Option<u16>,pub(crate) utc_offset:Option<i64>}

/// Run the session until the program ends, it stops, or the user quits.
/// Returns the closing line for the host terminal.
pub(crate) fn run(session:&mut Session,options:&Options)->Result<String,String>{
    let offset=options.utc_offset.unwrap_or_else(local_utc_offset);
    for command in [Some(format!("clock {offset}")),options.country.map(|c|format!("country {c}")),
        options.ansi.then(||"ansi on".to_string()),options.mouse.then(||"mouse on".to_string())].into_iter().flatten(){
        let (reply,_)=session.command(0,&command)?;
        if reply.contains("\"ok\":false"){return Err(format!("{command}: {reply}"));}
    }
    let (tx,rx)=mpsc::channel();
    let mut front=Front::open(options.window,tx)?;
    let result=drive(session,&mut front,&rx);
    drop(front);
    result
}

fn drive(session:&mut Session,front:&mut Front,rx:&mpsc::Receiver<Event>)->Result<String,String>{
    const SLICE:usize=20_000;
    let mut pending=std::collections::VecDeque::<u16>::new();
    let mut last_present=Instant::now()-Duration::from_secs(1);
    let mut note=None::<String>;
    loop{
        // Input first: keys wait while the BIOS ring is full.
        let mut events:Vec<Event>=rx.try_iter().collect();
        front.poll(&mut events);
        for event in events{
            match event{
                Event::Quit=>return Ok("DOS console closed by the user".into()),
                Event::Break=>{if let Err(refused)=session.ctrl_break()?{note=Some(refused);}},
                Event::Key(key,mods)=>if let Some(word)=bios_word(key,mods){pending.push_back(word);},
                Event::Mouse{x,y,width,height,buttons}=>{
                    if width>0&&height>0{
                        let view=session.machine().map(|p|p.mouse.view()).ok_or("session state lost")?;
                        let fx=(x.clamp(0,width-1) as i64*(i64::from(view.max_x)+1)/i64::from(width)) as u16;
                        let fy=(y.clamp(0,height-1) as i64*(i64::from(view.max_y)+1)/i64::from(height)) as u16;
                        session.mouse(fx,fy,buttons)?;
                    }
                },
            }
        }
        while let Some(&word)=pending.front(){
            if session.key(word as u8,(word>>8) as u8)?{pending.pop_front();}else{break;}
        }
        // Run for about one frame.
        let until=Instant::now()+Duration::from_millis(15);
        loop{
            session.run(SLICE)?;
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
struct Front{
    tty:Option<tty::Terminal>,
    #[cfg(windows)]
    window:Option<WindowFront>,
    #[cfg(windows)]
    window_tx:Option<mpsc::Sender<Event>>,
    held:Vec<Event>,
    previous:Option<Vec<u8>>,
    graphics_note:bool,
}
impl Front{
    fn open(window:bool,tx:mpsc::Sender<Event>)->Result<Self,String>{
        #[cfg(not(windows))]
        if window{return Err("the DOS console window needs Windows; use --tty".into());}
        let tty=if window{None}else{Some(tty::Terminal::open(tx.clone())?)};
        #[cfg(windows)]
        let (window,window_tx)=if window{(Some(WindowFront::new(tx)),None)}else{(None,Some(tx))};
        #[cfg(not(windows))]
        let _=tx;
        Ok(Self{tty,#[cfg(windows)]window,#[cfg(windows)]window_tx,held:Vec::new(),previous:None,graphics_note:false})
    }
    fn poll(&mut self,events:&mut Vec<Event>){
        events.splice(0..0,self.held.drain(..));
        if let Some(t)=self.tty.as_mut(){t.poll(events);}
    }
    fn unpoll(&mut self,event:Event){self.held.push(event);}
    fn closed(&self)->bool{
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
            if let Some(w)=self.window.as_mut(){w.present(session)?;}
        }
        if let Some(t)=self.tty.as_mut(){
            if graphics{
                if !self.graphics_note{
                    self.graphics_note=true;self.previous=None;
                    let mode=p.ram.load(0x449,MemWidth::Byte).unwrap_or(0);
                    #[cfg(windows)]
                    let text=format!("\x1b[0m\x1b[2J\x1b[1;1HGraphics mode {mode:02X}h: shown in the Dine window; keys work there and here.");
                    #[cfg(not(windows))]
                    let text=format!("\x1b[0m\x1b[2J\x1b[1;1HGraphics mode {mode:02X}h: this terminal shows text modes only; keys still reach the program.");
                    t.write(&text)?;
                }
            }else{
                self.graphics_note=false;
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
            for input in raw{
                let event=match input{
                    crate::dine_window::Input::Char{ch,shift,ctrl,alt}=>Some(Event::Key(Key::Char(ch),Mods{shift,ctrl,alt})),
                    crate::dine_window::Input::Key{vk,shift,ctrl,alt}=>windows_key(vk).map(|k|if k==Event::Break{k}else if let Event::Key(k,_)=k{Event::Key(k,Mods{shift,ctrl,alt})}else{k}),
                    crate::dine_window::Input::Mouse{x,y,width,height,buttons}=>Some(Event::Mouse{x,y,width,height,buttons}),
                };
                if let Some(event)=event{if tx.send(event).is_err(){break;}}
            }
        }).ok();
        let font=super::console::graphics_text::font().unwrap_or_else(|_|vec![0;2048]);
        Self{window:crate::dine_window::Window::interactive("Dine DOS console",input),font,started:Instant::now(),last:None}
    }
    fn closed(&self)->bool{self.window.closed()}
    fn present(&mut self,session:&Session)->Result<(),String>{
        let p=session.machine().ok_or("session state lost")?;
        let frame=if graphics_mode(p){desktop_vga_frame(p)}else{
            let (cells,columns,(row,column))=session.screen()?;
            // The hardware cursor blinks at about 3.75 Hz (16 frames of 60).
            let on=self.started.elapsed().as_millis()/267%2==0;
            let cursor=cursor_shape(p).filter(|_|on).map(|(a,b)|(row,column,a,b));
            Some(text_pixels(&cells,columns,&self.font,cursor))
        };
        let Some(frame)=frame else{self.window.no_frame();return Ok(());};
        if self.last.as_ref()==Some(&frame){return Ok(());}
        self.window.present(crate::dine_window::Frame{width:frame.0 as i32,height:frame.1 as i32,pixels:frame.2.clone()});
        self.last=Some(frame);
        Ok(())
    }
}
/// Windows virtual-key codes of the keys that type nothing.
#[cfg(windows)]
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

/// The host terminal in raw mode, on the alternate screen; restored on drop.
mod tty{
    use super::Event;
    #[cfg(not(windows))]
    use super::VtParser;
    use std::io::Write as _;
    use std::sync::mpsc;
    #[cfg(not(windows))]
    use std::time::{Duration,Instant};

    pub(super) struct Terminal{
        #[cfg(not(windows))]
        saved:Option<String>,
        #[cfg(not(windows))]
        bytes:mpsc::Receiver<u8>,
        #[cfg(not(windows))]
        parser:VtParser,
        #[cfg(not(windows))]
        last_byte:Instant,
        #[cfg(windows)]
        modes:Option<(u32,u32,u32)>,
    }
    impl Terminal{
        #[cfg(not(windows))]
        pub(super) fn open(_events:mpsc::Sender<Event>)->Result<Self,String>{
            use std::process::{Command,Stdio};
            let saved=Command::new("stty").arg("-g").stdin(Stdio::inherit()).output().ok()
                .filter(|o|o.status.success()).and_then(|o|String::from_utf8(o.stdout).ok()).map(|s|s.trim().to_string());
            if saved.is_some(){
                let _=Command::new("stty").args(["raw","-echo"]).stdin(Stdio::inherit()).status();
            }
            let (tx,bytes)=mpsc::channel();
            std::thread::Builder::new().name("dine-console-stdin".into()).spawn(move||{
                use std::io::Read as _;
                let mut stdin=std::io::stdin();let mut buf=[0u8;64];
                while let Ok(n)=stdin.read(&mut buf){
                    if n==0{break;}
                    if buf[..n].iter().any(|b|tx.send(*b).is_err()){break;}
                }
            }).map_err(|e|e.to_string())?;
            let t=Self{saved,bytes,parser:VtParser::default(),last_byte:Instant::now()};
            t.write("\x1b[?1049h\x1b[2J")?;
            Ok(t)
        }
        #[cfg(not(windows))]
        pub(super) fn poll(&mut self,out:&mut Vec<Event>){
            let mut got=false;
            while let Ok(b)=self.bytes.try_recv(){self.parser.feed(b,out);got=true;}
            if got{self.last_byte=Instant::now();}
            else if self.parser.waiting()&&self.last_byte.elapsed()>=Duration::from_millis(40){self.parser.flush(out);}
        }
        #[cfg(windows)]
        pub(super) fn open(events:mpsc::Sender<Event>)->Result<Self,String>{
            let modes=windows::raw()?;
            windows::spawn_reader(events)?;
            let t=Self{modes:Some(modes)};
            t.write("\x1b[?1049h\x1b[2J")?;
            Ok(t)
        }
        #[cfg(windows)]
        pub(super) fn poll(&mut self,_out:&mut Vec<Event>){}
        pub(super) fn write(&self,text:&str)->Result<(),String>{
            let mut out=std::io::stdout().lock();
            out.write_all(text.as_bytes()).and_then(|_|out.flush()).map_err(|e|e.to_string())
        }
    }
    impl Drop for Terminal{
        fn drop(&mut self){
            let _=self.write("\x1b[0m\x1b[?25h\x1b[?1049l");
            #[cfg(not(windows))]
            if let Some(saved)=self.saved.take(){
                let _=std::process::Command::new("stty").arg(saved).stdin(std::process::Stdio::inherit()).status();
            }
            #[cfg(windows)]
            if let Some(modes)=self.modes.take(){windows::restore(modes);}
        }
    }

    /// Windows console: raw key events with their scan codes.
    #[cfg(windows)]
    mod windows{
        use super::super::{Event,Key,Mods};
        use std::ffi::c_void;
        use std::sync::mpsc;
        type Handle=*mut c_void;
        const STD_INPUT:u32=-10i32 as u32;
        const STD_OUTPUT:u32=-11i32 as u32;
        const ENABLE_PROCESSED_OUTPUT:u32=1;
        const ENABLE_VIRTUAL_TERMINAL_PROCESSING:u32=4;
        const ENABLE_WINDOW_INPUT:u32=8;
        const ENABLE_EXTENDED_FLAGS:u32=0x80;
        #[repr(C)]
        #[derive(Clone,Copy)]
        struct KeyEvent{down:i32,repeat:u16,vk:u16,scan:u16,ch:u16,state:u32}
        #[repr(C)]
        #[derive(Clone,Copy)]
        struct Record{kind:u16,_pad:u16,key:KeyEvent}
        #[link(name="kernel32")]
        extern "system"{
            fn GetStdHandle(which:u32)->Handle;
            fn GetConsoleMode(console:Handle,mode:*mut u32)->i32;
            fn SetConsoleMode(console:Handle,mode:u32)->i32;
            fn GetConsoleOutputCP()->u32;
            fn SetConsoleOutputCP(cp:u32)->i32;
            fn ReadConsoleInputW(console:Handle,records:*mut Record,length:u32,read:*mut u32)->i32;
        }
        pub(super) fn raw()->Result<(u32,u32,u32),String>{
            unsafe{
                let (input,output)=(GetStdHandle(STD_INPUT),GetStdHandle(STD_OUTPUT));
                let (mut im,mut om)=(0u32,0u32);
                if GetConsoleMode(input,&mut im)==0||GetConsoleMode(output,&mut om)==0{
                    return Err("--tty needs a Windows console (run it in a terminal window)".into());
                }
                let cp=GetConsoleOutputCP();
                // No line input, echo or Ctrl+C processing: every key reaches DOS.
                SetConsoleMode(input,ENABLE_WINDOW_INPUT|ENABLE_EXTENDED_FLAGS);
                SetConsoleMode(output,om|ENABLE_PROCESSED_OUTPUT|ENABLE_VIRTUAL_TERMINAL_PROCESSING);
                SetConsoleOutputCP(65001);
                Ok((im,om,cp))
            }
        }
        pub(super) fn restore((im,om,cp):(u32,u32,u32)){
            unsafe{
                SetConsoleMode(GetStdHandle(STD_INPUT),im);
                SetConsoleMode(GetStdHandle(STD_OUTPUT),om);
                SetConsoleOutputCP(cp);
            }
        }
        pub(super) fn spawn_reader(events:mpsc::Sender<Event>)->Result<(),String>{
            let input=unsafe{GetStdHandle(STD_INPUT)} as usize;
            std::thread::Builder::new().name("dine-console-keys".into()).spawn(move||{
                let mut surrogate=None::<u16>;
                loop{
                    let mut records=[Record{kind:0,_pad:0,key:KeyEvent{down:0,repeat:0,vk:0,scan:0,ch:0,state:0}};16];
                    let mut read=0u32;
                    if unsafe{ReadConsoleInputW(input as Handle,records.as_mut_ptr(),16,&mut read)}==0{break;}
                    for r in &records[..read as usize]{
                        if r.kind!=1||r.key.down==0{continue;}
                        let k=r.key;
                        let m=Mods{shift:k.state&0x10!=0,ctrl:k.state&0x0c!=0,alt:k.state&0x03!=0};
                        // Keys named by their virtual-key code first: those that
                        // type nothing, and Enter/Backspace/Tab/Esc whatever Ctrl does.
                        let event=match k.vk{
                            0x03=>Some(Event::Break),
                            0x08=>Some(Event::Key(Key::Backspace,m)),0x09=>Some(Event::Key(Key::Tab,m)),
                            0x0d=>Some(Event::Key(Key::Enter,m)),0x1b=>Some(Event::Key(Key::Escape,m)),
                            0x21=>Some(Event::Key(Key::PageUp,m)),0x22=>Some(Event::Key(Key::PageDown,m)),
                            0x23=>Some(Event::Key(Key::End,m)),0x24=>Some(Event::Key(Key::Home,m)),
                            0x25=>Some(Event::Key(Key::Left,m)),0x26=>Some(Event::Key(Key::Up,m)),
                            0x27=>Some(Event::Key(Key::Right,m)),0x28=>Some(Event::Key(Key::Down,m)),
                            0x2d=>Some(Event::Key(Key::Insert,m)),0x2e=>Some(Event::Key(Key::Delete,m)),
                            0x70..=0x7b=>Some(Event::Key(Key::F((k.vk-0x6f) as u8),m)),
                            _ if k.ch==0=>None,
                            _=>{
                                let unit=k.ch;
                                let ch=if (0xd800..0xdc00).contains(&unit){surrogate=Some(unit);continue;}
                                    else if (0xdc00..0xe000).contains(&unit){
                                        let Some(high)=surrogate.take() else{continue};
                                        char::from_u32(0x10000+((u32::from(high)-0xd800)<<10)+(u32::from(unit)-0xdc00))
                                    }else{char::from_u32(u32::from(unit))};
                                let Some(ch)=ch else{continue};
                                // Ctrl+] ends the console, as in the terminal front end.
                                if ch=='\u{1d}'{let _=events.send(Event::Quit);return;}
                                // AltGr is right Alt with left Ctrl: a typed character.
                                let altgr=m.ctrl&&m.alt&&!ch.is_control();
                                let m=if altgr{Mods{shift:m.shift,..Mods::default()}}else{m};
                                // Ctrl+letter arrives as its control code; keep the letter.
                                let ch=if (1..=26).contains(&(ch as u32)){char::from(ch as u8+0x60)}else{ch};
                                Some(Event::Key(Key::Char(ch),m))
                            },
                        };
                        if let Some(event)=event{
                            for _ in 0..k.repeat.max(1){if events.send(event).is_err(){return;}}
                        }
                    }
                }
            }).map(|_|()).map_err(|e|e.to_string())
        }
    }
}

#[cfg(test)]
#[path="../tests/unit/host_console.rs"]
mod tests;
