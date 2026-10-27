//! Original expected states for FF /6, independent of its PIL description.
use super::*;
use address::{SegmentAccess,SegmentRegister};

fn setup(bytes:&[u8],big:bool)->(guest::Image,Ram,Cpu){
    let(mut image,_)=guest::Image::load(bytes).unwrap();
    image.segments[2].access=SegmentAccess::Data{writable:true,expand_down:false,big};
    let mut ram=Ram::new(0x22000);ram.write_bytes(0x100,bytes).unwrap();
    let cpu=Cpu{regs:[0x12345678,0x87654321,0,0x600,0x800,0x600,8,0],eflags:0xed7,undefined_flags:u32::MAX,eip:0x100};
    (image,ram,cpu)
}
fn step(image:&mut guest::Image,ram:&mut Ram,cpu:&mut Cpu)->Result<(),ExecError>{
    let decoded=image.decode(ram,cpu.eip).unwrap();
    if let guest::Operation::Pil{program,dst,src,flow,address}=decoded.operation{
        let mut machine=image.segment_state();
        let result=execute_machine(&program,cpu,dst,src,flow,ram,address,&mut machine);
        image.set_segment_state(machine);result
    }else{panic!("expected PIL")}
}
#[test]
fn memory_push_operand_address_and_ss_sizes_are_independent(){
    for w in [16,32]{for big in [false,true]{for addr32 in [false,true]{
        let mut bytes=vec![];if w==32{bytes.push(0x66);}if addr32{bytes.push(0x67);}bytes.extend([0xff,0x37]);
        let(mut image,mut ram,mut cpu)=setup(&bytes,big);
        // m16 [BX] versus m32 [EDI]; high ESP survives a 16-bit stack update.
        cpu.regs[7]=0x600;if !big{cpu.regs[4]|=0xabcd0000;}
        ram.store(0x700,MemWidth::Dword,0xdeadbeef).unwrap();
        let mut want_ram=ram.clone();let mut want_cpu=cpu.clone();let old_image=image.clone();
        let n=(w/8)as u32;want_ram.store(0xa00-n,MemWidth::from_bits(w).unwrap(),0xdeadbeef).unwrap();
        want_cpu.regs[4]-=n;want_cpu.eip+=bytes.len()as u32;
        step(&mut image,&mut ram,&mut cpu).unwrap();
        assert_eq!(cpu,want_cpu);assert_eq!(ram,want_ram);assert_eq!(image,old_image);
    }}}
}
#[test]
fn memory_push_esp_sib_reads_before_decrement_and_handles_overlap(){
    for w in [16,32]{for big in [false,true]{for sib in [false,true]{
        let mut bytes=vec![];if w==32{bytes.push(0x66);}bytes.push(0x67);
        bytes.extend(if sib{vec![0xff,0x74,0x74,0xfc]}else{vec![0xff,0x34,0x24]});
        let(mut image,mut ram,mut cpu)=setup(&bytes,big);
        let n=(w/8)as u32;let source=0xa00+if sib{12}else{0};
        ram.store(source,MemWidth::from_bits(w).unwrap(),0xa5b6c7d8).unwrap();
        // A wrong post-decrement EA yields different bytes.
        ram.store(source-n,MemWidth::from_bits(w).unwrap(),0x11223344).unwrap();
        step(&mut image,&mut ram,&mut cpu).unwrap();
        assert_eq!(ram.load(0xa00-n,MemWidth::from_bits(w).unwrap()).unwrap(),0xa5b6c7d8&mask(w)as u32);
        assert_eq!(cpu.regs[4],0x800-n);
    }}}
    let(mut image,mut ram,mut cpu)=setup(&[0xff,0x77,0xff],false);
    cpu.regs[3]=0x8ff; // DS:BX-1 = 0x9fe, exactly the stack destination.
    ram.store(0x9fe,MemWidth::Word,0xbeef).unwrap();
    step(&mut image,&mut ram,&mut cpu).unwrap();assert_eq!(ram.load(0x9fe,MemWidth::Word).unwrap(),0xbeef);
    let(mut image,mut ram,mut cpu)=setup(&[0xff,0x37],false);
    cpu.regs[3]=0x8ff;ram.store(0x9ff,MemWidth::Word,0xbeef).unwrap();
    step(&mut image,&mut ram,&mut cpu).unwrap();assert_eq!(ram.load(0x9fe,MemWidth::Word).unwrap(),0xbeef);
}
#[test]
fn memory_push_segment_overrides_select_source_only_and_allow_readonly_sources(){
    for (prefix,base) in [(None,0x200),(Some(0x3e),0x100),(Some(0x64),0x300),(Some(0x2e),0)]{
        let mut bytes=vec![];if let Some(p)=prefix{bytes.push(p);}bytes.extend([0xff,0x76,0]);
        let(mut image,mut ram,mut cpu)=setup(&bytes,false);
        ram.store(base+0x600,MemWidth::Word,0xbeef).unwrap();ram.protect(base+0x600,2).unwrap();
        let old_cpu=cpu.clone();step(&mut image,&mut ram,&mut cpu).unwrap();
        assert_eq!(ram.load(0x9fe,MemWidth::Word).unwrap(),0xbeef);
        let mut want=old_cpu;want.regs[4]-=2;want.eip+=bytes.len()as u32;assert_eq!(cpu,want);
    }
}
#[test]
fn memory_push_faults_preserve_cpu_ram_and_retirement_markers(){
    for fault in 0..7{
        let bytes=if fault==1{vec![0xff,0x76,0]}else{vec![0xff,0x37]};
        let(mut image,mut ram,mut cpu)=setup(&bytes,false);
        image.ss_shadow=1;image.sti_shadow=1;
        match fault{
            0=>image.segments[3].limit=0x600,
            1=>image.segments[2].limit=0x600,
            2=>image.segments[3].base=0x22000,
            3=>ram.protect(0x9ff,1).unwrap(),
            4=>image.segments[2].limit=0x7fe,
            5=>image.segments[2].base=0x22000,
            _=>image.segments[2].access=SegmentAccess::Data{writable:false,expand_down:false,big:false},
        }
        let old_cpu=cpu.clone();let old_ram=ram.clone();let old_image=image.clone();
        let error=step(&mut image,&mut ram,&mut cpu).unwrap_err();
        match fault{0=>assert!(matches!(error,ExecError::GeneralProtection{..})),1|4=>assert!(matches!(error,ExecError::StackSegment{..})),2=>assert!(matches!(error,ExecError::MemoryAccess{write:false,..})),6=>assert_eq!(error,ExecError::InvalidContext),_=>assert!(matches!(error,ExecError::MemoryAccess{write:true,..}))}
        assert_eq!(cpu,old_cpu);assert_eq!(ram,old_ram);assert_eq!(image,old_image);
    }
}
#[test]
fn memory_push_stack_wrap_and_complete_access_spans_are_checked(){
    let(mut image,mut ram,mut cpu)=setup(&[0xff,0x37],false);
    cpu.regs[4]=0xabcd0000;ram.store(0x700,MemWidth::Word,0xbeef).unwrap();
    step(&mut image,&mut ram,&mut cpu).unwrap();assert_eq!(cpu.regs[4],0xabcdfffe);assert_eq!(ram.load(0x101fe,MemWidth::Word).unwrap(),0xbeef);
    for (w,sp) in [(16,1),(32,2)]{
        let mut bytes=vec![];if w==32{bytes.push(0x66);}bytes.extend([0xff,0x37]);
        let(mut image,mut ram,mut cpu)=setup(&bytes,false);cpu.regs[4]=sp;
        let old=cpu.clone();let old_ram=ram.clone();
        assert!(matches!(step(&mut image,&mut ram,&mut cpu),Err(ExecError::StackSegment{..})));
        assert_eq!(cpu,old);assert_eq!(ram,old_ram);
    }
    // Expand-down SS allows the new top only above its lower bound.
    let(mut image,mut ram,mut cpu)=setup(&[0xff,0x37],false);
    image.segments[SegmentRegister::Ss.index()].access=SegmentAccess::Data{writable:true,expand_down:true,big:false};
    image.segments[2].limit=0x7fd;step(&mut image,&mut ram,&mut cpu).unwrap();
}
#[test]
fn memory_push_encoding_boundaries_and_register_group_remain_distinct(){
    for bytes in [vec![0xff],vec![0xff,0x36,0],vec![0x67,0xff,0x34],vec![0xff,0x3f],vec![0xf0,0xff,0x37],vec![0xf3,0xff,0x37]]{
        let(image,ram,_)=setup(&bytes,false);assert!(image.decode(&ram,0x100).is_err());
    }
    for bytes in [vec![0xff,0xf4],vec![0xff,0x74,0x14]]{
        let(mut image,mut ram,mut cpu)=setup(&bytes,false);
        cpu.regs[6]=0x700;ram.store(0x814,MemWidth::Word,0xbeef).unwrap();
        let decoded=image.decode(&ram,0x100).unwrap();assert!(decoded.label.starts_with("PUSH "));
        let expected=if bytes.len()==2{0x800}else{0xbeef};
        step(&mut image,&mut ram,&mut cpu).unwrap();assert_eq!(ram.load(0x9fe,MemWidth::Word).unwrap(),expected);
    }
}
