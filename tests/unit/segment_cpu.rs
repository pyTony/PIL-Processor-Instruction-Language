use super::*;
use address::*;
fn flow()->FlowContext {FlowContext{next_eip:0x402,displacement:0,displacement_bits:32,code_limit:0x4ff}}
fn views(w:u8)->(View,View) {(View{register:0,low:0,width:w},View{register:1,low:0,width:w})}
fn program(name:&str,w:u8)->Program {compile(&instantiate(description(name).unwrap(),w),w).unwrap()}
fn cpu()->Cpu {let mut c=Cpu::default();c.eip=0x400;c.eflags=0x203;c.undefined_flags=16;c.regs[0]=0xaabbccdd;c.regs[1]=1;c.regs[3]=0xffff;c.regs[5]=3;c.regs[6]=4;c}
// Shared host/generated fixture source, not shared address calculations.
const FIXTURE:&str=r#"
fn fixture(s:usize)->AddressContext {
 let mut a=AddressContext{effective:EffectiveAddress::Bits32{base:None,index:None,scale:1,displacement:3},override_segment:None,segments:[SegmentCache::real_mode(1);6]};
 a.segments[SegmentRegister::Ss.index()]=SegmentCache::real_mode(2);
 a.segments[SegmentRegister::Fs.index()]=SegmentCache::real_mode(3);
 let mut selected=SegmentRegister::Ds;
 match s {
  1|6=>{a.effective=EffectiveAddress::Bits32{base:Some(Gpr::Ebp),index:None,scale:1,displacement:0};selected=SegmentRegister::Ss;},
  2=>a.override_segment=Some(SegmentRegister::Fs),
  3=>a.effective=EffectiveAddress::Bits32{base:None,index:Some(Gpr::Ebp),scale:1,displacement:0},
  4=>a.effective=EffectiveAddress::Bits16{form:Address16::BxSi,displacement:0},
  7..=10=>{
   a.segments[3].limit=3;a.segments[3].access=SegmentAccess::Data{writable:true,expand_down:true,big:s==10};
   a.effective=EffectiveAddress::Bits32{base:None,index:None,scale:1,displacement:match s{7=>4,8=>3,9=>0xffff,_=>0x10000}};
  },
  11=>a.segments[3].access=SegmentAccess::Data{writable:false,expand_down:false,big:false},
  12|13=>a.segments[3].access=SegmentAccess::Code{readable:s==12},
  14=>a.segments[3].access=SegmentAccess::Unusable,
  15=>a.effective=EffectiveAddress::Bits32{base:None,index:Some(Gpr::Esp),scale:1,displacement:0},
  16=>{a.override_segment=Some(SegmentRegister::Ss);a.segments[2].access=SegmentAccess::Data{writable:false,expand_down:false,big:false};},
  _=>{},
 }
 if s==5||s==6 {a.segments[selected.index()].limit=3;}
 a
}
"#;
// Keep the fixture definitions compiled in the interpreter test as well.
include!("segment_fixture.rs");

