//! COM1: 16450 UART at 3F8h-3FFh with interrupt enable/identification,
//! loopback modem status, PIC IRQ4 and an optional Microsoft 2-button serial
//! mouse. BIOS serial calls (INT 14h, `bios::int14`), DOS's AUX device
//! (`dos/aux_device.rs`) and booted operating systems share the port. Bytes
//! the program sends wait in `tx` for the frontend; incoming bytes wait in `rx`.
use super::*;
use std::collections::VecDeque;
/// The port's I/O base, as the BIOS data area lists it.
pub const BASE:u16=0x3f8;
/// Line status: transmitter empty and holding register empty; bit 0 is added
/// while a received byte waits.
pub const LSR_EMPTY:u8=0x60;
/// Modem status: the frontend is always there, so CTS, DSR and DCD are on.
pub const MSR:u8=0xb0;
/// Maximum bytes queued in `rx` so untouched mouse motion does not grow unboundedly.
const RX_LIMIT:usize=256;

#[derive(Clone,Debug,Default)]
pub struct Serial{
    pub tx:Vec<u8>,pub rx:VecDeque<u8>,
    /// Line control (bit 7 selects the divisor latch), modem control,
    /// interrupt enable (bits 0-3), scratch and the divisor latch.
    pub lcr:u8,pub mcr:u8,pub ier:u8,pub scratch:u8,pub divisor:u16,
    /// Pending transmitter-holding-register-empty interrupt (cleared on THR write or IIR 02h read).
    thre_int:bool,
    /// Last sampled modem-status input bits (4-7) and latched delta bits (0-3).
    msr_state:u8,msr_delta:u8,
    /// Current level driven toward master PIC IRQ4 (rising edge sets IRR).
    irq_line:bool,
    /// Whether a Microsoft 2-button serial mouse is plugged into COM1.
    pub mouse:bool,
    /// The AUX device's information word low byte for INT 21h AX=4400h/4401h:
    /// a device, not at end of input; CTTY sets bits 0-1 (console input and
    /// output) and bit 5 (raw).
    pub info:u8,
    /// The column DOS output on AUX has reached, as the FreeDOS kernel's
    /// scr_pos counts it: line input there starts a cancelled line below it.
    pub column:usize,
}
impl Serial{
    pub fn lsr(&self)->u8{LSR_EMPTY|u8::from(!self.rx.is_empty())}
    fn msr_lines(&self)->u8{
        if self.mcr&0x10!=0{
            // 8250/16450 internal loopback maps MCR RTS->CTS, DTR->DSR, OUT1->RI, OUT2->DCD.
            let cts=if self.mcr&0x02!=0{0x10}else{0};
            let dsr=if self.mcr&0x01!=0{0x20}else{0};
            let ri=if self.mcr&0x04!=0{0x40}else{0};
            let dcd=if self.mcr&0x08!=0{0x80}else{0};
            cts|dsr|ri|dcd
        }else{MSR}
    }
    fn refresh_msr(&mut self){
        let new=self.msr_lines();
        let old=self.msr_state;
        let diff=new^old;
        if diff&0x10!=0{self.msr_delta|=0x01;}
        if diff&0x20!=0{self.msr_delta|=0x02;}
        if old&0x40!=0&&new&0x40==0{self.msr_delta|=0x04;}
        if diff&0x80!=0{self.msr_delta|=0x08;}
        self.msr_state=new;
    }
    /// Priority-encoded IIR value without clearing the THRE latch.
    pub fn iir_peek(&self)->u8{
        if self.ier&0x01!=0&&!self.rx.is_empty(){0x04}
        else if self.ier&0x02!=0&&self.thre_int{0x02}
        else if self.ier&0x08!=0&&self.msr_delta!=0{0x00}
        else{0x01}
    }
    /// True when OUT2 gates interrupts on (and loopback is off) with an enabled source pending.
    pub fn irq_active(&self)->bool{self.mcr&0x18==0x08&&self.iir_peek()!=0x01}
    /// Drive master PIC IRQ4 on a rising edge of the UART's interrupt output.
    pub fn update_irq(&mut self,pic:&mut crate::ports::pic::Pair){
        let active=self.irq_active();
        if active&&!self.irq_line{pic.raise(4);}
        self.irq_line=active;
    }
    /// Attach a Microsoft 2-button serial mouse to COM1.
    pub fn attach_mouse(&mut self){self.mouse=true;}
    /// True while a guest driver has DTR, RTS, OUT2 and receive interrupts enabled.
    pub fn mouse_active(&self)->bool{self.mouse&&self.mcr&0x1b==0x0b&&self.ier&0x01!=0}
    /// Queue Microsoft 3-byte relative motion/button packets (`buttons`: bit 0 left, bit 1 right).
    pub fn mouse_motion(&mut self,dx:i32,dy:i32,buttons:u16,pic:&mut crate::ports::pic::Pair){
        if !self.mouse||self.mcr&0x13!=0x03{return;}
        let left=u8::from(buttons&1!=0);
        let right=u8::from(buttons&2!=0);
        let mut rx_rem=dx;
        let mut ry_rem=dy;
        let mut first=true;
        while first||rx_rem!=0||ry_rem!=0{
            first=false;
            if self.rx.len()+3>RX_LIMIT{break;}
            let sx=rx_rem.clamp(-127,127) as i8;
            let sy=ry_rem.clamp(-127,127) as i8;
            rx_rem-=i32::from(sx);
            ry_rem-=i32::from(sy);
            let ux=sx as u8;
            let uy=sy as u8;
            let b0=0x40|(left<<5)|(right<<4)|(((uy>>6)&3)<<2)|((ux>>6)&3);
            let b1=ux&0x3f;
            let b2=uy&0x3f;
            self.rx.push_back(b0);
            self.rx.push_back(b1);
            self.rx.push_back(b2);
        }
        self.update_irq(pic);
    }
}
/// Lists COM1 in the BIOS data area (0040:0000 and one serial port in the
/// equipment word) and gives the session the port.
pub fn attach<F>(p:&mut Machine<F>)->Result<(),String>{
    let equipment=p.ram.load(0x410,MemWidth::Word).map_err(|e|format!("BIOS equipment read: {e:?}"))?;
    p.ram.store(0x400,MemWidth::Word,u32::from(BASE)).map_err(|e|format!("BIOS COM1 address: {e:?}"))?;
    p.ram.store(0x410,MemWidth::Word,equipment&!0x0e00|0x0200).map_err(|e|format!("BIOS equipment write: {e:?}"))?;
    p.com1=Some(Serial{info:0xc0,msr_state:MSR,..Serial::default()});Ok(())
}
pub fn port<F>(p:&mut Machine<F>)->Result<&mut Serial,String>{p.com1.as_mut().ok_or_else(||"COM1 is not attached (only a DOS session has it)".into())}
/// One byte IN or OUT at a serial port; `None` when the port is not one.
/// A UART that is not there (COM2-COM4 always, COM1 unless a session
/// attached it) is an open bus: reads give FFh and writes go nowhere, which
/// is how drivers find that the port is absent.
pub fn io<F>(p:&mut Machine<F>,port_number:u16,input:bool,value:u8)->Result<Option<u8>,String>{
    let com1=(BASE..BASE+8).contains(&port_number);
    let other=matches!(port_number,0x2f8..=0x2ff|0x3e8..=0x3ef|0x2e8..=0x2ef);
    if !com1&&!other{return Ok(None);}
    if other||p.com1.is_none(){return Ok(Some(0xff));}
    let s=p.com1.as_mut().unwrap();
    let pic=&mut p.speaker.pic;
    let latch=s.lcr&0x80!=0;
    let out=match(port_number-BASE,input){
        (0,true) if latch=>s.divisor as u8,
        (1,true) if latch=>(s.divisor>>8)as u8,
        (0,false) if latch=>{s.divisor=s.divisor&0xff00|u16::from(value);0},
        (1,false) if latch=>{s.divisor=s.divisor&0xff|u16::from(value)<<8;0},
        (0,true)=>s.rx.pop_front().unwrap_or(0),
        (0,false)=>{
            if s.mcr&0x10!=0{if s.rx.len()<RX_LIMIT{s.rx.push_back(value);}}
            else{s.tx.push(value);}
            s.thre_int=true;
            // Re-arm edge if THRE interrupt is enabled while already pending.
            if s.ier&0x02!=0&&s.iir_peek()==0x02{s.irq_line=false;}
            0
        },
        (1,true)=>s.ier,
        (1,false)=>{
            s.ier=value&0x0f;
            s.thre_int=s.ier&0x02!=0;
            if s.ier&0x02!=0&&s.iir_peek()==0x02{s.irq_line=false;}
            0
        },
        // 16450 IIR (reading 02h clears THRE); FCR writes are taken and ignored (no FIFO).
        (2,true)=>{
            let iir=s.iir_peek();
            if iir==0x02{s.thre_int=false;}
            iir
        },
        (2,false)=>0,
        (3,true)=>s.lcr,(3,false)=>{s.lcr=value;0},
        (4,true)=>s.mcr,(4,false)=>{
            let old=s.mcr;
            s.mcr=value&0x1f;
            s.refresh_msr();
            if s.mouse&&(s.mcr&0x10)==0&&(old&0x03)!=0x03&&(s.mcr&0x03)==0x03{
                s.rx.clear();
                s.rx.push_back(b'M');
            }
            0
        },
        (5,true)=>s.lsr(),
        (6,true)=>{
            s.refresh_msr();
            let val=s.msr_state|s.msr_delta;
            s.msr_delta=0;
            val
        },
        (5|6,false)=>0,
        (7,true)=>s.scratch,(7,false)=>{s.scratch=value;0},
        _=>unreachable!(),
    };
    s.update_irq(pic);
    Ok(Some(out))
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn loopback_reflects_mcr_to_msr_and_loops_tx_to_rx(){
        let mut pic=crate::ports::pic::Pair::default();
        let mut s=Serial{info:0xc0,msr_state:MSR,..Serial::default()};
        // 16450 loopback probe as Linux serial autoconfig() runs it.
        s.mcr=0x10;s.refresh_msr();assert_eq!(s.msr_state&0xf0,0x00);
        s.mcr=0x1e;s.refresh_msr();assert_eq!(s.msr_state&0xf0,0xd0);
        s.mcr=0x1f;s.refresh_msr();assert_eq!(s.msr_state&0xf0,0xf0);
        s.ier=0x01;s.update_irq(&mut pic);
        assert!(!s.irq_active(),"loopback holds external IRQ4 off");
    }
    #[test]fn iir_priority_and_irq4_gating(){
        let mut pic=crate::ports::pic::Pair::default();
        let mut s=Serial{info:0xc0,msr_state:MSR,..Serial::default()};
        s.mcr=0x0b; // DTR | RTS | OUT2
        s.ier=0x03; // RDA | THRE
        s.thre_int=true;
        assert_eq!(s.iir_peek(),0x02);
        s.update_irq(&mut pic);
        assert!(pic.master_ready(4));
        pic.acknowledge_master(4);
        pic.write(0x20,0x20).unwrap();
        // Reading IIR clears THRE; RDA takes priority over THRE when rx has data.
        s.rx.push_back(0x41);
        assert_eq!(s.iir_peek(),0x04);
        s.rx.pop_front();
        assert_eq!(s.iir_peek(),0x02);
        s.thre_int=false;
        assert_eq!(s.iir_peek(),0x01);
        assert!(!s.irq_active());
    }
    #[test]fn serial_mouse_sends_id_on_dtr_rts_and_encodes_packets(){
        let mut pic=crate::ports::pic::Pair::default();
        let mut s=Serial{info:0xc0,msr_state:MSR,..Serial::default()};
        s.attach_mouse();
        s.ier=0x01;
        // Outside loopback, asserting DTR+RTS emits 'M' and raises IRQ4 once OUT2 is set.
        let old=s.mcr;
        s.mcr=0x0b;
        if (old&3)!=3&&(s.mcr&3)==3{s.rx.push_back(b'M');}
        s.update_irq(&mut pic);
        assert_eq!(s.rx.pop_front(),Some(b'M'));
        assert!(s.mouse_active());
        assert!(pic.master_ready(4));
        pic.acknowledge_master(4);
        pic.write(0x20,0x20).unwrap();
        // Relative motion: dx=+10, dy=-5, left+right buttons (3).
        s.mouse_motion(10,-5,3,&mut pic);
        let p:[u8;3]=[s.rx.pop_front().unwrap(),s.rx.pop_front().unwrap(),s.rx.pop_front().unwrap()];
        // dx=10 (0x0A, top 2 bits 0), dy=-5 (0xFB, top 2 bits 3): b0 = 0x40|0x20|0x10|(3<<2)|0 = 0x7C
        assert_eq!(p,[0x7c,10,(-5i8 as u8)&0x3f]);
        // Large deltas split across multiple 3-byte packets (max 127 per packet).
        s.mouse_motion(200,0,1,&mut pic);
        assert_eq!(s.rx.len(),6);
        let first:[u8;3]=[s.rx.pop_front().unwrap(),s.rx.pop_front().unwrap(),s.rx.pop_front().unwrap()];
        let second:[u8;3]=[s.rx.pop_front().unwrap(),s.rx.pop_front().unwrap(),s.rx.pop_front().unwrap()];
        let dx1=(((first[0]&0x03)<<6)|first[1]) as i8;
        let dx2=(((second[0]&0x03)<<6)|second[1]) as i8;
        assert_eq!((i32::from(dx1),i32::from(dx2)),(127,73));
        // Closing interrupts or OUT2 deactivates mouse_active immediately.
        s.ier=0;assert!(!s.mouse_active());
    }
}
