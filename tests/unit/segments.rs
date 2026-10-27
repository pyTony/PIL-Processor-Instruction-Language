//! Real-mode selector/cache changes and precise commits, independent expectations.
use super::*;
use address::*;
fn setup(bytes:&[u8])->(guest::Image,Ram,Cpu){let (image,_)=guest::Image::load(bytes).unwrap();let mut ram=Ram::new(0x21000);ram.write_bytes(0x100,bytes).unwrap();let c=Cpu{regs:[0x12345678;8],eflags:0xed7,undefined_flags:u32::MAX,eip:0x100};(image,ram,c)}
fn step(image:&mut guest::Image,ram:&mut Ram,c:&mut Cpu)->Result<(),ExecError>{
    let d=image.decode(ram,c.eip).unwrap();if let guest::Operation::Pil{program,dst,src,flow,address}=d.operation{
        let mut machine=image.segment_state();let result=execute_machine(&program,c,dst,src,flow,ram,address,&mut machine);if result.is_ok(){image.set_segment_state(machine);}result
    }else{panic!("expected PIL")}
}
#[test]
fn mov_register_selectors_preserve_upper_halves_and_rebuild_real_mode_caches(){
    for segment in 0..6{for reg in 0..8{for selector in [0,1,0x1234,0xffff]{
        let bytes=[0x8c,0xc0|(segment<<3)|reg];let (mut image,mut ram,mut c)=setup(&bytes);image.selectors[segment as usize]=selector;
        let mut want=c.clone();want.regs[reg as usize]=0x12340000|u32::from(selector);want.eip+=2;
        let old=image.clone();step(&mut image,&mut ram,&mut c).unwrap();assert_eq!(c,want);assert_eq!(image,old);
        if segment!=1{
            ram.write_bytes(0x100,&[0x8e,0xc0|(segment<<3)|reg]).unwrap();c.eip=0x100;c.regs[reg as usize]=0xabcd0000|u32::from(selector);let old=c.clone();
            step(&mut image,&mut ram,&mut c).unwrap();let mut want=old;want.eip+=2;assert_eq!(c,want);assert_eq!(image.selectors[segment as usize],selector);assert_eq!(image.segments[segment as usize],SegmentCache::real_mode(selector));assert_eq!(image.ss_shadow,u8::from(segment==2));
        }
    }}}
}
#[test]
fn memory_segment_moves_use_old_segment_and_word_access_even_with_66(){
    let bytes=[0x66,0x8c,0x1e,0x00,0x08,0x66,0x8e,0x1e,0x00,0x08];let (mut image,mut ram,mut c)=setup(&bytes);
    image.selectors[3]=0x200;image.segments[3]=SegmentCache::real_mode(0x100);ram.write_bytes(0x1800,&[0xaa,0xbb,0xcc,0xdd]).unwrap();
    step(&mut image,&mut ram,&mut c).unwrap();assert_eq!(&ram.bytes()[0x1800..0x1804],&[0,2,0xcc,0xdd]);
    step(&mut image,&mut ram,&mut c).unwrap();assert_eq!(image.selectors[3],0x200);assert_eq!(image.segments[3],SegmentCache::real_mode(0x200));
}
#[test]
fn segment_pop_uses_old_ss_operand_width_and_commits_pointer_and_cache_together(){
    for big in [false,true]{for w in [16,32]{for (segment,opcode) in [(0,vec![0x07]),(2,vec![0x17]),(3,vec![0x1f]),(4,vec![0x0f,0xa1]),(5,vec![0x0f,0xa9])]{
        let mut bytes=Vec::new();if w==32{bytes.push(0x66);}bytes.extend(opcode);
        let (mut image,mut ram,mut c)=setup(&bytes);image.segments[2]=SegmentCache{base:0x400,limit:0x1ffff,access:SegmentAccess::Data{writable:true,expand_down:false,big}};c.regs[4]=if big{0xfffe}else{0xabcd0800};
        let pointer=c.regs[4];let mask=if big{u32::MAX}else{65535};let at=0x400+(pointer&mask);ram.store(at,MemWidth::from_bits(w).unwrap(),0xabcd0040).unwrap();let old=c.clone();let oldram=ram.clone();
        step(&mut image,&mut ram,&mut c).unwrap();let mut want=old;want.regs[4]=(pointer&!mask)|(pointer.wrapping_add((w/8)as u32)&mask);want.eip=0x100+bytes.len()as u32;assert_eq!(c,want);
        assert_eq!(ram,oldram);assert_eq!(image.selectors[segment],0x40);assert_eq!(image.segments[segment],SegmentCache::real_mode(0x40));assert_eq!(image.ss_shadow,u8::from(segment==2));
    }}}
}
#[test]
fn ss_shadow_survives_fault_and_clears_only_after_successful_retirement(){
    let (mut image,mut ram,mut c)=setup(&[0x8e,0xd0,0x89,0x07,0x90]);c.regs[0]=0x20;c.regs[3]=0x800;
    step(&mut image,&mut ram,&mut c).unwrap();assert_eq!(image.ss_shadow,1);
    ram.protect(0x900,2).unwrap();let old=c.clone();let oldimage=image.clone();assert!(step(&mut image,&mut ram,&mut c).is_err());assert_eq!(c,old);assert_eq!(image,oldimage);
    c.eip=0x104;step(&mut image,&mut ram,&mut c).unwrap();assert_eq!(image.ss_shadow,0);
}
#[test]
fn faulting_segment_memory_and_pop_leave_all_state_unchanged(){
    for bytes in [vec![0x8e,0x1e,0xff,0xff],vec![0x8c,0x1e,0xff,0xff],vec![0x17]]{
        let (mut image,mut ram,mut c)=setup(&bytes);c.regs[4]=0xffff;image.ss_shadow=1;let old=c.clone();let oldimage=image.clone();let oldram=ram.clone();
        assert!(step(&mut image,&mut ram,&mut c).is_err());assert_eq!(c,old);assert_eq!(image,oldimage);assert_eq!(ram,oldram);
    }
    let (mut image,mut ram,mut c)=setup(&[0x8c,0x1e,0,8]);ram.protect(0x900,2).unwrap();let old=c.clone();let oldram=ram.clone();assert!(step(&mut image,&mut ram,&mut c).is_err());assert_eq!(c,old);assert_eq!(ram,oldram);
}
#[test]
fn invalid_encodings_modes_and_missing_machine_context_are_explicit(){
    for bytes in [vec![0x8e,0xc8],vec![0x8e,0xf0],vec![0x8c,0xf8],vec![0x66,0x8c,0xc0],vec![0xf0,0x8e,0xd8]]{let (image,ram,_)=setup(&bytes);assert!(image.decode(&ram,0x100).is_err());}
    let (mut image,ram,_)=setup(&[0x8e,0xd8]);image.real_mode=false;assert!(image.decode(&ram,0x100).is_err());
    for text in ["const x 16 0\nwrite_segment 1 x\npreserve_flags all\nretire","read_segment x 6\npreserve_flags all\nretire"]{assert!(compile(text,16).is_err());}
    let p=compile("read_segment x 3\nwrite dst x\npreserve_flags all\nretire",16).unwrap();let v=View{register:0,low:0,width:16};let mut c=Cpu::default();assert!(execute(&p,&mut c,v,v).is_err());
}
#[test]
fn generated_segment_writes_have_independent_full_state_expectations(){
    use std::{fs,process::Command,time::{SystemTime,UNIX_EPOCH}};
    let dir=std::env::temp_dir().join(format!("pil-segments-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir_all(&dir).unwrap();
    let text="read selector src 16\nwrite_segment 2 selector\npreserve_flags all\nretire";let p=compile(text,16).unwrap();let mut source=emit_rust(&p);source.push_str(r#"
fn main(){for selector in [0,1,0x1234,0xffff]{
 let mut regs=[0xabcdef01;8];regs[0]=0xdead0000|selector;let before=regs;let mut flags=0xed7;let mut undefined=u32::MAX;let mut ip=0x100;let mut ram=Ram::new(4096);
 let mut machine=address::SegmentState{segments:[address::SegmentCache::real_mode(0);6],selectors:[0;6],msw:0x10,real_mode:true,ss_shadow:0,sti_shadow:0,flag_checkpoint:None};
 let flow=FlowContext{next_eip:0x102,displacement:0,displacement_bits:32,code_limit:65535};
 execute_with_machine(&mut regs,&mut flags,&mut undefined,&mut ip,(0,0),(0,0),flow,&mut ram,None,&mut machine).unwrap();
 assert_eq!(regs,before);assert_eq!((flags,undefined,ip),(0xed7,u32::MAX,0x102));assert_eq!(machine.selectors[2],selector as u16);assert_eq!(machine.segments[2],address::SegmentCache::real_mode(selector as u16));assert_eq!(machine.ss_shadow,1);
 let old=machine;machine.real_mode=false;ip=0x100;assert!(execute_with_machine(&mut regs,&mut flags,&mut undefined,&mut ip,(0,0),(0,0),flow,&mut ram,None,&mut machine).is_err());assert_eq!(ip,0x100);assert_eq!(machine.segments,old.segments);
}}
"#);let path=dir.join("proof.rs");fs::write(&path,source).unwrap();let bin=dir.join(if cfg!(windows){"proof.exe"}else{"proof"});let out=Command::new(std::env::var_os("RUSTC").unwrap_or_else(||"rustc".into())).args(["--edition=2021","-O"]).arg(path).arg("-o").arg(&bin).output().unwrap();assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));assert!(Command::new(bin).status().unwrap().success());fs::remove_dir_all(dir).unwrap();
}
#[test]
fn machine_trace_compares_selectors_cache_and_shadow(){
    let (image,_,c)=setup(&[0x90]);let m=image.segment_state();let mut before=Vec::new();trace::record_machine(&mut before,0,[0;5],&c.regs,c.eflags,c.undefined_flags,c.eip,&[],&[],&[],&m);
    for kind in 0..3{let mut after=Vec::new();let mut n=m;match kind{0=>n.selectors[3]^=1,1=>n.segments[3].base^=16,_=>n.ss_shadow=1};trace::record_machine(&mut after,0,[0;5],&c.regs,c.eflags,c.undefined_flags,c.eip,&[],&[],&[],&n);assert!(native::verify_traces(&before,&after).unwrap_err().contains("segments"));}
}