#[test]
fn segmented_store_and_load_use_real_mode_base_and_snapshot_registers() {
    let (dst,src)=views(32);let mut c=cpu();let mut ram=Ram::new(64);
    execute_segmented(&program("store_ram",32),&mut c,dst,src,flow(),&mut ram,fixture(0)).unwrap();
    assert_eq!(ram.load(19,MemWidth::Dword),Ok(1));
    let address=AddressContext{effective:EffectiveAddress::Bits32{base:Some(Gpr::Eax),index:None,scale:1,displacement:0},..fixture(0)};
    c.regs[0]=3;
    execute_segmented(&program("load_ram",32),&mut c,dst,src,flow(),&mut ram,address).unwrap();
    assert_eq!(c.regs[0],1);assert_eq!(c.eip,0x402);assert_eq!(c.eflags,0x203);
}
#[test]
fn ss_default_and_override_choose_different_linear_addresses() {
    let (dst,src)=views(8);let mut c=cpu();let mut ram=Ram::new(64);
    execute_segmented(&program("store_ram",8),&mut c,dst,src,flow(),&mut ram,fixture(1)).unwrap();
    execute_segmented(&program("store_ram",8),&mut c,dst,src,flow(),&mut ram,fixture(2)).unwrap();
    assert_eq!(ram.load(35,MemWidth::Byte),Ok(1));assert_eq!(ram.load(51,MemWidth::Byte),Ok(1));assert_eq!(ram.load(19,MemWidth::Byte),Ok(0));
}
#[test]
fn segment_faults_precede_ram_bounds_and_preserve_all_state() {
    let (dst,src)=views(32);
    for (s,error) in [(5,ExecError::GeneralProtection{eip:0x400,error_code:0}),(6,ExecError::StackSegment{eip:0x400,error_code:0})] {
        let mut c=cpu();let before=c.clone();let mut ram=Ram::new(0);let old=ram.clone();
        assert_eq!(execute_segmented(&program("add_ram",32),&mut c,dst,src,flow(),&mut ram,fixture(s)),Err(error));
        assert_eq!(c,before);assert_eq!(ram,old);
    }
}
#[test]
fn byte_operand_uses_16bit_address_and_entire_actual_token_width_is_checked() {
    let (dst,src)=views(8);let mut c=cpu();let mut ram=Ram::new(64);
    execute_segmented(&program("store_ram",8),&mut c,dst,src,flow(),&mut ram,fixture(4)).unwrap();
    assert_eq!(ram.load(19,MemWidth::Byte),Ok(1));
    let p=compile("read_address a\nconst v 32 1\nstore a v\npreserve_flags all\nretire",8).unwrap();
    let before=c.clone();let old=ram.clone();
    assert_eq!(execute_segmented(&p,&mut c,dst,src,flow(),&mut ram,fixture(5)),Err(ExecError::GeneralProtection{eip:0x402,error_code:0}));
    assert_eq!(c,before);assert_eq!(ram,old);
}
#[test]
fn rmw_read_allowed_write_denied_preserves_flags_registers_and_ram() {
    let (dst,src)=views(32);let mut c=cpu();let before=c.clone();let mut ram=Ram::new(64);let old=ram.clone();
    assert_eq!(execute_segmented(&program("add_ram",32),&mut c,dst,src,flow(),&mut ram,fixture(11)),Err(ExecError::GeneralProtection{eip:0x400,error_code:0}));
    assert_eq!(c,before);assert_eq!(ram,old);
}
#[test]
fn later_branch_fault_cancels_translated_store() {
    let p=compile("read_address a\nconst v 32 42\nstore a v\nwrite dst v\nconst target 32 1280\nconst take 1 1\nbranch_if target take\npreserve_flags all\nretire",32).unwrap();
    let (dst,src)=views(32);let mut c=cpu();let before=c.clone();let mut ram=Ram::new(64);let old=ram.clone();
    assert_eq!(execute_segmented(&p,&mut c,dst,src,flow(),&mut ram,fixture(0)),Err(ExecError::GeneralProtection{eip:0x400,error_code:0}));
    assert_eq!(c,before);assert_eq!(ram,old);
}
#[test]
fn generated_segmented_execution_matches_interpreter() {
    use std::{fs,process::Command,time::{SystemTime,UNIX_EPOCH}};
    let dir=std::env::temp_dir().join(format!("pil-segments-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir(&dir).unwrap();
    // Prevent drift between the interpreter fixture and its generated copy.
    assert_eq!(FIXTURE.trim(),include_str!("segment_fixture.rs").trim());
    let mut source=String::new();let mut main=String::from("fn main(){\n");let mut cases=0;
    for op in ["load_ram","store_ram","add_ram"] {for w in [8,16,32] {
        let p=program(op,w);let id=format!("{op}{w}");let (dst,src)=views(w);
        source.push_str(&format!("mod {id} {{ {}\nuse address::*;\n{}\n",emit_rust(&p),FIXTURE));
        source.push_str("pub fn check(s:usize,expected:&str,expected_regs:[u32;8],expected_flags:u32,expected_undefined:u32,expected_ip:u32,expected_ram:&[u8]) {\nlet mut regs=[0xaabbccdd,1,0,0xffff,0,3,4,0];let mut flags=0x203;let mut undefined=16;let mut ip=0x400;let mut ram=Ram::new(64);\nlet context=FlowContext{next_eip:0x402,displacement:0,displacement_bits:32,code_limit:0x4ff};\nlet r=execute_with_segments(&mut regs,&mut flags,&mut undefined,&mut ip,(0,0),(1,0),context,&mut ram,fixture(s));\nassert_eq!(format!(\"{r:?}\"),expected);assert_eq!(regs,expected_regs);assert_eq!(flags,expected_flags);assert_eq!(undefined,expected_undefined);assert_eq!(ip,expected_ip);assert_eq!(ram.bytes(),expected_ram);\n}\n}\n");
        for s in 0..17 {
            let mut c=cpu();let mut ram=Ram::new(64);
            let result=execute_segmented(&p,&mut c,dst,src,flow(),&mut ram,fixture(s));
            main.push_str(&format!("{id}::check({s},{:?},{:?},{},{},{},&{:?});\n",format!("{result:?}"),c.regs,c.eflags,c.undefined_flags,c.eip,ram.bytes()));cases+=1;
        }
    }}
    assert_eq!(cases,153);main.push_str("}\n");source.push_str(&main);
    fs::write(dir.join("generated.rs"),source).unwrap();let bin=dir.join(if cfg!(windows){"segments.exe"}else{"segments"});
    let rustc=std::env::var_os("RUSTC").unwrap_or_else(||"rustc".into());let out=Command::new(rustc).arg("--edition=2021").arg("-O").arg(dir.join("generated.rs")).arg("-o").arg(&bin).output().unwrap();
    assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));let out=Command::new(&bin).output().unwrap();assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));fs::remove_dir_all(dir).unwrap();
}
