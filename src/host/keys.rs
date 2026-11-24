//! Native input state machine, independent of Win32 calls and testable on any host.
//! Physical identity survives layout/Shift changes. Text supplies the BIOS word;
//! it never creates a second IRQ make. Terminal taps and paste remain separate.
use super::{Event,Key,Mods,window_event,windows_key};
use crate::dine_window::Input;

#[derive(Default)]
pub(super) struct WindowKeys{
    held:Vec<(u8,bool)>,
    delivered:Vec<(u8,bool)>,
    // TranslateMessage follows a printable down with Text or DeadKey. Wait
    // for that translation so a chained BIOS handler receives the right word.
    pending:Option<(u8,bool)>,
    // Native Alt+letter BIOS words must not depend on receiving WM_SYSCHAR.
    // When Windows also translates the keystroke, suppress that duplicate.
    alt_letters:Vec<(u8,bool)>,
}
fn edge(scan:u8,extended:bool,down:bool,key:Option<(Key,Mods)>)->Event{
    Event::NativeKey{scan,extended,down,key}
}
impl WindowKeys{
    fn flush(&mut self,out:&mut Vec<Event>){
        if let Some((scan,extended))=self.pending.take(){out.push(edge(scan,extended,true,None));}
    }
    pub(super) fn input(&mut self,input:Input)->Vec<Event>{
        let mut out=Vec::new();
        match input{
            Input::Physical{vk,scan,extended,down,repeat,shift,ctrl,alt}=>{
                self.flush(&mut out);
                let id=(scan,extended);
                if !down{
                    self.alt_letters.retain(|k|*k!=id);
                    if let Some(at)=self.held.iter().position(|k|*k==id){
                        self.held.remove(at);out.push(edge(scan,extended,false,None));
                    }
                }else if scan!=0{
                    let m=Mods{shift,ctrl,alt};
                    // Host shortcuts never acquire a guest-held key. Their
                    // release is ignored, even if modifiers change first.
                    let mapped=window_event(Input::Key{vk,shift,ctrl,alt});
                    let shortcut=match mapped{
                        Some(Event::Key(Key::F(1..=4),_)) if ctrl&&shift&&!alt=>mapped,
                        Some(Event::Key(..))|None=>None,
                        other=>other,
                    };
                    if let Some(event)=shortcut{
                        if self.delivered.contains(&id){out.push(edge(scan,extended,false,None));}
                        self.held.retain(|k|*k!=id);
                        out.push(event);
                    }else{
                        // Alt+F4 belongs to the host. Windows keys and Pause's E1
                        // sequence have no set-1 edge representation in this path.
                        if !((alt&&vk==0x73)||matches!(vk,0x5b|0x5c|0x13)){
                            if !self.held.contains(&id){self.held.push(id);}
                            if alt&&!ctrl&&(0x41..=0x5a).contains(&vk){
                                // BIOS Alt+letter is scan:00h; in particular Alt+F
                                // must reach EDIT even when WM_SYSCHAR is consumed
                                // by Windows system-menu processing. WM_SYSCHAR,
                                // if received, must not produce another make.
                                if !self.alt_letters.contains(&id){self.alt_letters.push(id);}
                                let key=(Key::Char(char::from(vk as u8)),m);
                                for _ in 0..repeat.max(1){out.push(edge(scan,extended,true,Some(key)));}
                            }else if let Some(Event::Key(k,_))=windows_key(vk){
                                for _ in 0..repeat.max(1){out.push(edge(scan,extended,true,Some((k,m))));}
                            }else if matches!(vk,0x10..=0x12|0xa0..=0xa5|0x14|0x90|0x91){out.push(edge(scan,extended,true,None));}
                            else{self.pending=Some(id);}
                        }
                    }
                }
            },
            Input::Text{scan,extended,ch,shift,ctrl,alt}=>{
                let id=(scan,extended);
                if self.pending!=Some(id){self.flush(&mut out);}
                self.pending=None;
                // Ignore queued text after focus loss; it must not re-press a key.
                if self.held.contains(&id)&&!self.alt_letters.contains(&id){
                    match window_event(Input::Char{ch,shift,ctrl,alt}){
                        Some(Event::Key(k,m))=>out.push(edge(scan,extended,true,Some((k,m)))),
                        Some(event)=>{
                            if self.delivered.contains(&id){out.push(edge(scan,extended,false,None));}
                            self.held.retain(|k|*k!=id);out.push(event);
                        },
                        None=>out.push(edge(scan,extended,true,None)),
                    }
                }
            },
            Input::FocusLost=>{
                self.flush(&mut out);
                self.alt_letters.clear();
                for (scan,extended) in self.held.drain(..){out.push(edge(scan,extended,false,None));}
            },
            Input::DeadKey=>self.flush(&mut out),
            other=>{if let Some(event)=window_event(other){out.push(event);}},
        }
        for event in &out{
            if let Event::NativeKey{scan,extended,down,..}=*event{
                let id=(scan,extended);
                self.delivered.retain(|k|*k!=id);
                if down{self.delivered.push(id);}
            }
        }
        out
    }
}
