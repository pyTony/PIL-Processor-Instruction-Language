//! Session mouse driver (INT 33h). Standalone probes keep the absent-driver stub.
//! This models DOS polling and real-mode FAR event callbacks, not PS/2 IRQ12.
//! Behaviour follows RBIL's INT 33h list and CuteMouse 2.1, the FreeDOS
//! driver (checked against its documentation and source, not copied).
use super::*;
use std::collections::VecDeque;

#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
struct Edge{count:u16,x:u16,y:u16}
/// The AX=000Ch/0014h routine. Only the low byte of the call mask is kept
/// (bits 7-15 are unused); a zero mask means no routine is called.
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
struct Handler{mask:u16,segment:u16,offset:u16}
impl Handler{fn active(self)->bool{self.mask&0x7f!=0}}
/// One host event; `mx`/`my` are the motion counters (as AX=000Bh would
/// read them) at the time of the event.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
struct Event{flags:u16,buttons:u16,x:u16,y:u16,mx:u16,my:u16}
#[derive(Clone,Debug,PartialEq,Eq)]
struct Frame{
    cpu:Cpu,selectors:[u16;6],segments:[SegmentCache;6],
    ss_shadow:u8,sti_shadow:u8,flag_checkpoint:Option<(u32,u32,u32)>,sp:u16,
}
const EVENT_LIMIT:usize=256;
/// AX=0024h: the Microsoft driver version CuteMouse 2.1 imitates (7.05),
/// and mouse type 4 (PS/2) with interrupt 0, as RBIL defines for PS/2.
const VERSION:u16=0x0705;
/// AX=0015h: bytes in an AX=0016h save buffer (a signature and 78 words).
pub(super) const STATE_BYTES:u16=158;
const STATE_SIGNATURE:u16=0x4d44; // "DM": Dine mouse state
#[derive(Clone,Debug,PartialEq,Eq)]
pub(super) struct Mouse{
    enabled:bool,ready:bool,disabled:bool,mode:u8,x:u16,y:u16,buttons:u16,show:i16,
    xmin:u16,xmax:u16,ymin:u16,ymax:u16,screen_x:u16,screen_y:u16,
    pressed:[Edge;3],released:[Edge;3],mx:i32,my:i32,ratio_x:u16,ratio_y:u16,
    sensitivity:(u16,u16),threshold:u16,
    /// AX=000Ah: 0 for the software cursor, then its screen and cursor masks.
    text_cursor:(u16,u16,u16),
    /// AX=0009h: hot spot and the 16 screen-mask then 16 cursor-mask rows;
    /// None until a program defines one (the frontend draws its own arrow).
    shape:Option<(i16,i16,[u16;32])>,
    /// AX=0010h: the cursor is hidden while inside it; AX=0001h clears it.
    region:Option<(u16,u16,u16,u16)>,
    handler:Handler,events:VecDeque<Event>,frame:Option<Frame>,
    /// The INT 33h vector before the session installed the driver (AX=001Fh).
    previous_vector:u32,
}
impl Default for Mouse{
    fn default()->Self{Self{enabled:false,ready:false,disabled:false,mode:3,x:320,y:96,buttons:0,show:-1,
        xmin:0,xmax:639,ymin:0,ymax:199,screen_x:639,screen_y:199,
        pressed:[Edge::default();3],released:[Edge::default();3],
        mx:0,my:0,ratio_x:8,ratio_y:16,sensitivity:(50,50),threshold:64,
        text_cursor:(0,0x77ff,0x7700),shape:None,region:None,
        handler:Handler::default(),events:VecDeque::new(),frame:None,previous_vector:0}}
}
fn low(r:&mut u32,v:u16){*r=(*r&0xffff0000)|u32::from(v);}
fn bounds(mode:u8)->(u16,u16){(639,match mode&0x7f{0x10=>349,0x12=>479,_=>199})}
fn counter(v:i32)->u16{v.clamp(-32768,32767)as i16 as u16}
/// What the frontend needs to draw the pointer.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) struct View{pub present:bool,pub active:bool,pub visible:bool,pub x:u16,pub y:u16,pub max_x:u16,pub max_y:u16}
impl Mouse{
    pub(super) fn enable(&mut self,previous_vector:u32){self.enabled=true;self.previous_vector=previous_vector;}
    pub(super) fn present(&self)->bool{self.enabled}
    pub(crate) fn view(&self)->View{
        let hidden=self.region.is_some_and(|(x0,y0,x1,y1)|(x0..=x1).contains(&self.x)&&(y0..=y1).contains(&self.y));
        View{present:self.enabled,active:self.ready&&!self.disabled,visible:self.ready&&!self.disabled&&self.show==0&&!hidden,
            x:self.x,y:self.y,max_x:self.screen_x,max_y:self.screen_y}
    }
    /// The software text cursor's masks, when it is the one in use.
    pub(super) fn text_masks(&self)->Option<(u16,u16)>{(self.text_cursor.0==0).then_some((self.text_cursor.1,self.text_cursor.2))}
    pub(super) fn shape(&self)->Option<(i16,i16,[u16;32])>{self.shape}
    /// AX=0000h/0021h: everything but the enable state, the buttons held
    /// now (hardware state), the save of an interrupted callback and the
    /// vector from before installation.
    fn reset(&mut self,mode:u8){
        let keep=(self.enabled,self.disabled,self.buttons,self.frame.take(),self.previous_vector);
        *self=Self::default();(self.enabled,self.disabled,self.buttons,self.frame,self.previous_vector)=keep;
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
        if !self.enabled||!self.ready||self.disabled{return;}
        self.sync(mode);
        let (ox,oy,ob)=(self.x,self.y,self.buttons);
        self.x=x.min(self.screen_x);self.y=y.min(self.screen_y);self.clamp();
        self.mx=self.mx.saturating_add((i32::from(self.x)-i32::from(ox))*i32::from(self.ratio_x)/8);
        self.my=self.my.saturating_add((i32::from(self.y)-i32::from(oy))*i32::from(self.ratio_y)/8);
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
        if self.handler.active()&&flags&self.handler.mask!=0{
            let event=Event{flags,buttons:self.buttons,x:self.x,y:self.y,mx:counter(self.mx),my:counter(self.my)};
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
    fn state(&self)->Vec<u16>{
        let mut w=vec![STATE_SIGNATURE,self.ratio_x,self.ratio_y,self.sensitivity.0,self.sensitivity.1,self.threshold,
            self.text_cursor.0,self.text_cursor.1,self.text_cursor.2];
        let (hx,hy,rows)=self.shape.unwrap_or((0,0,[0;32]));
        w.extend([u16::from(self.shape.is_some()),hx as u16,hy as u16]);w.extend(rows);
        let (x0,y0,x1,y1)=self.region.unwrap_or_default();
        w.extend([self.show as u16,self.xmin,self.xmax,self.ymin,self.ymax,u16::from(self.region.is_some()),x0,y0,x1,y1,
            self.x,self.y,self.handler.mask,self.handler.offset,self.handler.segment,counter(self.mx),counter(self.my)]);
        for e in self.pressed.iter().chain(&self.released){w.extend([e.count,e.x,e.y]);}
        debug_assert_eq!(w.len()*2,usize::from(STATE_BYTES));
        w
    }
    fn restore(&mut self,w:&[u16])->Result<(),String>{
        if w[0]!=STATE_SIGNATURE{return Err(format!("INT 33h AX=0017h buffer has no saved mouse state (signature {:04x}h)",w[0]));}
        (self.ratio_x,self.ratio_y,self.sensitivity,self.threshold)=(w[1].max(1),w[2].max(1),(w[3],w[4]),w[5]);
        self.text_cursor=(w[6],w[7],w[8]);
        self.shape=(w[9]!=0).then(||(w[10]as i16,w[11]as i16,w[12..44].try_into().unwrap()));
        self.show=(w[44]as i16).min(0);
        // Ranges and positions are taken against the current mode's screen.
        self.xmin=w[45].min(self.screen_x);self.xmax=w[46].clamp(self.xmin,self.screen_x);
        self.ymin=w[47].min(self.screen_y);self.ymax=w[48].clamp(self.ymin,self.screen_y);
        self.region=(w[49]!=0).then_some((w[50],w[51],w[52],w[53]));
        self.x=w[54];self.y=w[55];self.clamp();
        self.handler=Handler{mask:w[56],offset:w[57],segment:w[58]};
        self.mx=i32::from(w[59]as i16);self.my=i32::from(w[60]as i16);
        for (i,e) in self.pressed.iter_mut().chain(self.released.iter_mut()).enumerate(){
            *e=Edge{count:w[61+3*i],x:w[62+3*i],y:w[63+3*i]};
        }
        self.events.clear();
        Ok(())
    }
    /// One call with the registers; returns a new ES selector when the
    /// function hands back a far pointer in ES.
    fn call(&mut self,r:&mut [u32;8],mode:u8,es:u16,page:u16)->Result<Option<u16>,String>{
        if !self.enabled{return Ok(None);}
        let f=r[0]as u16;
        if f!=0&&f!=0x21{self.sync(mode);}
        match f{
            // Reset and software reset are the same driver reset; reset also
            // re-enables a disabled driver, as CuteMouse does.
            0|0x21=>{if f==0{self.disabled=false;}self.reset(mode);low(&mut r[0],0xffff);low(&mut r[3],3);},
            1=>{self.show=self.show.saturating_add(1).min(0);self.region=None;},
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
            0x0a if r[3]as u16==0=>self.text_cursor=(0,r[1]as u16,r[2]as u16),
            0x0a=>return Err("INT 33h AX=000Ah hardware text cursor (BX<>0) is not modelled".into()),
            0x0b=>{low(&mut r[1],counter(self.mx));low(&mut r[2],counter(self.my));self.mx=0;self.my=0;},
            0x0c|0x14=>{
                let new=Handler{mask:r[1]as u16&0xff,segment:es,offset:r[2]as u16};
                let old=std::mem::replace(&mut self.handler,new);
                self.events.clear();
                if f==0x14{low(&mut r[1],old.mask);low(&mut r[2],old.offset);return Ok(Some(old.segment));}
            },
            // Light pen emulation (0Dh/0Eh), the large graphics cursor (12h,
            // AX stays 0012h: not done), the InPort interrupt rate (1Ch) and
            // the display page (1Dh) have no effect, as in CuteMouse; the
            // US driver ignores the message language (22h).
            0x0d|0x0e|0x12|0x1c|0x1d|0x22=>{},
            0x0f=>{self.ratio_x=(r[1]as u16).max(1);self.ratio_y=(r[2]as u16).max(1);},
            0x10=>{let(x0,y0,x1,y1)=(r[1]as u16,r[2]as u16,r[6]as u16,r[7]as u16);
                self.region=Some((x0.min(x1),y0.min(y1),x0.max(x1),y0.max(y1)));},
            // No acceleration is modelled: the pointer follows the host's.
            0x13=>self.threshold=if r[2]as u16==0{64}else{r[2]as u16},
            0x15=>low(&mut r[3],STATE_BYTES),
            0x1a=>{self.sensitivity=(r[3]as u16,r[1]as u16);self.threshold=r[2]as u16;},
            0x1b=>{low(&mut r[3],self.sensitivity.0);low(&mut r[1],self.sensitivity.1);low(&mut r[2],self.threshold);},
            0x1e=>low(&mut r[3],page),
            0x1f=>{
                self.disabled=true;self.buttons=0;self.events.clear();
                low(&mut r[0],0x1f);low(&mut r[3],self.previous_vector as u16);
                return Ok(Some((self.previous_vector>>16)as u16));
            },
            0x20=>self.disabled=false,
            0x23=>low(&mut r[3],0),
            0x24=>{low(&mut r[3],VERSION);low(&mut r[1],0x0400);},
            0x26=>{low(&mut r[3],u16::from(self.disabled));low(&mut r[1],self.screen_x);low(&mut r[2],self.screen_y);},
            _=>return Err(format!("INT 33h mouse AX={f:04x}h is not implemented")),
        }
        Ok(None)
    }
}
/// ES:DX+i for `count` bytes, each offset wrapping at 64 KiB, all checked
/// before any is used.
fn buffer(p:&Loaded,count:u16,write:bool)->Result<Vec<u32>,String>{
    let context=MemoryContext{segment:SegmentRegister::Es,cache:p.image.segments[SegmentRegister::Es.index()]};
    let dx=p.cpu.regs[2]&0xffff;
    (0..u32::from(count)).map(|i|{
        let at=context.translate((dx+i)&0xffff,MemWidth::Byte,write,p.cpu.eip).map_err(|e|format!("INT 33h buffer: {e:?}"))?;
        if write{p.ram.check_write(at,MemWidth::Byte)}else{p.ram.load(at,MemWidth::Byte).map(|_|())}
            .map_err(|e|format!("INT 33h buffer: {e:?}"))?;
        Ok(at)
    }).collect()
}
fn words(p:&Loaded,count:u16)->Result<Vec<u16>,String>{
    let at=buffer(p,count*2,false)?;
    let byte=|i:usize|p.ram.load(at[i],MemWidth::Byte).map(|b|b as u16).map_err(|e|format!("INT 33h buffer: {e:?}"));
    (0..usize::from(count)).map(|i|Ok(byte(2*i)?|byte(2*i+1)?<<8)).collect()
}
pub(super) fn service(p:&mut Loaded)->Result<(),String>{
    let mode=p.ram.load(0x449,MemWidth::Byte).map_err(|e|format!("mouse video mode: {e:?}"))?as u8;
    let page=p.ram.load(0x462,MemWidth::Byte).map_err(|e|format!("mouse video page: {e:?}"))?as u16;
    let f=p.cpu.regs[0]as u16;
    if p.mouse.enabled&&f!=0&&f!=0x21{p.mouse.sync(mode);}
    match f{
        0x09 if p.mouse.enabled=>{
            let rows=words(p,32)?;
            p.mouse.shape=Some((p.cpu.regs[3]as i16,p.cpu.regs[1]as i16,rows.try_into().unwrap()));
            return Ok(());
        },
        0x16 if p.mouse.enabled=>{
            let at=buffer(p,STATE_BYTES,true)?;
            for (i,w) in p.mouse.state().into_iter().enumerate(){
                for (j,b) in w.to_le_bytes().into_iter().enumerate(){
                    p.ram.store(at[2*i+j],MemWidth::Byte,u32::from(b)).map_err(|e|format!("INT 33h buffer: {e:?}"))?;
                }
            }
            return Ok(());
        },
        0x17 if p.mouse.enabled=>{let w=words(p,STATE_BYTES/2)?;return p.mouse.restore(&w);},
        _=>{},
    }
    let es=p.selectors[SegmentRegister::Es.index()];
    if let Some(es)=p.mouse.call(&mut p.cpu.regs,mode,es,page)?{
        let i=SegmentRegister::Es.index();
        p.selectors[i]=es;p.image.selectors[i]=es;p.image.segments[i]=SegmentCache::real_mode(es);
    }
    Ok(())
}
pub(super) fn ready(p:&Loaded)->bool{
    p.image.real_mode&&p.cpu.eflags&0x200!=0
        &&p.image.sti_shadow==0&&p.image.ss_shadow==0
        &&p.mouse.frame.is_none()&&p.mouse.handler.active()&&!p.mouse.events.is_empty()
}
/// Run one queued user callback as a FAR call between guest instructions.
/// This is not IRQ12: the software driver has no hardware controller.
pub(super) fn deliver(p:&mut Loaded)->Result<bool,String>{
    if !ready(p){return Ok(false);}
    let handler=p.mouse.handler;
    let Some(event)=p.mouse.events.front().copied() else{return Ok(false);};
    let conditions=event.flags&handler.mask&0x7f;
    if conditions==0{p.mouse.events.pop_front();return Ok(false);}
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
    // RBIL table 03172: AX = the conditions that matched the call mask,
    // BX buttons, CX/DX position, SI/DI the horizontal/vertical motion
    // counters (as CuteMouse passes them), DS the driver's data.
    low(&mut p.cpu.regs[0],conditions);
    low(&mut p.cpu.regs[3],event.buttons);
    low(&mut p.cpu.regs[1],event.x);
    low(&mut p.cpu.regs[2],event.y);
    low(&mut p.cpu.regs[6],event.mx);
    low(&mut p.cpu.regs[7],event.my);
    let ds=SegmentRegister::Ds.index();
    p.selectors[ds]=DOSDATA;p.image.selectors[ds]=DOSDATA;
    p.image.segments[ds]=SegmentCache::real_mode(DOSDATA);
    let cs=SegmentRegister::Cs.index();
    p.selectors[cs]=handler.segment;p.image.selectors[cs]=handler.segment;
    p.image.segments[cs]=SegmentCache::real_mode(handler.segment);
    p.cpu.eip=u32::from(handler.offset);
    // CuteMouse enables interrupts (STI) before the far call; IF was set
    // for delivery, so only TF is cleared.
    p.cpu.eflags&=!0x100;p.cpu.undefined_flags&=!0x100;
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
