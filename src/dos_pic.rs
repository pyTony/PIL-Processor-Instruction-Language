//! Bounded AT interrupt-controller state: a master 8259A at ports 20h/21h
//! with a slave at A0h/A1h on its IR2. The PIT's IRQ0 and the keyboard's
//! IRQ1 are wired to devices; other request lines remain inactive.
//!
//! Reinitialization (ICW1-ICW4) is modelled for the AT wiring: ICW1 with
//! edge triggering, cascade mode and ICW4 to follow; ICW2 moves the vector
//! base (its low three bits are ignored in 8086 mode); ICW3 must describe the
//! slave on IR2 (04h to the master, ID 2 to the slave); ICW4 selects 8086
//! mode, with or without automatic EOI. As on the 8259A, ICW1 clears the
//! mask register, sets status reads to the IRR and resets the edge sense,
//! so a pending request is dropped; the in-service register is cleared too,
//! as PC emulators commonly do (the data sheet does not list it). Level
//! triggering, single mode, other cascade wiring, buffered mode, special
//! fully nested mode, polling, priority rotation and special-mask mode stop
//! the run as not modelled.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum Init{Ready,Icw2,Icw3,Icw4}
#[derive(Clone,Debug,PartialEq,Eq)]
struct Controller{mask:u8,request:u8,in_service:u8,read_service:bool,
    /// The vector of IR0 (ICW2 & F8h).
    base:u8,auto_eoi:bool,init:Init,slave:bool}
