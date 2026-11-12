//! Session mouse driver (INT 33h). Standalone probes keep the absent-driver stub.
//! This models DOS polling and real-mode FAR event callbacks, not PS/2 IRQ12.
use super::*;
use std::collections::VecDeque;

#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
struct Edge{count:u16,x:u16,y:u16}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
struct Handler{mask:u16,segment:u16,offset:u16}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
struct Event{flags:u16,buttons:u16,x:u16,y:u16,vertical:i16,horizontal:i16}
#[derive(Clone,Debug,PartialEq,Eq)]
struct Frame{
    cpu:Cpu,selectors:[u16;6],segments:[SegmentCache;6],
    ss_shadow:u8,sti_shadow:u8,flag_checkpoint:Option<(u32,u32,u32)>,sp:u16,
}
const EVENT_LIMIT:usize=256;
#[derive(Clone,Debug,PartialEq,Eq)]
pub(super) struct Mouse{
    enabled:bool,ready:bool,mode:u8,x:u16,y:u16,buttons:u16,show:i16,
    xmin:u16,xmax:u16,ymin:u16,ymax:u16,screen_x:u16,screen_y:u16,
    pressed:[Edge;3],released:[Edge;3],mx:i32,my:i32,ratio_x:u16,ratio_y:u16,
    handler:Option<Handler>,events:VecDeque<Event>,frame:Option<Frame>,
}
impl Default for Mouse{
    fn default()->Self{Self{enabled:false,ready:false,mode:3,x:320,y:96,buttons:0,show:-1,
        xmin:0,xmax:639,ymin:0,ymax:199,screen_x:639,screen_y:199,
        pressed:[Edge::default();3],released:[Edge::default();3],
        mx:0,my:0,ratio_x:8,ratio_y:16,handler:None,events:VecDeque::new(),frame:None}}
}
fn low(r:&mut u32,v:u16){*r=(*r&0xffff0000)|u32::from(v);}
fn bounds(mode:u8)->(u16,u16){(639,match mode&0x7f{0x10=>349,0x12=>479,_=>199})}
impl Mouse{
    pub(super) fn enable(&mut self){self.enabled=true;}
    pub(super) fn view(&self)->(bool,bool,bool,u16,u16,u16,u16){
        (self.enabled,self.ready,self.ready&&self.show==0,self.x,self.y,self.screen_x,self.screen_y)
    }
    fn reset(&mut self,mode:u8){
        let enabled=self.enabled;let frame=self.frame.take();*self=Self::default();self.enabled=enabled;self.frame=frame;
        self.ready=true;self.mode=mode;(self.screen_x,self.screen_y)=bounds(mode);
        self.xmax=self.screen_x;self.ymax=self.screen_y;
        self.x=(self.screen_x+1)/2;self.y=(self.screen_y+1)/2;self.clamp();
    }
    fn sync(&mut self,mode:u8){if self.ready&&self.mode!=mode{self.reset(mode);}}
    fn clamp(&mut self){
        self.x=self.x.clamp(self.xmin,self.xmax);self.y=self.y.clamp(self.ymin,self.ymax);
        if self.mode&0x7f<=3{self.x=self.x/8*8;self.y=self.y/8*8;}
    }
    pub(super) fn host(&mut self,x:u16,y:u16,buttons:u16,mode:u8){
        if !self.enabled||!self.ready{return;}
        self.sync(mode);
        let (ox,oy,ob)=(self.x,self.y,self.buttons);
        self.x=x.min(self.screen_x);self.y=y.min(self.screen_y);self.clamp();
        let horizontal=(i32::from(self.x)-i32::from(ox))*i32::from(self.ratio_x)/8;
        let vertical=(i32::from(self.y)-i32::from(oy))*i32::from(self.ratio_y)/8;
        self.mx=self.mx.saturating_add(horizontal);
        self.my=self.my.saturating_add(vertical);
        self.buttons=buttons&7;
        let mut flags=if (ox,oy)!=(self.x,self.y){1u16}else{0};
        for i in 0..3{
            let bit=1<<i;
            if ob&bit==self.buttons&bit{continue;}
            let pressed=self.buttons&bit!=0;
            let item=if pressed{&mut self.pressed[i]}else{&mut self.released[i]};
            item.count=item.count.saturating_add(1);item.x=self.x;item.y=self.y;
            flags|=1<<(1+2*i+usize::from(!pressed));
        }
        if self.handler.is_some_and(|h|flags&h.mask!=0){
            let event=Event{flags,buttons:self.buttons,x:self.x,y:self.y,
                vertical:vertical.clamp(-32768,32767)as i16,
                horizontal:horizontal.clamp(-32768,32767)as i16};
            // Replace pure motion with the latest position, but never merge
            // a button edge into a neighboring update.
            if flags==1&&self.events.back().is_some_and(|e|e.flags==1){
                *self.events.back_mut().unwrap()=event;
            }else{
                if self.events.len()==EVENT_LIMIT{
                    if let Some(i)=self.events.iter().position(|e|e.flags==1){
                        self.events.remove(i);
                    }else{self.events.pop_front();}
                }
                self.events.push_back(event);
            }
        }
    }
    fn call(&mut self,r:&mut [u32;8],mode:u8,es:u16)->Result<(),String>{
        if !self.enabled{return Ok(());}
        let f=r[0]as u16;
        if f!=0{self.sync(mode);}
        match f{
            0=>{self.reset(mode);low(&mut r[0],0xffff);low(&mut r[3],3);},
            1=>self.show=self.show.saturating_add(1).min(0),
            2=>self.show=self.show.saturating_sub(1),
            3=>{low(&mut r[3],self.buttons);low(&mut r[1],self.x);low(&mut r[2],self.y);},
            4=>{self.x=r[1]as u16;self.y=r[2]as u16;self.clamp();},
            5|6=>{
                let i=(r[3]&0xffff)as usize;
                if i>=3{return Err(format!("INT 33h button {i} outside 0..2"));}
                let e=if f==5{&mut self.pressed[i]}else{&mut self.released[i]};
                low(&mut r[0],self.buttons);low(&mut r[3],e.count);
                low(&mut r[1],e.x);low(&mut r[2],e.y);e.count=0;
            },
            7=>{let(a,b)=(r[1]as u16,r[2]as u16);
                self.xmin=a.min(b).min(self.screen_x);self.xmax=a.max(b).min(self.screen_x);self.clamp();},
            8=>{let(a,b)=(r[1]as u16,r[2]as u16);
                self.ymin=a.min(b).min(self.screen_y);self.ymax=a.max(b).min(self.screen_y);self.clamp();},
            0x0b=>{low(&mut r[1],self.mx.clamp(-32768,32767)as i16 as u16);
                low(&mut r[2],self.my.clamp(-32768,32767)as i16 as u16);self.mx=0;self.my=0;},
            0x0c=>{
                let mask=r[1]as u16;
                if mask&!0x7f!=0{return Err(format!("INT 33h callback reserved event bits {mask:04x}h"));}
                self.handler=(mask!=0).then_some(Handler{mask,segment:es,offset:r[2]as u16});
                self.events.clear();
            },
            0x0f=>{self.ratio_x=(r[1]as u16).max(1);self.ratio_y=(r[2]as u16).max(1);},
            0x21=>{low(&mut r[0],0xffff);low(&mut r[3],3);},
            0x26=>{low(&mut r[3],0);low(&mut r[1],self.screen_x);low(&mut r[2],self.screen_y);},
            _=>return Err(format!("INT 33h mouse AX={f:04x}h is not implemented")),
        }
        Ok(())
    }
}
pub(super) fn service(p:&mut Loaded)->Result<(),String>{
    let mode=p.ram.load(0x449,MemWidth::Byte).map_err(|e|format!("mouse video mode: {e:?}"))?as u8;
    let es=p.selectors[SegmentRegister::Es.index()];
    p.mouse.call(&mut p.cpu.regs,mode,es)
}
/// Run one queued user callback as a FAR call between guest instructions.
/// This is not IRQ12: the software driver has no hardware controller.
pub(super) fn deliver(p:&mut Loaded)->Result<bool,String>{
    if !p.image.real_mode||p.cpu.eflags&0x200==0||p.image.sti_shadow!=0||p.image.ss_shadow!=0
        ||p.mouse.frame.is_some(){return Ok(false);}
    let (Some(handler),Some(event))=(p.mouse.handler,p.mouse.events.front().copied())else{return Ok(false);};
    if event.flags&handler.mask==0{p.mouse.events.pop_front();return Ok(false);}
    let target=u32::from(handler.segment)*16+u32::from(handler.offset);
    let inside=(target>=p.image.fetch_base+p.image.start&&target<p.image.fetch_base+p.image.end)
        ||p.image.system.iter().any(|&(a,b)|target>=a&&target<b);
    if !inside{return Err(format!("INT 33h callback at {:04x}:{:04x} is outside executable DOS memory",handler.segment,handler.offset));}
    let sp=p.cpu.regs[4]&0xffff;
    let context=MemoryContext{segment:SegmentRegister::Ss,cache:p.image.segments[SegmentRegister::Ss.index()]};
    let mut writes=Vec::new();
    for (delta,value) in [(2,u32::from(vectors::MOUSE_RETURN_SEGMENT)),
        (4,u32::from(vectors::MOUSE_RETURN_OFFSET))]{
        let offset=sp.wrapping_sub(delta)&0xffff;
        let at=context.translate(offset,MemWidth::Word,true,p.cpu.eip)
            .map_err(|e|format!("mouse callback stack: {e:?}"))?;
        p.ram.check_write(at,MemWidth::Word).map_err(|e|format!("mouse callback stack: {e:?}"))?;
        writes.push((at,value));
    }
    let frame=Frame{cpu:p.cpu.clone(),selectors:p.selectors,segments:p.image.segments,
        ss_shadow:p.image.ss_shadow,sti_shadow:p.image.sti_shadow,
        flag_checkpoint:p.image.flag_checkpoint,sp:sp as u16};
    for (at,value) in writes{
        p.ram.store(at,MemWidth::Word,value).map_err(|e|format!("mouse callback stack write: {e:?}"))?;
    }
    p.mouse.events.pop_front();p.mouse.frame=Some(frame);
    p.cpu.regs[4]=(p.cpu.regs[4]&0xffff0000)|(sp.wrapping_sub(4)&0xffff);
    low(&mut p.cpu.regs[0],event.flags);
    low(&mut p.cpu.regs[3],event.buttons);
    low(&mut p.cpu.regs[1],event.x);
    low(&mut p.cpu.regs[2],event.y);
    // Microsoft callback convention: SI = vertical and DI = horizontal mickeys.
    low(&mut p.cpu.regs[6],event.vertical as u16);
    low(&mut p.cpu.regs[7],event.horizontal as u16);
    let ds=SegmentRegister::Ds.index();
    p.selectors[ds]=DOSDATA;p.image.selectors[ds]=DOSDATA;
    p.image.segments[ds]=SegmentCache::real_mode(DOSDATA);
    let cs=SegmentRegister::Cs.index();
    p.selectors[cs]=handler.segment;p.image.selectors[cs]=handler.segment;
    p.image.segments[cs]=SegmentCache::real_mode(handler.segment);
    p.cpu.eip=u32::from(handler.offset);
    p.cpu.eflags&=!0x300;p.cpu.undefined_flags&=!0x300;
    p.image.sti_shadow=0;p.image.ss_shadow=0;p.image.flag_checkpoint=None;
    Ok(true)
}
/// A callback must RETF to the driver continuation. Restore the interrupted
/// registers and segment caches, without rolling back guest memory changes.
pub(super) fn returned(p:&mut Loaded)->Result<bool,String>{
    let at=p.image.segments[SegmentRegister::Cs.index()].base.wrapping_add(p.cpu.eip);
    if at!=vectors::MOUSE_RETURN{return Ok(false);}
    let frame=p.mouse.frame.take().ok_or("mouse callback return reached without a pending far call")?;
    if p.cpu.regs[4]&0xffff!=u32::from(frame.sp){
        return Err(format!("mouse callback returned with SP={:04x}, expected {:04x} (RETF required)",p.cpu.regs[4]&0xffff,frame.sp));
    }
    p.cpu=frame.cpu;
    p.selectors=frame.selectors;p.image.selectors=frame.selectors;p.image.segments=frame.segments;
    p.image.ss_shadow=frame.ss_shadow;p.image.sti_shadow=frame.sti_shadow;
    p.image.flag_checkpoint=frame.flag_checkpoint;
    Ok(true)
}
#[cfg(test)]
#[path="../tests/unit/dos_mouse.rs"]
mod tests;
