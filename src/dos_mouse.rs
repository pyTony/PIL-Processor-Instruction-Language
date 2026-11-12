//! Session mouse driver (INT 33h). Standalone probes keep the absent-driver stub.
//! This models DOS polling, not PS/2 IRQ12 or guest callbacks.
use super::*;

#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
struct Edge{count:u16,x:u16,y:u16}
#[derive(Clone,Debug,PartialEq,Eq)]
pub(super) struct Mouse{
    enabled:bool,ready:bool,mode:u8,x:u16,y:u16,buttons:u16,show:i16,
    xmin:u16,xmax:u16,ymin:u16,ymax:u16,screen_x:u16,screen_y:u16,
    pressed:[Edge;3],released:[Edge;3],mx:i32,my:i32,ratio_x:u16,ratio_y:u16,
}
impl Default for Mouse{
    fn default()->Self{Self{enabled:false,ready:false,mode:3,x:320,y:96,buttons:0,show:-1,
        xmin:0,xmax:639,ymin:0,ymax:199,screen_x:639,screen_y:199,
        pressed:[Edge::default();3],released:[Edge::default();3],
        mx:0,my:0,ratio_x:8,ratio_y:16}}
}
fn low(r:&mut u32,v:u16){*r=(*r&0xffff0000)|u32::from(v);}
fn bounds(mode:u8)->(u16,u16){(639,match mode&0x7f{0x10=>349,0x12=>479,_=>199})}
impl Mouse{
    pub(super) fn enable(&mut self){self.enabled=true;}
    pub(super) fn view(&self)->(bool,bool,bool,u16,u16,u16,u16){
        (self.enabled,self.ready,self.ready&&self.show==0,self.x,self.y,self.screen_x,self.screen_y)
    }
    fn reset(&mut self,mode:u8){
        let enabled=self.enabled;*self=Self::default();self.enabled=enabled;
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
        self.mx=self.mx.saturating_add((i32::from(self.x)-i32::from(ox))*i32::from(self.ratio_x)/8);
        self.my=self.my.saturating_add((i32::from(self.y)-i32::from(oy))*i32::from(self.ratio_y)/8);
        self.buttons=buttons&7;
        for i in 0..3{
            let bit=1<<i;
            if ob&bit==self.buttons&bit{continue;}
            let item=if self.buttons&bit!=0{&mut self.pressed[i]}else{&mut self.released[i]};
            item.count=item.count.saturating_add(1);item.x=self.x;item.y=self.y;
        }
    }
    fn call(&mut self,r:&mut [u32;8],mode:u8)->Result<(),String>{
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
            0x0c if r[1]&0xffff==0=>{},
            0x0c=>return Err("INT 33h mouse event callbacks (AX=000Ch) are not implemented".into()),
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
    p.mouse.call(&mut p.cpu.regs,mode)
}
#[cfg(test)]
#[path="../tests/unit/dos_mouse.rs"]
mod tests;
