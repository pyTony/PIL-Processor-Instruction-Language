//! The PC/AT and VGA BIOS firmware on top of `pil_pc::Machine<F>`.
//!
//! Everything here works on any `Machine<F>` without naming Dine's DOS or
//! host front ends: a booted operating system (`Machine<Bios>` or Dine's
//! booted session) and Dine's DOS (`Machine<Dos>`) share the same BIOS
//! interrupt services, vector entries, fonts and parameter tables.
use pil_cpu::address::{MemoryContext,SegmentCache,SegmentRegister};
use pil_cpu::memory::{MemWidth,Ram};
use pil_pc::Machine;

pub mod block_move;
pub mod boot;
pub mod color_text;
pub mod console;
pub mod disk;
pub mod int09;
pub mod int14;
pub mod planar;
pub mod rom_font;
pub mod services;
pub mod vectors;

pub use rom_font::rom;
pub use services::{bios_ticks,bios_timer,dispatch,halt_tick,timer_interrupt,timer_interrupt_at};

/// Plain BIOS firmware for a machine with no host DOS layer (`Machine<Bios>`).
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
pub struct Bios;

/// Whether keyboard scan codes go through IRQ1 to a guest INT 09h handler.
pub fn keyboard_irq_owned<F>(p:&Machine<F>)->Result<bool,String>{
    Ok(!p.rom_mode&&(!p.image.real_mode||vectors::reachable(p,p.speaker.pic.keyboard_vector())?))
}

/// VGA sequencer, graphics controller and attribute ports under BIOS rules.
pub fn vga_controller_io<F>(p:&mut Machine<F>,port:u16,input:bool,value:u16,word:bool)->Result<u16,String>{
    planar::io(p,port,input,value,word)?.ok_or_else(||format!("port {port:04x}h is not a VGA controller port"))
}

impl pil_pc::Firmware for Bios{
    fn keyboard_irq_owned(p:&Machine<Self>)->Result<bool,String>{keyboard_irq_owned(p)}
    fn deliver_irq(p:&mut Machine<Self>,vector:u8)->Result<(),String>{vectors::deliver_irq(p,vector)}
    fn vga_controller_io(p:&mut Machine<Self>,port:u16,input:bool,value:u16,word:bool)->Result<u16,String>{
        vga_controller_io(p,port,input,value,word)
    }
}

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn standalone_bios_boots_sector_and_answers_bios_interrupts(){
        let mut disk_bytes=vec![0u8;1440*1024];
        disk_bytes[..4].copy_from_slice(&[0xb4,0x0e,0xb0,b'B']);
        disk_bytes[510..512].copy_from_slice(&[0x55,0xaa]);
        let mut m=boot::boot_bios(&disk_bytes,0).unwrap();
        assert!(m.booted);
        assert_eq!(m.cpu.eip,0x7c00);
        assert_eq!(m.cpu.regs[2]&0xff,0);
        // INT 10h AH=0Eh writes 'B' to B800:0000 without any DOS layer.
        m.cpu.regs[0]=0x0e42;
        assert_eq!(dispatch(&mut m,0x10).unwrap(),Some(None));
        assert_eq!(m.ram.load(color_text::VIDEO,MemWidth::Byte).unwrap(),u32::from(b'B'));
        // INT 12h reports 640 KiB conventional memory; INT 15h AH=88h reports 15 MiB extended.
        assert_eq!(dispatch(&mut m,0x12).unwrap(),Some(None));
        assert_eq!(m.cpu.regs[0]&0xffff,640);
        m.cpu.regs[0]=0x8800;
        assert_eq!(dispatch(&mut m,0x15).unwrap(),Some(None));
        assert_eq!(m.cpu.regs[0]&0xffff,15*1024);
        // INT 1Ah AH=01h sets the 32-bit BDA tick counter and clears midnight rollover.
        m.ram.store(0x0470,MemWidth::Byte,1).unwrap();
        m.cpu.regs[0]=0x0100;m.cpu.regs[1]=0x0012;m.cpu.regs[2]=0x3456;
        assert_eq!(dispatch(&mut m,0x1a).unwrap(),Some(None));
        assert_eq!(m.ram.load(0x046c,MemWidth::Dword).unwrap(),0x0012_3456);
        assert_eq!(m.ram.load(0x0470,MemWidth::Byte).unwrap(),0);
        m.cpu.regs[0]=0x0000;
        assert_eq!(dispatch(&mut m,0x1a).unwrap(),Some(None));
        assert_eq!((m.cpu.regs[0]&0xff,m.cpu.regs[1]&0xffff,m.cpu.regs[2]&0xffff),(0,0x0012,0x3456));
        // INT 10h AX=1104h loads the ROM 8x16 character set into plane 2 block BL.
        m.cpu.regs[0]=0x1104;m.cpu.regs[3]=0x0000;
        assert_eq!(dispatch(&mut m,0x10).unwrap(),Some(None));
        let expected=rom_font::font().unwrap();
        let plane=m.vga.font.as_ref().expect("font plane populated");
        assert_eq!(&plane.plane(2)[usize::from(b'A')*32..usize::from(b'A')*32+16],&expected[usize::from(b'A')*16..usize::from(b'A')*16+16]);
        assert_eq!(&plane.plane(2)[usize::from(b'A')*32+16..usize::from(b'A')*32+32],&[0;16]);
    }
}
