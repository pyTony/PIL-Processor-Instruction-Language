//! MOV form decoding uses existing shared memory tokens; untouched state is explicit.
use super::*;
fn step(bytes:&[u8],c:&mut Cpu,ram:&mut Ram,mut image:guest::Image)->Result<(),ExecError>{
    let d=image.decode(ram,c.eip).unwrap();if let guest::Operation::Pil{program,dst,src,flow,address}=d.operation{execute_machine(&program,c,dst,src,flow,ram,address,&mut image.segment_state())}else{panic!()}
}
#[test]
fn moffs_operand_and_address_sizes_are_independent_and_preserve_state(){
    for w in [8,16,32]{for addr32 in [false,true]{for store in [false,true]{
        let mut bytes=Vec::new();if w==32{bytes.push(0x66);}if addr32{bytes.push(0x67);}bytes.push(if store{0xa2}else{0xa0}|u8::from(w!=8));
        if addr32{bytes.extend(0x10020u32.to_le_bytes());}else{bytes.extend(0x820u16.to_le_bytes());}
        let (image,_)=guest::Image::load(&bytes).unwrap();let mut ram=Ram::new(0x20000);ram.write_bytes(0x100,&bytes).unwrap();let at=if addr32{0x10120}else{0x920};ram.store(at,MemWidth::Dword,0xfedcba98).unwrap();
        let mut c=Cpu{regs:[0x12345678;8],eflags:0xed7,undefined_flags:u32::MAX,eip:0x100};let mut want=c.clone();let before=ram.clone();
        want.eip+=bytes.len()as u32;if !store{want.regs[0]=(want.regs[0]&!(mask(w)as u32))|(0xfedcba98&mask(w)as u32);}
        step(&bytes,&mut c,&mut ram,image).unwrap();assert_eq!(c,want);
        if store{let mut expected=before;expected.store(at,MemWidth::from_bits(w).unwrap(),0x12345678).unwrap();assert_eq!(ram,expected);}else{assert_eq!(ram,before);}
    }}}
}
#[test]
fn immediate_moves_cover_high_bytes_memory_sib_and_bp_default_ss(){
    for (bytes,expected_reg,at,w,value) in [
        (vec![0xc6,0xc4,0x9a],Some(0x12349a78),0,8,0),
        (vec![0xc7,0xc0,0x34,0x12],Some(0x12341234),0,16,0),
        (vec![0x66,0xc7,0xc0,1,2,3,4],Some(0x04030201),0,32,0),
        (vec![0xc6,0x46,0xfe,0x9a],None,0x9fe,8,0x9a),
        (vec![0x67,0xc7,0x44,0x88,4,0x34,0x12],None,0x924,16,0x1234),
    ]{
        let (image,mut ram)=guest::Image::load(&bytes).unwrap();let mut c=Cpu{regs:[0;8],eflags:0xed7,undefined_flags:u32::MAX,eip:0x100};c.regs[0]=if expected_reg.is_some(){0x12345678}else{0x800};c.regs[1]=8;c.regs[5]=0x800;let before=c.clone();
        step(&bytes,&mut c,&mut ram,image).unwrap();let mut want=before;want.eip+=bytes.len()as u32;if let Some(v)=expected_reg{want.regs[0]=v;}assert_eq!(c,want);if expected_reg.is_none(){assert_eq!(ram.load(at,MemWidth::from_bits(w).unwrap()).unwrap(),value);}
    }
}
#[test]
fn segment_override_and_precise_mov_faults(){
    let bytes=[0x64,0xa0,0x20,0x08];let (image,mut ram)=guest::Image::load(&bytes).unwrap();ram.store(0xb20,MemWidth::Byte,0x9a).unwrap();let mut c=Cpu::default();c.eip=0x100;step(&bytes,&mut c,&mut ram,image).unwrap();assert_eq!(c.regs[0],0x9a);
    for bytes in [vec![0xa1,0xff,0xff],vec![0xa3,0xff,0xff],vec![0xc7,0x06,0xff,0xff,0x34,0x12],vec![0xa2,0,8]]{
        let (image,mut ram)=guest::Image::load(&bytes).unwrap();ram.protect(0x900,2).unwrap();let mut c=Cpu{eip:0x100,..Cpu::default()};let old=c.clone();let before=ram.clone();assert!(step(&bytes,&mut c,&mut ram,image).is_err());assert_eq!(c,old);assert_eq!(ram,before);
    }
}
#[test]
fn immediate_invalid_extensions_and_truncated_fetches_are_rejected(){
    for bytes in [vec![0xc6,0xc8,1],vec![0xc7,0xf8,1,2],vec![0xa1,0],vec![0x67,0xa0,1,2],vec![0xc7,0x06,0,8,1],vec![0xf0,0xa0,0,8]]{let (image,ram)=guest::Image::load(&bytes).unwrap();assert!(image.decode(&ram,0x100).is_err());}
}
