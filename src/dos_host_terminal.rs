//! Host terminal operations only. BIOS mapping and screen rendering are shared
//! with the Rust window frontend. Polling owns no background reader thread.
use super::{Event,Key,Mods};
use crossterm::{event::{self,KeyCode,KeyEventKind,KeyModifiers},terminal};
use std::{io::{self,Write},sync::mpsc,time::Duration};

pub(super) struct Terminal{active:bool,#[cfg(windows)] _output:OutputMode}
impl Terminal{
    pub(super) fn open(_:mpsc::Sender<Event>)->Result<Self,String>{
        #[cfg(windows)]
        let output=OutputMode::open()?;
        terminal::enable_raw_mode().map_err(|e|e.to_string())?;
        let mut host=Self{active:true,#[cfg(windows)] _output:output};
        if let Err(e)=host.write("\x1b[?1049h\x1b[?7l\x1b[2J"){
            host.restore();return Err(e);
        }
        Ok(host)
    }
    pub(super) fn poll(&mut self,out:&mut Vec<Event>)->Result<(),String>{
        // Bound one poll so a flood of input cannot starve DOS execution.
        for _ in 0..64{
            if !event::poll(Duration::ZERO).map_err(|e|e.to_string())?{break;}
            if let event::Event::Key(e)=event::read().map_err(|e|e.to_string())?{
                if e.kind==KeyEventKind::Release{continue;}
                let m=Mods{shift:e.modifiers.contains(KeyModifiers::SHIFT),ctrl:e.modifiers.contains(KeyModifiers::CONTROL),alt:e.modifiers.contains(KeyModifiers::ALT)};
                if m.ctrl&&e.code==KeyCode::Char(']'){out.push(Event::Quit);continue;}
                let key=match e.code{
                    KeyCode::Char(c)=>Key::Char(c),KeyCode::Enter=>Key::Enter,KeyCode::Backspace=>Key::Backspace,
                    KeyCode::Tab|KeyCode::BackTab=>Key::Tab,KeyCode::Esc=>Key::Escape,
                    KeyCode::Up=>Key::Up,KeyCode::Down=>Key::Down,KeyCode::Left=>Key::Left,KeyCode::Right=>Key::Right,
                    KeyCode::Home=>Key::Home,KeyCode::End=>Key::End,KeyCode::PageUp=>Key::PageUp,KeyCode::PageDown=>Key::PageDown,
                    KeyCode::Insert=>Key::Insert,KeyCode::Delete=>Key::Delete,KeyCode::F(n)=>Key::F(n),_=>continue,
                };
                out.push(Event::Key(key,Mods{shift:m.shift||e.code==KeyCode::BackTab,..m}));
            }
        }
        Ok(())
    }
    pub(super) fn write(&self,text:&str)->Result<(),String>{
        let mut out=io::stdout().lock();out.write_all(text.as_bytes()).and_then(|_|out.flush()).map_err(|e|e.to_string())
    }
    fn restore(&mut self){
        if self.active{
            let _=self.write("\x1b[0m\x1b[?25h\x1b[?7h\x1b[?1049l");
            let _=terminal::disable_raw_mode();self.active=false;
        }
    }
}
impl Drop for Terminal{fn drop(&mut self){self.restore();}}

#[cfg(windows)]
struct OutputMode{handle:*mut std::ffi::c_void,saved:u32}
#[cfg(windows)]
#[link(name="kernel32")]
extern "system"{
    fn GetStdHandle(which:u32)->*mut std::ffi::c_void;
    fn GetConsoleMode(handle:*mut std::ffi::c_void,mode:*mut u32)->i32;
    fn SetConsoleMode(handle:*mut std::ffi::c_void,mode:u32)->i32;
}
#[cfg(windows)]
impl OutputMode{
    fn open()->Result<Self,String>{unsafe{
        let handle=GetStdHandle(-11i32 as u32);let mut saved=0;
        if GetConsoleMode(handle,&mut saved)==0||SetConsoleMode(handle,saved|4)==0{
            return Err("Rust DOS text output requires a Windows terminal with VT processing".into());
        }
        Ok(Self{handle,saved})
    }}
}
#[cfg(windows)]
impl Drop for OutputMode{fn drop(&mut self){unsafe{SetConsoleMode(self.handle,self.saved);}}}
