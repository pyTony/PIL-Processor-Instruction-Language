//! Bounded AT interrupt-controller state. The PIT's IRQ0 and the keyboard's
//! IRQ1 are wired to devices; other request lines remain inactive. ICW initialization, polling,
//! priority rotation and special-mask mode are not modelled.
#[derive(Clone,Debug,Default,PartialEq,Eq)]
struct Controller{mask:u8,request:u8,in_service:u8,read_service:bool}
impl Controller{
    fn command(&mut self,value:u8)->Result<(),String>{
        if value&0x10!=0{return Err(format!("PIC initialization command {value:02X}h is not modelled"));}
        if value&8!=0{
            if value&0xe4!=0{return Err(format!("PIC poll/special-mask command {value:02X}h is not modelled"));}
            if value&2!=0{self.read_service=value&1!=0;}
            return Ok(());
        }
        match value>>5{
            0|2=>Ok(()), // disable automatic rotation / NOP in normal EOI mode
            1=>{if self.in_service!=0{self.in_service&=!(1<<self.in_service.trailing_zeros());}Ok(())},
            3=>{self.in_service&=!(1<<(value&7));Ok(())},
            _=>Err(format!("PIC rotation/priority command {value:02X}h is not modelled")),
        }
    }
    fn read_command(&self)->u8{if self.read_service{self.in_service}else{self.request}}
}
#[derive(Clone,Debug,Default,PartialEq,Eq)]
pub(in crate::dos) struct Pair{master:Controller,slave:Controller}
impl Pair{
    pub fn read(&self,port:u16)->u8{let c=if port&0x80!=0{&self.slave}else{&self.master};if port&1!=0{c.mask}else{c.read_command()}}
    pub fn write(&mut self,port:u16,value:u8)->Result<(),String>{let c=if port&0x80!=0{&mut self.slave}else{&mut self.master};if port&1!=0{c.mask=value;Ok(())}else{c.command(value)}}
    pub fn raise_timer(&mut self){self.master.request|=1;}
    pub fn timer_ready(&self)->bool{self.master.request&!self.master.mask&1!=0&&self.master.in_service&1==0}
    pub fn acknowledge_timer(&mut self){self.master.request&=!1;self.master.in_service|=1;}
    pub fn finish_timer(&mut self){self.master.in_service&=!1;}
    /// IRQ1 can be taken: unmasked, and neither it nor IRQ0 (higher
    /// priority) in service. The request is raised as the code is latched.
    pub fn keyboard_ready(&self)->bool{self.master.mask&2==0&&self.master.in_service&3==0}
    pub fn acknowledge_keyboard(&mut self){self.master.request&=!2;self.master.in_service|=2;}
    /// The BIOS handler's specific EOI for IRQ1.
    pub fn end_keyboard(&mut self){self.master.in_service&=!2;}
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn masks_selection_and_eoi_are_independent_and_persistent(){
        let mut p=Pair::default();p.write(0x21,0xa5).unwrap();p.write(0xa1,0x5a).unwrap();assert_eq!((p.read(0x21),p.read(0xa1)),(0xa5,0x5a));
        p.raise_timer();assert_eq!(p.read(0x20),1);assert!(!p.timer_ready());p.write(0x21,0xa4).unwrap();assert!(p.timer_ready());
        p.acknowledge_timer();assert_eq!(p.read(0x20),0);p.write(0x20,0x0b).unwrap();assert_eq!(p.read(0x20),1);assert_eq!(p.read(0x20),1);
        p.raise_timer();assert!(!p.timer_ready());p.write(0x20,0x20).unwrap();assert_eq!(p.read(0x20),0);assert!(p.timer_ready());
        p.acknowledge_timer();p.write(0x20,0x60).unwrap();assert_eq!(p.read(0x20),0);p.write(0x20,0x0a).unwrap();assert_eq!(p.read(0x20),0);
        let old=p.clone();for value in [0x11,0x0c,0xa0,0x68]{assert!(p.write(0x20,value).is_err());assert_eq!(p,old);}
    }
}
