//! Independent expected-state tests for the bounded real-mode control profile.
use super::*;
fn setup(code:&[u8])->(guest::Image,Ram,Cpu){
    let(i,r)=guest::Image::load(code).unwrap();
    let c=Cpu{regs:[0xabcdef01,0x12345678,0x87654321,0x76543210,0x800,0x900,0x1000,0x2000],eflags:0xad7,undefined_flags:0x10000,eip:0x100};(i,r,c)
}
fn step(i:&mut guest::Image,r:&mut Ram,c:&mut Cpu)->Result<(),ExecError>{
    let d=i.decode(r,c.eip).unwrap();if let guest::Operation::Pil{program,dst,src,flow,address}=d.operation{
        let mut m=i.segment_state();execute_machine(&program,c,dst,src,flow,r,address,&mut m)?;i.set_segment_state(m);Ok(())
    }else{panic!()}
}
#[test]
fn control_moves_transfer_all_32_bits_for_every_gpr_and_ignored_size_prefix(){
    for cr in [0,2,3]{for reg in 0..8{for prefix in [vec![],vec![0x66],vec![0x67],vec![0x66,0x67]]{
        let mut code=prefix.clone();code.extend([0x0f,0x22,0xc0|(cr<<3)|reg]);
        let(mut i,mut r,mut c)=setup(&code);let value=match cr{0=>0x1e,2=>0xabcdef01,_=>0xfedcb000};c.regs[reg as usize]=value;
        let old=c.clone();let oldr=r.clone();let mut wanti=i.clone();
        if cr==0{wanti.msw=value as u16;}else{wanti.controls[if cr==2{0}else{1}]=value;}
        step(&mut i,&mut r,&mut c).unwrap();let mut want=old;want.eip+=code.len()as u32;want.undefined_flags|=ARITH_FLAGS;want.eflags&=!ARITH_FLAGS;
        assert_eq!(c,want);assert_eq!(r,oldr);assert_eq!(i,wanti);
        let mut code=prefix;code.extend([0x0f,0x20,0xc0|(cr<<3)|reg]);r.write_bytes(0x100,&code).unwrap();c.eip=0x100;c.regs[reg as usize]=0x5555aaaa;
        let old=c.clone();step(&mut i,&mut r,&mut c).unwrap();let mut want=old;want.eip+=code.len()as u32;want.regs[reg as usize]=value;
        assert_eq!(c,want);assert_eq!(i,wanti);
    }}}
}
#[test]
fn control_rejections_do_not_commit_cpu_machine_ram_or_undefined_flags(){
    for (cr,value,gp) in [(0,0x80000011,false),(0,0x80000010,true),(0,0x20,false),(3,0x12345001,false)]{
        let(mut i,mut r,mut c)=setup(&[0x0f,0x22,0xc0|(cr<<3)]);c.regs[0]=value;i.ss_shadow=1;i.sti_shadow=1;
        let old=c.clone();let oldi=i.clone();let oldr=r.clone();let err=step(&mut i,&mut r,&mut c).unwrap_err();
        assert_eq!(err,if gp{ExecError::GeneralProtection{eip:0x100,error_code:0}}else{ExecError::UnsupportedControl{eip:0x100,register:cr,value}});
        assert_eq!(c,old);assert_eq!(i,oldi);assert_eq!(r,oldr);
    }
    for opcode in [0x20,0x22]{let(mut i,mut r,mut c)=setup(&[0x0f,opcode,0xd0]);c.eflags|=0x20000;let old=c.clone();let oldi=i.clone();assert_eq!(step(&mut i,&mut r,&mut c),Err(ExecError::GeneralProtection{eip:0x100,error_code:0}));assert_eq!(c,old);assert_eq!(i,oldi);}
}
#[test]
fn control_decoder_rejects_reserved_registers_memory_lock_rep_and_truncation(){
    for opcode in [0x20,0x22]{for m in [0xc8,0xe0,0xe8,0xf0,0xf8,0x18,0x5c,0x9c]{let(i,r,_)=setup(&[0x0f,opcode,m]);assert!(i.decode(&r,0x100).is_err());}
        for code in [vec![0x0f,opcode],vec![0xf0,0x0f,opcode,0xd8],vec![0xf3,0x0f,opcode,0xd8]]{let(i,r,_)=setup(&code);assert!(i.decode(&r,0x100).is_err());}
    }
    for text in ["read_control v 1\npreserve_flags all\nretire","const v 16 0\nwrite_control 3 v\npreserve_flags all\nretire","const v 32 0\nwrite_control 3 v\nwrite_control 2 v\npreserve_flags all\nretire"]{assert!(compile(text,32).is_err());}
}
#[test]
fn cr0_writes_are_coherent_with_smsw_and_absent_fpu_traps(){
    let(mut i,mut r,mut c)=setup(&[0x0f,0x22,0xc0,0x0f,1,0xe1]);c.regs[0]=0x1c;
    step(&mut i,&mut r,&mut c).unwrap();step(&mut i,&mut r,&mut c).unwrap();assert_eq!(c.regs[1]&65535,0x1c);
    let instruction=fpu::decode(|n|Ok::<_,()>([0xd9,0xe8][n as usize])).unwrap().unwrap();
    assert_eq!(fpu::AbsentFpu.execute(&instruction,&mut c,&mut r,&i),fpu::Outcome::NotAvailable);
}
#[test]
fn staged_control_write_is_atomic_if_a_later_pil_requirement_fails(){
    let p=compile("const v 32 8192\nwrite_control 3 v\nconst no 1 0\nrequire no\npreserve_flags all\nretire",32).unwrap();
    let(i,mut r,mut c)=setup(&[0x90]);let mut m=i.segment_state();let old=m;let oldc=c.clone();
    assert_eq!(execute_machine(&p,&mut c,View{register:0,low:0,width:32},View{register:0,low:0,width:32},FlowContext{next_eip:0x101,displacement:0,displacement_bits:32,code_limit:65535},&mut r,None,&mut m),Err(ExecError::InvalidContext));assert_eq!(m,old);assert_eq!(c,oldc);
}
#[test]
fn emitted_control_moves_share_state_and_reject_mode_change_atomically(){
    use std::{fs,process::Command,time::{SystemTime,UNIX_EPOCH}};
    let dir=std::env::temp_dir().join(format!("pil-controls-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir_all(&dir).unwrap();
    let mut source=String::new();
    for (name,text) in [("read",include_str!("../../isa/i386/read_control.pil").replace("$CR","3")),("write",include_str!("../../isa/i386/write_control.pil").replace("$CR","3")),("cr0",include_str!("../../isa/i386/write_control.pil").replace("$CR","0"))]{source.push_str(&format!("mod {name}{{{} }}\n",emit_rust(&compile(&text,32).unwrap())));}
    // Emitters include their own machine types, so each module has its own setup.
    source.push_str(r#"
fn main(){
 for value in [0,0x1000,0xfffff000]{
  let mut regs=[0x12345678;8];regs[6]=value;let before=regs;let mut flags=0xad7;let mut undefined=0;let mut ip=0x100;let mut ram=write::Ram::new(4096);
  let mut machine=write::address::SegmentState{segments:[write::address::SegmentCache::real_mode(0);6],selectors:[0;6],msw:16,real_mode:true,ss_shadow:0,sti_shadow:0,tables:[(0,0);2],controls:[0;2],flag_checkpoint:None,ldt_tr:Default::default()};
  let flow=write::FlowContext{next_eip:0x103,displacement:0,displacement_bits:32,code_limit:65535};
  write::execute_with_machine(&mut regs,&mut flags,&mut undefined,&mut ip,(6,0),(6,0),flow,&mut ram,None,&mut machine).unwrap();
  assert_eq!(regs,before);assert_eq!((flags,undefined,ip),(0x202,0x8d5,0x103));assert_eq!(machine.controls,[0,value]);
 }
 let mut regs=[0x80000011;8];let before=regs;let mut flags=0xad7;let mut undefined=0x10000;let mut ip=0x100;let mut ram=cr0::Ram::new(4096);
 let mut machine=cr0::address::SegmentState{segments:[cr0::address::SegmentCache::real_mode(0);6],selectors:[0;6],msw:16,real_mode:true,ss_shadow:1,sti_shadow:1,tables:[(0,0);2],controls:[0,0x1000],flag_checkpoint:None,ldt_tr:Default::default()};let old=machine;
 let flow=cr0::FlowContext{next_eip:0x103,displacement:0,displacement_bits:32,code_limit:65535};
 assert!(matches!(cr0::execute_with_machine(&mut regs,&mut flags,&mut undefined,&mut ip,(0,0),(0,0),flow,&mut ram,None,&mut machine),Err(cr0::ExecError::UnsupportedControl{eip:0x100,register:0,value:0x80000011})));
 assert_eq!(regs,before);assert_eq!((flags,undefined,ip),(0xad7,0x10000,0x100));assert_eq!(machine,old);
 let mut regs=[0;8];let mut flags=0xad7;let mut undefined=0;let mut ip=0x100;let mut ram=read::Ram::new(4096);
 let mut machine=read::address::SegmentState{segments:[read::address::SegmentCache::real_mode(0);6],selectors:[0;6],msw:16,real_mode:true,ss_shadow:0,sti_shadow:0,tables:[(0,0);2],controls:[0,0xdeadb000],flag_checkpoint:None,ldt_tr:Default::default()};
 let flow=read::FlowContext{next_eip:0x103,displacement:0,displacement_bits:32,code_limit:65535};
 read::execute_with_machine(&mut regs,&mut flags,&mut undefined,&mut ip,(7,0),(7,0),flow,&mut ram,None,&mut machine).unwrap();assert_eq!(regs[7],0xdeadb000);assert_eq!(flags,0x202);assert_eq!(undefined,0x8d5);
}
"#);
    fs::write(dir.join("generated.rs"),source).unwrap();let bin=dir.join(if cfg!(windows){"control.exe"}else{"control"});
    let out=Command::new(std::env::var_os("RUSTC").unwrap_or_else(||"rustc".into())).args(["--edition=2021","-O"]).arg(dir.join("generated.rs")).arg("-o").arg(&bin).output().unwrap();assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));
    let out=Command::new(&bin).output().unwrap();assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));fs::remove_dir_all(dir).unwrap();
}
