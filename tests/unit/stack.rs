//! Independent stack values, alias ordering, size separation and precise faults.
use super::*;
use address::{SegmentAccess,SegmentRegister};
fn step(image:&guest::Image,ram:&mut Ram,c:&mut Cpu)->Result<(),ExecError>{
    let d=image.decode(ram,c.eip).unwrap();
    if let guest::Operation::Pil{program,dst,src,flow,address:Some(a)}=d.operation{
        execute_segmented(&program,c,dst,src,flow,ram,a)
    }else{panic!("expected stack PIL")}
}
fn setup(bytes:&[u8],big:bool)->(guest::Image,Ram,Cpu){
    let (mut image,_)=guest::Image::load(bytes).unwrap();
    image.segments[2]=address::SegmentCache{base:0x200,limit:if big{u32::MAX}else{65535},access:SegmentAccess::Data{writable:true,expand_down:false,big}};
    let mut ram=Ram::new(0x20200);ram.write_bytes(0x100,bytes).unwrap();
    let c=Cpu{regs:[0x12345678,0x87654321,0,0,if big{0x800}else{0xabcd0800},0,0,0],eflags:0xed7,undefined_flags:u32::MAX,eip:0x100};
    (image,ram,c)
}
#[test]
fn push_pop_all_registers_sizes_and_sp_snapshot(){
    for w in [16,32]{for big in [false,true]{for reg in 0..8{
        let mut bytes=Vec::new();if w==32{bytes.push(0x66);}bytes.push(0x50+reg);
        if w==32{bytes.push(0x66);}bytes.push(0x58+reg);
        let (image,mut ram,mut c)=setup(&bytes,big);let before=c.clone();
        let mask=if big{u32::MAX}else{65535};let n=(w/8)as u32;
        let off=c.regs[4].wrapping_sub(n)&mask;
        let expected=c.regs[reg as usize] & super::mask(w)as u32;
        step(&image,&mut ram,&mut c).unwrap();
        assert_eq!(ram.load(0x200+off,MemWidth::from_bits(w).unwrap()).unwrap(),expected);
        let mut want=before.clone();want.regs[4]=(before.regs[4]&!mask)|off;want.eip=c.eip;
        assert_eq!(c,want);
        step(&image,&mut ram,&mut c).unwrap();
        let mut want=before;want.eip=0x100+bytes.len()as u32;assert_eq!(c,want,"w={w} big={big} reg={reg}");
    }}}
}
#[test]
fn pop_sp_uses_incremented_high_half_before_destination_overwrite(){
    for (w,big,esp,val,want) in [(16,true,0xfffe,0x1234,0x11234),(32,false,0xabcd0800,0x12345678,0x12345678)]{
        let bytes=if w==32{vec![0x66,0x5c]}else{vec![0x5c]};
        let (image,mut ram,mut c)=setup(&bytes,big);c.regs[4]=esp;
        ram.store(0x200+(esp&if big{u32::MAX}else{65535}),MemWidth::from_bits(w).unwrap(),val).unwrap();
        step(&image,&mut ram,&mut c).unwrap();assert_eq!(c.regs[4],want);
    }
}
#[test]
fn immediate_sign_extension_segment_selectors_and_prefix_independence(){
    for (bytes,w,value) in [(vec![0x6a,0x80],16,0xff80),(vec![0x66,0x6a,0xff],32,u32::MAX),(vec![0x68,0x34,0x12],16,0x1234),(vec![0x67,0x3e,0x1e],16,0xbeef)]{
        let (mut image,mut ram,mut c)=setup(&bytes,false);image.selectors[3]=0xbeef;
        step(&image,&mut ram,&mut c).unwrap();assert_eq!(ram.load(0x200+0x800-(w/8)as u32,MemWidth::from_bits(w).unwrap()).unwrap(),value);
    }
}
#[test]
fn stack_faults_leave_cpu_and_ram_unchanged_and_distinguish_ss_from_ram(){
    for bytes in [vec![0x50],vec![0x58],vec![0x66,0x50],vec![0x66,0x58]]{
        for fault in 0..3{
            let (mut image,mut ram,mut c)=setup(&bytes,false);
            if fault==0{image.segments[2].limit=0x700;}
            if fault==1{image.segments[2].base=0x30000;}
            if fault==2{c.regs[4]=if bytes.last()==Some(&0x50){1}else{0xffff};}
            let old=c.clone();let oldram=ram.clone();let error=step(&image,&mut ram,&mut c).unwrap_err();
            assert!(if fault==1{matches!(error,ExecError::MemoryAccess{..})}else{matches!(error,ExecError::StackSegment{..})});
            assert_eq!(c,old);assert_eq!(ram,oldram);
        }
    }
    let (image,mut ram,mut c)=setup(&[0x50],false);ram.protect(0x9fe,2).unwrap();
    let old=c.clone();let oldram=ram.clone();assert!(matches!(step(&image,&mut ram,&mut c),Err(ExecError::MemoryAccess{write:true,..})));assert_eq!(c,old);assert_eq!(ram,oldram);
}
#[test]
fn named_register_effects_read_snapshot_commit_in_order_and_reject_bound_mix(){
    let p=compile("read_gpr original 4 32\nconst low 16 4660\nwrite_gpr 4 low\nwrite_gpr 0 original\npreserve_flags all\nretire",16).unwrap();
    let mut c=Cpu::default();c.regs[4]=0xabcdef01;let v=View{register:0,low:0,width:16};execute(&p,&mut c,v,v).unwrap();
    assert_eq!(c.regs[0],0xabcdef01);assert_eq!(c.regs[4],0xabcd1234);
    assert!(compile("const x 16 1\nwrite_gpr 4 x\nwrite dst x\npreserve_flags all\nretire",16).is_err());
    assert!(compile("read_gpr x 8 16\npreserve_flags all\nretire",16).is_err());
}
#[test]
fn emitted_stack_aliases_match_independent_expected_state(){
    use std::{fs,process::Command,time::{SystemTime,UNIX_EPOCH}};
    let dir=std::env::temp_dir().join(format!("pil-stack-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir_all(&dir).unwrap();
    let mut source=String::from("#![allow(dead_code,unused_mut,unused_variables)]\n");
    let mut checks=String::new();
    for w in [16,32]{for stack in [16,32]{for pop in [false,true]{
        let name=format!("s{stack}_w{w}_{}",if pop{"pop"}else{"push"});
        let text=description(if pop{"pop"}else{"push"}).unwrap().replace("$S",&stack.to_string()).replace("$BYTES",&(w/8).to_string()).replace("$REG","4");
        let p=compile(&instantiate(text,w),w).unwrap();source.push_str(&format!("mod {name} {{ {} }}\n",emit_rust(&p)));
        let old:u32=if stack==16{0xabcd0800}else{0xfffe};let sm=mask(stack)as u32;let wm=mask(w)as u32;let n=(w/8)as u32;
        let off=if pop{old&sm}else{old.wrapping_sub(n)&sm};let next=(old&!sm)|(if pop{old.wrapping_add(n)}else{old.wrapping_sub(n)}&sm);
        let want=if pop{(next&!wm)|(0x12345678&wm)}else{next};
        checks.push_str(&format!("{{let mut ram={name}::Ram::new(0x20000);let mut regs=[0xdeadbeef;8];regs[4]={old};let mut flags=0xed7;let mut u=u32::MAX;let mut ip=0x100;"));
        if pop{checks.push_str(&format!("ram.store({off},{name}::MemWidth::{},0x12345678).unwrap();",if w==16{"Word"}else{"Dword"}));}
        checks.push_str(&format!("{name}::execute_with_memory(&mut regs,&mut flags,&mut u,&mut ip,(4,0),(0,0),{name}::FlowContext{{next_eip:0x101,displacement:0,displacement_bits:8,code_limit:0xffff}},&mut ram,0).unwrap();assert_eq!(regs[4],{want}u32);assert_eq!((flags,u,ip),(0xed7,u32::MAX,0x101));for i in [0,1,2,3,5,6,7]{{assert_eq!(regs[i],0xdeadbeef);}}"));
        if !pop{checks.push_str(&format!("assert_eq!(ram.load({off},{name}::MemWidth::{}).unwrap(),{}u32);",if w==16{"Word"}else{"Dword"},old&wm));}
        checks.push_str("}\n");
    }}}
    source.push_str(&format!("fn main(){{{checks}}}"));let path=dir.join("proof.rs");fs::write(&path,source).unwrap();let bin=dir.join(if cfg!(windows){"proof.exe"}else{"proof"});
    let out=Command::new(std::env::var_os("RUSTC").unwrap_or_else(||"rustc".into())).args(["--edition=2021","-O"]).arg(path).arg("-o").arg(&bin).output().unwrap();assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));assert!(Command::new(bin).status().unwrap().success());fs::remove_dir_all(dir).unwrap();
}
#[test]
fn stack_cache_identity_includes_ss_size_and_visible_selectors(){
    let (mut image,mut ram,c)=setup(&[0x1e],false);let mut cache=cache::DecodeCache::default();
    cache.fetch(&image,&ram,c.eip).unwrap();image.selectors[3]^=1;cache.fetch(&image,&ram,c.eip).unwrap();
    image.segments[2].access=SegmentAccess::Data{writable:true,expand_down:false,big:true};cache.fetch(&image,&ram,c.eip).unwrap();assert_eq!(cache.stats().invalidations,2);
    ram.write_bytes(0x100,&[0x66]).unwrap();let old=c.clone();let c=c;assert!(image.decode(&ram,c.eip).is_err());assert_eq!(c,old);
}

#[test]
fn alternate_register_encodings_and_fs_gs_push_use_same_pil(){
    for wide in [false,true]{for (instruction,value) in [(vec![0xff,0xf0],0x12345678u32),(vec![0x0f,0xa0],0xbeef),(vec![0x0f,0xa8],0xcafe)]{
        let mut bytes=Vec::new();if wide{bytes.push(0x66);}bytes.extend(instruction);if wide{bytes.push(0x66);}bytes.extend([0x8f,0xc3]);
        let (mut image,mut ram,mut c)=setup(&bytes,false);image.selectors[4]=0xbeef;image.selectors[5]=0xcafe;let esp=c.regs[4];
        step(&image,&mut ram,&mut c).unwrap();step(&image,&mut ram,&mut c).unwrap();assert_eq!(c.regs[3],value&if wide{u32::MAX}else{65535});assert_eq!(c.regs[4],esp);
    }}
    for bytes in [vec![0x8f,0xc8],vec![0xff,0x37],vec![0x8f,0x07],vec![0xf0,0x50],vec![0x66,0x66,0x50]]{let (image,ram,_)=setup(&bytes,false);assert!(image.decode(&ram,0x100).is_err());}
}
