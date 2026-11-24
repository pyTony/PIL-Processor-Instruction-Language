//! BIOS POST and disk-sector bootstrap, independent of Dine's DOS.
use super::*;
use pil_cpu::Cpu;
use pil_cpu::guest::{Image,SYSTEM_RANGES};
use pil_pc::disks::Disk;

/// Default physical RAM installed in a PC session (16 MiB).
pub const DEFAULT_RAM:usize=16*1024*1024;

/// Vectors the BIOS itself serves after POST: timer and keyboard IRQs,
/// video, equipment, memory size, disk, serial, system services, keyboard,
/// printer, bootstrap, clock, Ctrl-Break and the user timer hook.
pub const BIOS:[u8;14]=[0x08,0x09,0x10,0x11,0x12,0x13,0x14,0x15,0x16,0x17,0x19,0x1a,0x1b,0x1c];

/// Vectors a BIOS points at a bare IRET: the CPU exceptions 00h-07h, the
/// master PIC's IRQs 2-7 (0Ah-0Fh) and the slave PIC's IRQs 70h-77h.
pub fn dummy(n:u8)->bool{n<8||(0x0a..=0x0f).contains(&n)||(0x70..=0x77).contains(&n)}

/// Attach BIOS drives, clear conventional memory above the BDA, set POST IVT
/// vectors, parameter tables and equipment word, and read sector 0 to 0000:7C00.
pub fn setup_boot<F>(p:&mut Machine<F>,floppy:Option<(u8,Disk)>,fixed:Vec<Disk>,drive:u8)->Result<(),String>{
    if let Some((at,disk))=floppy{p.disks.attach(at,disk)?;}
    for(unit,disk)in fixed.into_iter().enumerate(){p.disks.attach(0x80+unit as u8,disk)?;}
    p.booted=true;
    p.vga.free=true;
    p.ram.write_bytes(0x500,&vec![0;0xa0000-0x500]).map_err(|e|format!("clearing conventional memory: {e:?}"))?;
    for n in 0..=255u8{
        let vector=if BIOS.contains(&n){vectors::entry(n)}else if dummy(n){vectors::entry(n)+2}else{
            if matches!(n,0x1d|0x1f|0x43){continue;}
            0
        };
        p.ram.store(u32::from(n)*4,MemWidth::Dword,vector).map_err(|e|format!("INT {n:02x}h vector: {e:?}"))?;
    }
    disk::install_table(p)?;
    disk::install_fixed_table(p)?;
    let floppies=p.disks.floppies();
    let equipment=p.ram.load(0x410,MemWidth::Word).map_err(|e|format!("BIOS equipment read: {e:?}"))? as u16;
    let equipment=(equipment&!0x00c1)|if floppies>0{1|((floppies-1)&3)<<6}else{0};
    p.ram.store(0x410,MemWidth::Word,u32::from(equipment)).map_err(|e|format!("BIOS equipment write: {e:?}"))?;
    p.image.fetch_base=0;p.image.start=0;p.image.end=p.ram.bytes().len() as u32;
    disk::load_boot_sector(p,drive)
}

/// Build a freshly POSTed `Machine<F>` and boot `drive` without touching any DOS layer.
pub fn prepare<F>(dos:F,ram_bytes:usize,floppy:Option<(u8,Disk)>,fixed:Vec<Disk>,drive:u8)->Result<Machine<F>,String>{
    let mut ram=Ram::new(ram_bytes);ram.set_a20_enabled(false);
    console::initialize(&mut ram)?;
    rom_font::rom(&mut ram)?;
    ram.store(0x410,MemWidth::Word,0x0025).map_err(|e|format!("BIOS equipment init: {e:?}"))?;
    ram.store(0x460,MemWidth::Word,0x0607).map_err(|e|format!("BIOS cursor init: {e:?}"))?;
    let selectors=[0u16;6];
    let segments=selectors.map(SegmentCache::real_mode);
    let mut system=[(0,0);SYSTEM_RANGES];
    system[1]=(vectors::ENTRIES,vectors::END);
    system[2]=(0x100000,ram_bytes as u32);
    let mut p=Machine{
        dos,cpu:Cpu::default(),ram,
        image:Image{
            fetch_base:0,start:0,end:ram_bytes as u32,system,segments,selectors,
            msw:0x10,real_mode:true,ss_shadow:0,sti_shadow:0,
            tables:[(0,0),pil_cpu::interrupt::RESET_IDTR],controls:[0;2],
            flag_checkpoint:None,ldt_tr:[Default::default(),pil_cpu::address::TR_RESET],
            paging:false,debug:pil_cpu::address::DEBUG_RESET,
        },
        selectors,timer_ticks:0,timer_tick_adjust:0,
        speaker:pil_pc::ports::Speaker::default(),
        sb:pil_pc::soundblaster::SoundBlaster::default(),
        clock_offset:None,cmos_index:0,cmos_shutdown:0,
        rtc:pil_pc::rtc::Rtc::default(),com1:None,
        cache:pil_cpu::cache::DecodeCache::default(),
        rom_mode:false,fpu:pil_cpu::fpu::Fpu::default(),
        vga:pil_pc::vga::Vga::default(),blocks:Default::default(),
        history:pil_pc::history::History::from_env(),
        services_trace:pil_pc::history::ServiceTrace::default(),
        disks:Default::default(),booted:true,
        fdc:Default::default(),ata:Default::default(),
    };
    setup_boot(&mut p,floppy,fixed,drive)?;
    Ok(p)
}

/// Boot `image` on a plain `Machine<Bios>` (no DOS state).
pub fn boot_bios(image:&[u8],drive:u8)->Result<Machine<Bios>,String>{
    let disk=match drive{
        0|1=>Disk::floppy(image.to_vec())?,
        0x80=>Disk::fixed(image.to_vec())?,
        _=>return Err("boot drive must be 00h, 01h or 80h".into()),
    };
    if drive==0x80{prepare(Bios,DEFAULT_RAM,None,vec![disk],drive)}
    else{prepare(Bios,DEFAULT_RAM,Some((drive,disk)),Vec::new(),drive)}
}

/// The shown 80x25 text page as lines, for a headless run: from the CRTC
/// start address when a program set it (an OS console scrolls that way),
/// else from the BIOS page.
pub fn screen_lines<F>(p:&Machine<F>)->Vec<String>{
    let origin=if p.booted||p.vga.start_override{u32::from(p.vga.start())*2}else{0};
    (0..25u32).map(|row|{
        let line:String=(0..80u32).map(|col|{
            let c=p.ram.load(0xb8000+((origin+(row*80+col)*2)&0x7fff),MemWidth::Byte).unwrap_or(0x20) as u8;
            if (0x20..0x7f).contains(&c){char::from(c)}else{' '}
        }).collect();
        line.trim_end().to_string()
    }).collect()
}