impl Controller{
    fn new(slave:bool,base:u8)->Self{Controller{mask:0,request:0,in_service:0,read_service:false,base,auto_eoi:false,init:Init::Ready,slave}}
    fn name(&self)->&'static str{if self.slave{"slave"}else{"master"}}
    fn command(&mut self,value:u8)->Result<(),String>{
        if value&0x10!=0{return self.icw1(value);}
        if self.init!=Init::Ready{return Err(format!("PIC {} command {value:02X}h during initialization is not modelled",self.name()));}
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
    /// ICW1: checked whole before anything changes.
    fn icw1(&mut self,value:u8)->Result<(),String>{
        let what=if value&8!=0{"level triggering"}else if value&2!=0{"single (uncascaded) mode"}else if value&1==0{"no ICW4 (8080/8085 mode)"}else{""};
        if !what.is_empty(){return Err(format!("PIC {} initialization command {value:02X}h ({what}) is not modelled",self.name()));}
        self.mask=0;self.request=0;self.in_service=0;self.read_service=false;self.auto_eoi=false;
        self.init=Init::Icw2;
        Ok(())
    }
    /// A write to the odd port: the mask, or the next ICW.
    fn data(&mut self,value:u8)->Result<(),String>{
        match self.init{
            Init::Ready=>{self.mask=value;Ok(())},
            Init::Icw2=>{self.base=value&0xf8;self.init=Init::Icw3;Ok(())},
            Init::Icw3=>{
                let wired=if self.slave{value&7==2}else{value==4};
                if !wired{return Err(format!("PIC {} ICW3 {value:02X}h: cascade wiring other than the AT's slave on IR2 is not modelled",self.name()));}
                self.init=Init::Icw4;Ok(())
            },
            Init::Icw4=>{
                if value&1==0{return Err(format!("PIC {} ICW4 {value:02X}h (8080/8085 mode) is not modelled",self.name()));}
                if value&0x1c!=0{return Err(format!("PIC {} ICW4 {value:02X}h (buffered or special fully nested mode) is not modelled",self.name()));}
                self.auto_eoi=value&2!=0;self.init=Init::Ready;Ok(())
            },
        }
    }
    fn read_command(&self)->u8{if self.read_service{self.in_service}else{self.request}}
    /// The interrupt acknowledge for line `bit`: IRR to ISR, or straight
    /// through with automatic EOI.
    fn acknowledge(&mut self,bit:u8){self.request&=!bit;if !self.auto_eoi{self.in_service|=bit;}}
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub(in crate::dos) struct Pair{master:Controller,slave:Controller}
/// The BIOS's power-on programming: IRQ0-7 at INT 08h-0Fh, IRQ8-15 at 70h-77h.
impl Default for Pair{fn default()->Self{Pair{master:Controller::new(false,0x08),slave:Controller::new(true,0x70)}}}
impl Pair{
    pub fn read(&self,port:u16)->u8{let c=if port&0x80!=0{&self.slave}else{&self.master};if port&1!=0{c.mask}else{c.read_command()}}
    pub fn write(&mut self,port:u16,value:u8)->Result<(),String>{let c=if port&0x80!=0{&mut self.slave}else{&mut self.master};if port&1!=0{c.data(value)}else{c.command(value)}}
    /// The vector IRQ0 is delivered through (INT 08h after the BIOS).
    pub fn timer_vector(&self)->u8{self.master.base}
    /// The vector IRQ1 is delivered through (INT 09h after the BIOS).
    pub fn keyboard_vector(&self)->u8{self.master.base|1}
    pub fn raise_timer(&mut self){self.master.request|=1;}
    /// IRQ0 can be taken: requested, unmasked, not in service, and neither
    /// controller halfway through initialization.
    pub fn timer_ready(&self)->bool{self.ready()&&self.master.request&!self.master.mask&1!=0&&self.master.in_service&1==0}
    pub fn acknowledge_timer(&mut self){self.master.acknowledge(1);}
    pub fn finish_timer(&mut self){self.master.in_service&=!1;}
    /// IRQ1 can be taken: unmasked, and neither it nor IRQ0 (higher
    /// priority) in service. The request is raised as the code is latched.
    pub fn keyboard_ready(&self)->bool{self.ready()&&self.master.mask&2==0&&self.master.in_service&3==0}
    pub fn acknowledge_keyboard(&mut self){self.master.acknowledge(2);}
    /// The BIOS handler's specific EOI for IRQ1.
    pub fn end_keyboard(&mut self){self.master.in_service&=!2;}
    fn ready(&self)->bool{self.master.init==Init::Ready}
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
        let old=p.clone();for value in [0x0c,0xa0,0x68]{assert!(p.write(0x20,value).is_err());assert_eq!(p,old);}
    }
    /// CWSDPMI's sequence (Quake's DPMI host): 11h, base, 04h, 01h.
    #[test]fn reinitialization_moves_the_vector_base_and_resets_state(){
        let mut p=Pair::default();assert_eq!((p.timer_vector(),p.keyboard_vector()),(0x08,0x09));
        p.write(0x21,0xb8).unwrap();p.raise_timer();p.acknowledge_timer();p.write(0x20,0x0b).unwrap();assert_eq!(p.read(0x20),1);p.raise_timer();
        p.write(0x20,0x11).unwrap();
        // ICW1 clears IMR, IRR and ISR and sets reads back to the IRR.
        assert_eq!((p.read(0x21),p.read(0x20)),(0,0));
        let mut p2=Pair::default();p2.write(0x20,0x11).unwrap();
        // Halfway through, no IRQ is taken and OCWs to the even port stop.
        p2.raise_timer();assert!(!p2.timer_ready());assert!(!p2.keyboard_ready());
        let held=p2.clone();assert!(p2.write(0x20,0x20).is_err());assert_eq!(p2,held);
        p2.write(0x21,0x7f).unwrap();assert_eq!(p2.timer_vector(),0x78);// low bits ignored
        p2.write(0x21,0x04).unwrap();p2.write(0x21,0x01).unwrap();
        assert_eq!((p2.timer_vector(),p2.keyboard_vector()),(0x78,0x79));
        // The odd port is the mask register again.
        p2.write(0x21,0x01).unwrap();assert_eq!(p2.read(0x21),0x01);assert!(!p2.timer_ready());p2.write(0x21,0).unwrap();assert!(p2.timer_ready());
        p2.acknowledge_timer();p2.write(0x20,0x0b).unwrap();assert_eq!(p2.read(0x20),1);
        // Restoring the BIOS base.
        p2.write(0x20,0x11).unwrap();p2.write(0x21,0x08).unwrap();p2.write(0x21,0x04).unwrap();p2.write(0x21,0x01).unwrap();
        assert_eq!(p2.timer_vector(),0x08);assert_eq!(p2.read(0x20),0);
        // The slave: ID 2, base 70h, its own state only.
        p2.write(0xa0,0x11).unwrap();p2.write(0xa1,0x70).unwrap();p2.write(0xa1,0x02).unwrap();p2.write(0xa1,0x01).unwrap();
        assert_eq!(p2.timer_vector(),0x08);
    }
    #[test]fn automatic_eoi_leaves_nothing_in_service(){
        let mut p=Pair::default();for v in [0x11,0x08,0x04,0x03]{p.write(0x20+u16::from(v!=0x11),v).unwrap();}
        p.raise_timer();assert!(p.timer_ready());p.acknowledge_timer();p.write(0x20,0x0b).unwrap();assert_eq!(p.read(0x20),0);
        p.write(0x20,0x0a).unwrap();assert_eq!(p.read(0x20),0);p.raise_timer();assert!(p.timer_ready());
        assert!(p.keyboard_ready());p.acknowledge_keyboard();assert!(p.keyboard_ready());
    }
    #[test]fn unmodelled_initialization_words_stop_without_changes(){
        // ICW1: level, single, no ICW4.
        for v in [0x19,0x13,0x10]{let mut p=Pair::default();p.write(0x21,0x55).unwrap();let old=p.clone();assert!(p.write(0x20,v).is_err());assert_eq!(p,old);}
        // ICW3 other wiring; ICW4 8080 mode, buffered, SFNM.
        let start=|slave:bool|{let mut p=Pair::default();let b=if slave{0xa0}else{0x20};p.write(b,0x11).unwrap();p.write(b+1,0x70).unwrap();p};
        for(slave,v)in [(false,0x02),(false,0x05),(true,0x04),(true,0x01)]{let mut p=start(slave);let old=p.clone();assert!(p.write(if slave{0xa1}else{0x21},v).is_err());assert_eq!(p,old);}
        for v in [0x00,0x02,0x05,0x09,0x0d,0x11]{let mut p=start(false);p.write(0x21,0x04).unwrap();let old=p.clone();assert!(p.write(0x21,v).is_err(),"{v:02x}");assert_eq!(p,old);}
    }
}
