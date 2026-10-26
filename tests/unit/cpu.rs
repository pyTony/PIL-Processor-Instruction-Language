use super::*;
fn run(p: &Program, a:u32, b:u32, f:u32) -> Cpu {
    let mut c = Cpu{regs:[0;8],eflags:f,undefined_flags:0,eip:0}; c.regs[0]=a; c.regs[1]=b;
    let w=p.operand_width;
    execute(p,&mut c,View{register:0,low:0,width:w},View{register:1,low:0,width:w}).unwrap(); c
}
fn expected_flags(a:u32,b:u32,w:u8) -> u32 {
    let m=mask(w); let a=(a as u64)&m; let b=(b as u64)&m; let r=(a+b)&m;
    let sign=1u64<<(w-1);
    let signed = |x:u64| -> i64 { if x&sign!=0 { x as i64 - (1i64<<w) } else { x as i64 } };
    let sum=signed(a)+signed(b); let min=-(1i64<<(w-1)); let max=(1i64<<(w-1))-1;
    u32::from(a+b>m)
        | (u32::from((r as u8).count_ones()%2==0)<<2)
        | (u32::from((a&15)+(b&15)>15)<<4)
        | (u32::from(r==0)<<6)
        | (u32::from(r&sign!=0)<<7)
        | (u32::from(sum<min || sum>max)<<11)
}
#[test]
fn exhaustive_add8() {
    let p=compile(&instantiate(ADD,8),8).unwrap();
    for a in 0..256 { for b in 0..256 {
        let c=run(&p,a,b,0x202);
        assert_eq!(c.regs[0],(a+b)&255);
        assert_eq!(c.eflags,0x202 | expected_flags(a,b,8));
    }}
}
#[test]
fn add16_and_32_edges() {
    for w in [16,32] {
        let p=compile(&instantiate(ADD,w),w).unwrap();
        let m=mask(w) as u32; let sign=1u32<<(w-1);
        for a in [0,1,15,16,sign-1,sign,m] { for b in [0,1,15,16,sign-1,sign,m] {
            let c=run(&p,a,b,0x202);
            assert_eq!(c.regs[0],(a.wrapping_add(b))&m);
            assert_eq!(c.eflags,0x202 | expected_flags(a,b,w));
        }}
    }
}
#[test]
fn merged_ax_and_ah_views() {
    let p=compile(&instantiate(ADD,16),16).unwrap();
    assert_eq!(run(&p,0xabcd_ffff,1,0).regs[0],0xabcd_0000);
    let p=compile(&instantiate(ADD,8),8).unwrap();
    let mut c=Cpu::default(); c.regs[0]=0x1234_ff56; c.regs[1]=1;
    execute(&p,&mut c,View{register:0,low:8,width:8},View{register:1,low:0,width:8}).unwrap();
    assert_eq!(c.regs[0],0x1234_0056);
}
#[test]
fn flag_preservation_is_description_controlled() {
    let description=ADD.replace("read b src $W","const b $W 1").replace("flag CF cf","preserve_flag CF");
    for w in [8,16,32] {
        let p=compile(&instantiate(&description,w),w).unwrap();
        for cf in [0,1] {
            let c=run(&p,mask(w) as u32,0,0x202|cf);
            assert_eq!(c.regs[0],0);
            assert_eq!(c.eflags&1,cf);
            assert_eq!(c.eflags&!1,(0x202|expected_flags(mask(w) as u32,1,w))&!1);
        }
    }
}
#[test]
fn rejects_invalid_descriptions() {
    for s in [
        "read a dst 8\nread b src 16\nretire",
        "const a 8 1\nconst b 16 1\nadd c a b\nretire",
        "const a 8 256\nretire",
        "read a dst 8\nflag CF a\nretire",
        "read_flag c INVALID\nretire",
        "read_flag c CF\nread a dst 8\nsub r a c\nretire",
        "read a dst 8\nbit b a 8\nretire",
        "write dst missing\nretire",
        "read a dst 8\nread a src 8\nretire",
        "retire\nretire",
        "read a dst 8",
    ] { assert!(compile(s,8).is_err(),"{s}"); }
}
#[test]
fn invalid_binding_preserves_cpu() {
    let p=compile(&instantiate(ADD,8),8).unwrap();
    let mut c=Cpu::default(); let before=c.clone();
    assert!(execute(&p,&mut c,View{register:0,low:0,width:16},View{register:1,low:0,width:8}).is_err());
    assert_eq!(c,before);
}
// Independent arithmetic oracle: signed range checks and nibble arithmetic,
// rather than the bitwise flag formulae in the PIL descriptions.
fn expected_arithmetic(op: &str, a:u32, b:u32, w:u8, initial:u32) -> (u32,u32) {
    let m=mask(w); let a=(a as u64)&m; let b=(b as u64)&m;
    let carry=if op=="adc" || op=="sbb" { (initial&1) as u64 } else { 0 };
    let subtract=op=="sub" || op=="cmp" || op=="sbb";
    let full=if subtract { a as i64-b as i64-carry as i64 }
        else { a as i64+b as i64+carry as i64 };
    let r=(full as u64)&m;
    let sign=1u64<<(w-1);
    let signed=|x:u64| if x&sign!=0 { x as i64-(1i64<<w) } else { x as i64 };
    let signed_full=if subtract { signed(a)-signed(b)-carry as i64 }
        else { signed(a)+signed(b)+carry as i64 };
    let overflow=signed_full < -(1i64<<(w-1)) || signed_full > (1i64<<(w-1))-1;
    let cf=if subtract { a < b+carry } else { a+b+carry > m };
    let af=if subtract { (a&15) < (b&15)+carry } else { (a&15)+(b&15)+carry > 15 };
    let flags=u32::from(cf) | (u32::from((r as u8).count_ones()%2==0)<<2)
        | (u32::from(af)<<4) | (u32::from(r==0)<<6)
        | (u32::from(r&sign!=0)<<7) | (u32::from(overflow)<<11);
    (r as u32,(initial & !0x8d5) | flags)
}
fn check_arithmetic(op:&str,w:u8,a:u32,b:u32,initial:u32) {
    let p=compile(&instantiate(description(op).unwrap(),w),w).unwrap();
    check_program(&p,op,w,a,b,initial);
}
fn check_program(p:&Program,op:&str,w:u8,a:u32,b:u32,initial:u32) {
    let c=run(p,a,b,initial);
    let (r,f)=expected_arithmetic(op,a,b,w,initial);
    let expected_dst=if op=="cmp" { a } else { (a & !(mask(w) as u32)) | r };
    assert_eq!(c.regs[0],expected_dst,"{op}/{w} a={a:x} b={b:x} f={initial:x}");
    assert_eq!(c.regs[1],b,"source changed");
    assert_eq!(c.eflags,f,"{op}/{w} a={a:x} b={b:x} f={initial:x}");
}
fn exhaustive8(op:&str) {
    let p=compile(&instantiate(description(op).unwrap(),8),8).unwrap();
    for carry in 0..=1 {
        for a in 0..256 { for b in 0..256 {
            check_program(&p,op,8,a,b,0x202|carry);
        }}
    }
}
#[test] fn exhaustive_sub8() { exhaustive8("sub"); }
#[test] fn exhaustive_cmp8_preserves_destination() { exhaustive8("cmp"); }
#[test] fn exhaustive_adc8_both_carries() { exhaustive8("adc"); }
#[test] fn exhaustive_sbb8_both_borrows() { exhaustive8("sbb"); }
#[test]
fn arithmetic16_32_edges_and_preserved_flags() {
    for op in ["add","sub","cmp","adc","sbb"] {
        for w in [16,32] {
            let m=mask(w) as u32; let sign=1u32<<(w-1);
            for a in [0,1,15,16,sign-1,sign,m] {
                for b in [0,1,15,16,sign-1,sign,m] {
                    for flags in [0,1,0x202,0x203,0xffff_ffff] {
                        check_arithmetic(op,w,a,b,flags);
                    }
                }
            }
        }
    }
}
#[test]
fn incoming_flags_and_registers_are_snapshots_until_retirement() {
    // Staging CF=0 must not change the value read by read_flag.
    let p=compile("const z 1 0\nflag CF z\nread_flag incoming CF\nzext r incoming 8\nwrite dst r\npreserve_flags remaining\nretire",8).unwrap();
    assert_eq!(run(&p,0,0,1).regs[0],1);
    // Read the old destination even after staging its replacement.
    let p=compile("const replacement 8 42\nwrite dst replacement\nread old dst 8\nbit old_cf old 0\nflag CF old_cf\npreserve_flags remaining\nretire",8).unwrap();
    let c=run(&p,1,0,0); assert_eq!(c.regs[0],42); assert_eq!(c.eflags&1,1);
    // All reads precede retirement even when dst and src alias.
    for op in ["add","sub","cmp","adc","sbb"] {
        let p=compile(&instantiate(description(op).unwrap(),8),8).unwrap();
        let mut c=Cpu::default(); c.regs[0]=0x1234_80ff; c.eflags=0x203;
        execute(&p,&mut c,View{register:0,low:0,width:8},View{register:0,low:0,width:8}).unwrap();
        let (r,f)=expected_arithmetic(op,255,255,8,0x203);
        assert_eq!(c.regs[0],if op=="cmp" {0x1234_80ff} else {0x1234_8000|r});
        assert_eq!(c.eflags,f);
    }
}
#[test]
fn carry_and_borrow_chain_across_registers() {
    for op in ["adc","sbb"] {
        let p=compile(&instantiate(description(op).unwrap(),8),8).unwrap();
        let mut c=Cpu::default();
        c.regs[0]=if op=="adc" {255} else {0};
        c.regs[1]=1;
        execute(&p,&mut c,View{register:0,low:0,width:8},View{register:1,low:0,width:8}).unwrap();
        assert_eq!(c.eflags&1,1);
        execute(&p,&mut c,View{register:2,low:0,width:8},View{register:3,low:0,width:8}).unwrap();
        assert_eq!(c.regs[2],if op=="adc" {1} else {255});
        assert_eq!(c.eflags&1,if op=="adc" {0} else {1});
    }
}
fn condition_flags(bits:u32)->u32 {
    let mut f=0x202;
    for (i,bit) in [0,2,4,6,7,11].iter().enumerate() {
        f|=((bits>>i)&1)<<*bit;
    }
    f
}
fn expected_condition(name:&str,f:u32)->u32 {
    let c=f&1!=0; let p=f&4!=0; let z=f&64!=0; let s=f&128!=0; let o=f&2048!=0;
    u32::from(match name {
        "o"=>o,"no"=>!o,"b"=>c,"ae"=>!c,"e"=>z,"ne"=>!z,
        "be"=>c||z,"a"=>!c&&!z,"s"=>s,"ns"=>!s,"p"=>p,"np"=>!p,
        "l"=>s!=o,"ge"=>s==o,"le"=>z||s!=o,"g"=>!z&&s==o,_=>panic!("bad condition"),
    })
}
#[test]
fn flag_classification_is_complete_and_unique() {
    for text in [
        "retire",
        "preserve_flag CF\nretire",
        "preserve_flags all\nundef_flag AF\nretire",
        "undef_flag AF\npreserve_flag AF\npreserve_flags remaining\nretire",
        "preserve_flags unknown\nretire",
        "undef_flag XYZ\npreserve_flags remaining\nretire",
    ] {assert!(compile(text,8).is_err(),"{text}");}
    assert!(compile("preserve_flags all\nretire",8).is_ok());
}
#[test]
fn undefined_is_distinct_from_preserved_and_not_poison() {
    let p=compile(&instantiate(AND_DESC,8),8).unwrap();
    let mut c=Cpu::default(); c.regs[0]=0xff; c.regs[1]=0x80; c.eflags=0x212;
    execute(&p,&mut c,View{register:0,low:0,width:8},View{register:1,low:0,width:8}).unwrap();
    assert_eq!(c.regs[0],0x80); assert_eq!(c.eflags&16,0);
    assert_eq!(c.undefined_flags,16);
    let keep=compile("preserve_flags all\nretire",8).unwrap();
    let before=c.clone();
    execute(&keep,&mut c,View{register:0,low:0,width:8},View{register:1,low:0,width:8}).unwrap();
    assert_eq!(c,before);
    // The selected concrete value may be consumed, with no UB or runtime trap.
    let read=compile("read_flag f AF\nzext r f 8\nwrite dst r\npreserve_flags all\nretire",8).unwrap();
    execute(&read,&mut c,View{register:0,low:0,width:8},View{register:1,low:0,width:8}).unwrap();
    assert_eq!(c.regs[0],0); assert_eq!(c.undefined_flags,16);
    let add=compile(&instantiate(ADD,8),8).unwrap();
    execute(&add,&mut c,View{register:0,low:0,width:8},View{register:1,low:0,width:8}).unwrap();
    assert_eq!(c.undefined_flags,0);
}
#[test]
fn all_setcc_conditions_preserve_flags_and_metadata() {
    for name in CONDITIONS_NAMES {
        let p=compile(&instantiate(description(&format!("set{name}")).unwrap(),8),8).unwrap();
        for bits in 0..64 {
            let f=condition_flags(bits);
            let mut c=Cpu::default(); c.regs[0]=0xabcd_ff99; c.eflags=f;
            c.undefined_flags=16; // AF is not tested by SETcc.
            execute(&p,&mut c,View{register:0,low:0,width:8},View{register:1,low:0,width:8}).unwrap();
            assert_eq!(c.regs[0],0xabcd_ff00|expected_condition(name,f),"set{name}/{f:x}");
            assert_eq!(c.eflags,f); assert_eq!(c.undefined_flags,16);
        }
        assert!(compile(&instantiate(description(&format!("set{name}")).unwrap(),16),16).is_err());
        assert!(compile(&instantiate(description(&format!("set{name}")).unwrap(),32),32).is_err());
    }
}
#[test]
fn cmp_then_setcc_distinguishes_signed_and_unsigned() {
    let cmp=compile(&instantiate(CMP,8),8).unwrap();
    for name in ["b","ae","e","ne","be","a","l","ge","le","g"] {
        let set=compile(&instantiate(description(&format!("set{name}")).unwrap(),8),8).unwrap();
        for a in [0u32,1,127,128,254,255] { for b in [0u32,1,127,128,254,255] {
            let mut c=run(&cmp,a,b,0x202);
            execute(&set,&mut c,View{register:2,low:0,width:8},View{register:1,low:0,width:8}).unwrap();
            let signed_a=a as u8 as i8; let signed_b=b as u8 as i8;
            let expected=match name {
                "b"=>a<b,"ae"=>a>=b,"e"=>a==b,"ne"=>a!=b,"be"=>a<=b,"a"=>a>b,
                "l"=>signed_a<signed_b,"ge"=>signed_a>=signed_b,
                "le"=>signed_a<=signed_b,"g"=>signed_a>signed_b,_=>unreachable!(),
            };
            assert_eq!(c.regs[2],u32::from(expected)); assert_eq!(c.regs[0],a);
        }}
    }
}
#[test]
fn conditional_write_preserves_destination_when_false() {
    for w in [8,16,32] {
        let p=compile(&instantiate(WRITE_IF_Z,w),w).unwrap();
        for taken in [false,true] {
            let mut c=Cpu::default(); c.regs[0]=0xabcd_1234; c.regs[1]=0x8765_4321;
            c.eflags=0x202|if taken {64} else {0}; c.undefined_flags=ARITH_FLAGS;
            let before=c.clone();
            execute(&p,&mut c,View{register:0,low:0,width:w},View{register:1,low:0,width:w}).unwrap();
            let m=mask(w) as u32;
            assert_eq!(c.regs[0],if taken {(before.regs[0]&!m)|(before.regs[1]&m)} else {before.regs[0]});
            assert_eq!(c.eflags,before.eflags); assert_eq!(c.undefined_flags,ARITH_FLAGS);
        }
    }
    for text in [
        "const c 8 1\nconst a 8 0\nselect r c a a\npreserve_flags all\nretire",
        "const c 1 1\nconst a 8 0\nconst b 16 0\nselect r c a b\npreserve_flags all\nretire",
        "const c 8 1\nread a src 8\nwrite_if dst a c\npreserve_flags all\nretire",
        "const c 1 1\nread a src 8\nwrite_if dst a c\nwrite dst a\npreserve_flags all\nretire",
    ] {assert!(compile(text,8).is_err());}
}
#[test]
fn and_defined_flags_match_result_and_af_is_undefined() {
    for w in [8,16,32] {
        let p=compile(&instantiate(AND_DESC,w),w).unwrap();
        let m=mask(w) as u32;
        for a in [0,1,127,128,m] {for b in [0,1,127,128,m] {
            let c=run(&p,a,b,0xffff_ffff); let r=a&b;
            let f=(0xffff_ffff&!ARITH_FLAGS) | (u32::from((r as u8).count_ones()%2==0)<<2)
                | (u32::from(r==0)<<6) | (((r>>(w-1))&1)<<7);
            assert_eq!(c.regs[0],r); assert_eq!(c.eflags,f); assert_eq!(c.undefined_flags,16);
        }}
    }
}
fn views(w:u8)->(View,View) {(View{register:0,low:0,width:w},View{register:1,low:0,width:w})}
fn branch_program(name:&str,w:u8)->Program {
    compile(&instantiate(description(&format!("j{name}")).unwrap(),w),w).unwrap()
}
#[test]
fn jcc_all_conditions_both_operand_widths() {
    for name in CONDITIONS_NAMES {for w in [16,32] {
        let p=branch_program(name,w);let (dst,src)=views(w);
        for bits in 0..64 {for (displacement,displacement_bits) in [(-128,8),(127,8),(-4,w)] {
            let mut c=Cpu::default();c.eip=0x1234_5670;c.eflags=condition_flags(bits);c.undefined_flags=16;
            let before=c.clone();let next=0x1234_5678u32;
            execute_flow(&p,&mut c,dst,src,FlowContext{next_eip:next,displacement,displacement_bits,code_limit:u32::MAX}).unwrap();
            let target=next.wrapping_add(displacement as u32)&mask(w) as u32;
            assert_eq!(c.eip,if expected_condition(name,before.eflags)!=0 {target} else {next});
            assert_eq!(c.regs,before.regs);assert_eq!(c.eflags,before.eflags);assert_eq!(c.undefined_flags,16);
        }}
    }}
}
#[test]
fn jcc_signed_displacements_wrap_at_selected_width() {
    for w in [16,32] {
        let p=branch_program("e",w);let (dst,src)=views(w);let m=mask(w) as u32;
        for (next,displacement,displacement_bits,expected) in [(m,1,8,0),(0,-1,8,m),(128,-128,8,0),(0,127,8,127)] {
            let mut c=Cpu::default();c.eflags=64;
            execute_flow(&p,&mut c,dst,src,FlowContext{next_eip:next,displacement,displacement_bits,code_limit:u32::MAX}).unwrap();
            assert_eq!(c.eip,expected);
        }
    }
}
#[test]
fn taken_branch_fault_is_restartable_and_limit_is_inclusive() {
    let p=compile("const n 32 42\nwrite dst n\nconst clear 1 0\nflag CF clear\nundef_flag AF\nconst target 32 257\nconst take 1 1\nbranch_if target take\npreserve_flags remaining\nretire",32).unwrap();
    let (dst,src)=views(32);let mut c=Cpu::default();c.regs[0]=7;c.eflags=0x213;c.eip=0x100;c.undefined_flags=1;
    let before=c.clone();
    let context=FlowContext{next_eip:0x106,displacement:0,displacement_bits:32,code_limit:256};
    assert_eq!(execute_flow(&p,&mut c,dst,src,context),Err(ExecError::GeneralProtection{eip:0x100,error_code:0}));
    assert_eq!(c,before);
    execute_flow(&p,&mut c,dst,src,FlowContext{code_limit:257,..context}).unwrap();
    assert_eq!(c.eip,257);assert_eq!(c.regs[0],42);assert_eq!(c.undefined_flags,16);
    // An untaken target beyond CS.limit is not checked by Jcc.
    let p=branch_program("e",32);let mut c=before;c.eflags=0;
    execute_flow(&p,&mut c,dst,src,FlowContext{next_eip:0x106,displacement:1000,displacement_bits:32,code_limit:256}).unwrap();
    assert_eq!(c.eip,0x106);
}
#[test]
fn flow_context_rejects_invalid_displacements_without_mutation() {
    for (w,bits,displacement) in [(16,8,128),(16,8,-129),(16,16,32768),(16,32,1),(32,16,1),(32,7,0)] {
        let p=branch_program("e",w);let (dst,src)=views(w);let mut c=Cpu::default();c.eip=100;
        let before=c.clone();
        assert_eq!(execute_flow(&p,&mut c,dst,src,FlowContext{next_eip:102,displacement,displacement_bits:bits,code_limit:u32::MAX}),Err(ExecError::InvalidContext));
        assert_eq!(c,before);
    }
    let p=branch_program("e",32);let mut c=Cpu::default();let before=c.clone();
    assert_eq!(execute_flow(&p,&mut c,View{register:8,low:0,width:32},views(32).1,FlowContext{next_eip:1,displacement:0,displacement_bits:32,code_limit:u32::MAX}),Err(ExecError::InvalidBinding));
    assert_eq!(c,before);
}
#[test]
fn sequential_eip_and_incoming_ip_snapshot() {
    let p=compile("read_ip old current 32\nread_ip following next 32\nconst take 1 1\nbranch_if following take\nwrite dst old\npreserve_flags all\nretire",32).unwrap();
    let mut c=Cpu::default();c.eip=0x100;let (dst,src)=views(32);
    execute_flow(&p,&mut c,dst,src,FlowContext{next_eip:0x106,displacement:0,displacement_bits:32,code_limit:u32::MAX}).unwrap();
    assert_eq!(c.regs[0],0x100);assert_eq!(c.eip,0x106);
    let p=compile(&instantiate(ADD,32),32).unwrap();
    execute_flow(&p,&mut c,dst,src,FlowContext{next_eip:0x108,displacement:0,displacement_bits:32,code_limit:u32::MAX}).unwrap();
    assert_eq!(c.eip,0x108);
    execute(&p,&mut c,dst,src).unwrap();assert_eq!(c.eip,0x108);
}
#[test]
fn flow_requires_explicit_context_and_well_typed_tokens() {
    let p=branch_program("e",16);let mut c=Cpu::default();let before=c.clone();let (dst,src)=views(16);
    assert!(execute(&p,&mut c,dst,src).is_err());assert_eq!(c,before);
    assert!(compile(&instantiate(description("je").unwrap(),8),8).is_err());
    for text in [
        "const t 16 0\nconst c 1 1\nbranch_if t c\npreserve_flags all\nretire",
        "const t 32 0\nconst c 8 1\nbranch_if t c\npreserve_flags all\nretire",
        "const t 32 0\nconst c 1 1\nbranch_if t c\nbranch_if t c\npreserve_flags all\nretire",
        "read_ip p unknown 32\npreserve_flags all\nretire",
        "read_disp d 16\npreserve_flags all\nretire",
    ] {assert!(compile(text,32).is_err(),"{text}");}
}
#[test]
fn small_decoded_instruction_loop_terminates() {
    let sub=compile(&instantiate(SUB,32),32).unwrap();let jne=branch_program("ne",32);
    let mut c=Cpu::default();c.eip=0x100;c.regs[0]=3;c.regs[1]=1;
    let (dst,src)=views(32);let mut steps=0;
    while c.eip!=0x108 {
        assert!(steps<10);steps+=1;
        match c.eip {
            0x100=>execute_flow(&sub,&mut c,dst,src,FlowContext{next_eip:0x102,displacement:0,displacement_bits:32,code_limit:u32::MAX}).unwrap(),
            0x102=>execute_flow(&jne,&mut c,dst,src,FlowContext{next_eip:0x108,displacement:-8,displacement_bits:32,code_limit:u32::MAX}).unwrap(),
            _=>panic!("unexpected guest offset"),
        }
    }
    assert_eq!(c.regs[0],0);assert_eq!(steps,6);
}
#[test]
fn generated_flow_matches_interpreter() {
    use std::{fs,process::Command,time::{SystemTime,UNIX_EPOCH}};
    let dir=std::env::temp_dir().join(format!("pil-flow-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    fs::create_dir(&dir).unwrap();let mut source=String::new();let mut dispatch=String::new();let mut cases=Vec::new();
    let mut id=0u32;
    for name in CONDITIONS_NAMES {for w in [16,32] {
        let p=branch_program(name,w);let (dst,src)=views(w);
        source.push_str(&format!("mod p{id} {{ {} }}\n",emit_rust(&p)));
        dispatch.push_str(&format!("{id} => p{id}::execute_with_flow(&mut regs,&mut f,&mut undefined,&mut ip,(0,0),(1,0),p{id}::FlowContext{{next_eip:next,displacement:disp,displacement_bits:bits,code_limit:limit}}).map_err(|e|match e {{p{id}::ExecError::GeneralProtection{{eip,error_code}} => (1,eip,error_code as u32),p{id}::ExecError::StackSegment{{eip,error_code}}=>(5,eip,error_code as u32),p{id}::ExecError::MemoryAccess{{eip,..}}=>(4,eip,0),p{id}::ExecError::InvalidContext=>(2,0,0),p{id}::ExecError::InvalidBinding=>(3,0,0)}}),\n"));
        for flag_bits in 0..64 {
            for (next,disp,bits,limit) in [(0x1234_5678,-128,8,u32::MAX),(0, -1,8,u32::MAX),(u32::MAX,1,8,u32::MAX),(0x100,127,8,0x100),(0x100,128,8,u32::MAX),(0x100,1,w,0x101)] {
                let initial=condition_flags(flag_bits);let mut c=Cpu::default();c.eip=0x100;c.regs[0]=7;c.regs[1]=9;c.eflags=initial;c.undefined_flags=16;
                let status=match execute_flow(&p,&mut c,dst,src,FlowContext{next_eip:next,displacement:disp,displacement_bits:bits,code_limit:limit}) {
                    Ok(())=>(0,0,0),Err(ExecError::GeneralProtection{eip,error_code})=>(1,eip,error_code as u32),Err(ExecError::StackSegment{eip,error_code})=>(5,eip,error_code as u32),Err(ExecError::MemoryAccess{eip,..})=>(4,eip,0),Err(ExecError::InvalidContext)=>(2,0,0),Err(ExecError::InvalidBinding)=>(3,0,0),
                };
                for n in [id,initial,next,disp as u32,bits as u32,limit,c.eip,c.regs[0],c.regs[1],c.eflags,c.undefined_flags,status.0,status.1,status.2] {cases.extend_from_slice(&n.to_le_bytes());}
            }
        }
        id+=1;
    }}
    source.push_str(r#"
fn main() {
 for c in include_bytes!("flow.bin").chunks_exact(56) {
  let n=|i:usize|u32::from_le_bytes(c[i..i+4].try_into().unwrap());
  let mut regs=[0u32;8];regs[0]=7;regs[1]=9;let mut f=n(4);let mut undefined=16;let mut ip=0x100;
  let next=n(8);let disp=n(12) as i32;let bits=n(16) as u8;let limit=n(20);
  let result:Result<(),(u32,u32,u32)>=match n(0) {
"#);
    source.push_str(&dispatch);
    source.push_str(r#"
   _=>panic!("bad variant"),
  };
  let status=result.err().unwrap_or((0,0,0));
  assert_eq!(ip,n(24));assert_eq!(regs[0],n(28));assert_eq!(regs[1],n(32));assert_eq!(f,n(36));assert_eq!(undefined,n(40));
  assert_eq!(status,(n(44),n(48),n(52)));
 }
}
"#);
    assert_eq!(cases.len()/56,12288);
    fs::write(dir.join("generated.rs"),source).unwrap();fs::write(dir.join("flow.bin"),cases).unwrap();
    let bin=dir.join(if cfg!(windows){"flow.exe"}else{"flow"});
    let rustc=std::env::var_os("RUSTC").unwrap_or_else(||"rustc".into());
    let out=Command::new(rustc).arg("--edition=2021").arg("-O").arg(dir.join("generated.rs")).arg("-o").arg(&bin).output().unwrap();
    assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));
    let out=Command::new(&bin).output().unwrap();assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));
    fs::remove_dir_all(dir).unwrap();
}
#[test]
fn generated_rust_matches_interpreter() {
    use std::{fs,process::Command,time::{SystemTime,UNIX_EPOCH}};
    let dir=std::env::temp_dir().join(format!("pil-proof-{}-{}",std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    fs::create_dir(&dir).unwrap();
    let mut source=String::new();
    let mut cases=Vec::new();
    let mut dispatch=String::new();
    let mut variants:Vec<(String,u8)>=Vec::new();
    for op in ["add","sub","cmp","adc","sbb","and","write_if_z"] {
        for w in [8,16,32] {variants.push((op.to_owned(),w));}
    }
    for op in ["movzx8","movsx8","cwd"]{for w in [16,32]{variants.push((op.into(),w));}}
    for op in ["movzx16","movsx16"]{for w in [16,32]{variants.push((op.into(),w));}}
    for name in CONDITIONS_NAMES {variants.push((format!("set{name}"),8));}
    for (id,(op,w)) in variants.iter().enumerate() {
        let w=*w;
        let p=compile(&instantiate(description(op).unwrap(),w),w).unwrap();
        source.push_str(&format!("mod p{id} {{ {} }}\n",emit_rust(&p)));
        dispatch.push_str(&format!("{id} => p{id}::execute_with_metadata(&mut regs,&mut f,&mut undefined,(0,0),(1,0)).unwrap(),\n"));
        let mut record=|a:u32,b:u32,initial:u32| {
            let mut c=Cpu::default();c.regs[0]=a;c.regs[1]=b;c.eflags=initial;c.undefined_flags=ARITH_FLAGS;
            execute(&p,&mut c,View{register:0,low:0,width:w},View{register:1,low:0,width:p.source_width}).unwrap();
            for n in [id as u32,a,b,initial,ARITH_FLAGS,c.regs[0],c.eflags,c.undefined_flags] {
                cases.extend_from_slice(&n.to_le_bytes());
            }
        };
        if op.starts_with("mov") {
            for b in (0..=255).chain([0x7fff,0x8000,0xffff,0x12348080]){record(0xabcd1234,b,0xad7);}
        } else if op.starts_with("set") {
            for bits in 0..64 {record(0xabcd_1234,0,condition_flags(bits));}
        } else if op=="write_if_z" {
            let m=mask(w) as u32;
            for a in [0,1,15,16,m>>1,(m>>1)+1,m] {
                for b in [0,1,15,16,m>>1,(m>>1)+1,m] {
                    for f in [0x202,0x242] {record(a,b,f);}
                }
            }
        } else {
            let carries=if op=="adc" || op=="sbb" {vec![0,1]} else {vec![1]};
            for carry in carries {
                if w==8 {
                    for a in 0..256 {for b in 0..256 {record(a,b,0x202|carry);}}
                } else {
                    let m=mask(w) as u32;let sign=1u32<<(w-1);
                    for a in [0,1,15,16,sign-1,sign,m] {
                        for b in [0,1,15,16,sign-1,sign,m] {record(a,b,0x202|carry);}
                    }
                }
            }
        }
    }
    source.push_str(r#"
fn main() {
for c in include_bytes!("expected.bin").chunks_exact(32) {
    let n = |i:usize| u32::from_le_bytes(c[i..i+4].try_into().unwrap());
    let mut regs=[0u32;8]; regs[0]=n(4); regs[1]=n(8); let mut f=n(12);let mut undefined=n(16);
    match n(0) {
"#);
    source.push_str(&dispatch);
    source.push_str(r#"
        _ => panic!("bad case"),
    }
    assert_eq!(regs[0],n(20));assert_eq!(regs[1],n(8));assert_eq!(f,n(24));assert_eq!(undefined,n(28));
}
}
"#);
    fs::write(dir.join("generated.rs"),source).unwrap();
    fs::write(dir.join("expected.bin"),cases).unwrap();
    let bin=dir.join(if cfg!(windows) {"check.exe"} else {"check"});
    let rustc=std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let result=Command::new(rustc).arg("--edition=2021").arg("-O")
        .arg(dir.join("generated.rs")).arg("-o").arg(&bin).output().unwrap();
    assert!(result.status.success(),"{}",String::from_utf8_lossy(&result.stderr));
    let result=Command::new(&bin).output().unwrap();
    assert!(result.status.success(),"{}",String::from_utf8_lossy(&result.stderr));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn exhaustive_byte_unary_flags_and_carry_preservation(){
    for op in ["inc","dec","neg","not"] {
        let p=compile(&instantiate(description(op).unwrap(),8),8).unwrap();
        for a in 0..=255u32 {for cf in [0,1] {
            let c=run(&p,a,0,0x202|cf);let result=match op{"inc"=>(a+1)&255,"dec"=>a.wrapping_sub(1)&255,"neg"=>0u32.wrapping_sub(a)&255,_=>a^255};
            assert_eq!(c.regs[0],result);
            let expected=if op=="not"{0x202|cf}else{
                let carry=if op=="neg"{u32::from(a!=0)}else{cf};
                let af=match op{"inc"=>a&15==15,"dec"=>a&15==0,_=>a&15!=0};
                let overflow=if op=="inc"{a==127}else{a==128};
                0x202|carry|(u32::from((result as u8).count_ones()%2==0)<<2)|(u32::from(af)<<4)|(u32::from(result==0)<<6)|(result&128)|(u32::from(overflow)<<11)
            };assert_eq!(c.eflags,expected,"{op} {a} CF={cf}");
        }}
    }
}

#[test]
fn imul_signed_edges_and_deterministic_products(){
    for w in [16,32] {
        let p=compile(&instantiate(description("imul").unwrap(),w),w).unwrap();let sign=1i64<<(w-1);let m=mask(w) as u32;
        let values=[-sign,-sign+1,-65536,-129,-1,0,1,127,255,32767,sign-1];
        let mut pairs=Vec::new();for a in values{for b in values{pairs.push((a,b));}}
        let mut seed=0x12345678u32;for _ in 0..500 {seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);let a=seed;seed=seed.wrapping_mul(1664525).wrapping_add(1013904223);pairs.push((a as i64,seed as i64));}
        for (a,b) in pairs {
            let (a,b)=(a as u32&m,b as u32&m);let signed=|x:u32|if x as i64&sign!=0{x as i64-2*sign}else{x as i64};let product=signed(a)*signed(b);let overflow=product < -sign || product>=sign;
            let c=run(&p,0xabcd0000&!m|a,b,0xffffffff);
            assert_eq!(c.regs[0],(0xabcd0000&!m)|((product as u32)&m));assert_eq!(c.regs[1],b);
            assert_eq!(c.eflags,(0xffffffff&!0x8d5)|if overflow{0x801}else{0});assert_eq!(c.undefined_flags,0xd4);
        }
    }
}
#[test]
fn signed_extension_and_modular_multiply_work_at_generic_width_limits(){
    use std::{fs,process::Command,time::{SystemTime,UNIX_EPOCH}};
    let dir=std::env::temp_dir().join(format!("pil-wide-mul-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir(&dir).unwrap();
    let mut source=String::from("#![allow(dead_code,unused_mut,unused_variables)]\n");let mut checks=String::new();let mut id=0;
    for w in [1,8,16,32,63,64] {for n in [mask(w)>>1,1u64<<(w-1),mask(w)] {
        let text=format!("const a {w} {n}\nsext wide a 64\nconst three 64 3\nmul product wide three\ntrunc r product 8\nbit sign wide 63\nbit top product 63\nwrite dst r\nflag CF sign\nflag OF top\npreserve_flags remaining\nretire");
        let p=compile(&text,8).unwrap();let extended=if n&(1u64<<(w-1))!=0{n|!mask(w)}else{n};let product=extended.wrapping_mul(3);let flags=((extended>>63) as u32)|(((product>>63) as u32)<<11);
        let c=run(&p,0,0,0);assert_eq!(c.regs[0],product as u8 as u32);assert_eq!(c.eflags,flags);
        source.push_str(&format!("mod p{id} {{ {} }}\n",emit_rust(&p)));checks.push_str(&format!("{{let mut regs=[0u32;8];let mut f=0;p{id}::execute(&mut regs,&mut f,(0,0),(1,0)).unwrap();assert_eq!(regs[0],{}u32);assert_eq!(f,{flags}u32);}}\n",product as u8));id+=1;
    }}
    source.push_str(&format!("fn main(){{{checks}}}"));let path=dir.join("proof.rs");fs::write(&path,source).unwrap();let exe=dir.join(if cfg!(windows){"proof.exe"}else{"proof"});
    let out=Command::new(std::env::var_os("RUSTC").unwrap_or_else(||"rustc".into())).arg("--edition=2021").arg(&path).arg("-o").arg(&exe).output().unwrap();assert!(out.status.success(),"{}",String::from_utf8_lossy(&out.stderr));assert!(Command::new(&exe).status().unwrap().success());fs::remove_dir_all(dir).unwrap();
}
#[test]
fn multiply_validation_rejects_invalid_widths_and_conversion_directions(){
    for text in ["const a 16 1\nsext b a 8\npreserve_flags all\nretire","const a 8 1\nconst b 16 1\nmul c a b\npreserve_flags all\nretire","const a 64 1\nsext b a 65\npreserve_flags all\nretire"] {assert!(compile(text,8).is_err());}
    assert!(compile(&instantiate(description("imul").unwrap(),8),8).is_err());
}

fn shift_oracle(op:&str,a:u32,count:u32,w:u8,f:u32,u:u32)->(u32,u32,u32){
    let n=count&31;if n==0{return (a,f,u);}
    let m=mask(w) as u32;let sign=1u32<<(w-1);let mut result=a&m;let mut carry=0;
    for _ in 0..n {if op=="shl"{carry=u32::from(result&sign!=0);result=(result<<1)&m;}else{carry=result&1;result=(result>>1)|if op=="sar"{result&sign}else{0};}}
    let cf_undefined=op!="sar"&&n>=u32::from(w);
    let undefined=16|if n>1{0x800}else{0}|u32::from(cf_undefined);
    let overflow=if n!=1{0}else if op=="shl"{u32::from(result&sign!=0)^carry}else if op=="shr"{u32::from(a&sign!=0)}else{0};
    let flags=(f&!0x8d5)|if cf_undefined{0}else{carry}|(u32::from((result as u8).count_ones()%2==0)<<2)|(u32::from(result==0)<<6)|(u32::from(result&sign!=0)<<7)|(overflow<<11);
    ((a&!m)|result,flags,undefined)
}
#[test]
fn shifts_match_exhaustive_byte_and_wide_edge_oracles(){
    for op in ["shl","shr","sar"] {for w in [8,16,32] {
        let p=compile(&instantiate(description(op).unwrap(),w),w).unwrap();
        let values:Vec<u32>=if w==8{(0..256).collect()}else{vec![0,1,3,mask(w) as u32,1u32<<(w-1),(1u32<<(w-1))-1,0xa55aa55a]};
        for a in values{for count in 0..=255{for (f,u) in [(0x202,0),(0xad7,0x8d5)]{
            let mut c=Cpu::default();c.regs[0]=a;c.regs[1]=count;c.eflags=f;c.undefined_flags=u;
            execute(&p,&mut c,View{register:0,low:0,width:w},View{register:1,low:0,width:w}).unwrap();
            assert_eq!((c.regs[0],c.eflags,c.undefined_flags),shift_oracle(op,a,count,w,f,u),"{op}/{w} a={a:x} count={count}");
        }}}
    }}
}
#[test]
fn conditional_flags_validate_types_and_stage_until_retirement(){
    for text in ["const a 8 1\nflag_if CF a a a\npreserve_flags remaining\nretire","const a 1 1\nflag_if CF a a a\nflag CF a\npreserve_flags remaining\nretire","const a 8 1\nconst b 16 2\nult c a b\npreserve_flags all\nretire"] {assert!(compile(text,8).is_err());}
    let p=compile("const t 1 1\nconst f 1 0\nflag_if CF t f f\nflag_if OF t f t\npreserve_flags remaining\nretire",8).unwrap();
    let c=run(&p,0,0,0xad7);assert_eq!(c.eflags,0x2d7);assert_eq!(c.undefined_flags,0x800);
}
#[test]
fn generic_shifts_define_counts_at_and_above_64_without_host_panics(){
    use std::{fs,process::Command,time::{SystemTime,UNIX_EPOCH}};
    let dir=std::env::temp_dir().join(format!("pil-shifts-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));fs::create_dir(&dir).unwrap();
    let mut source=String::from("#![allow(dead_code,unused_variables,unused_mut)]\n");let mut checks=String::new();let mut id=0;
    for op in ["shl","lshr","ashr"]{for count in [0,1,63,64,u64::MAX]{
        let text=format!("const a 64 9223372036854775809\nconst count 64 {count}\n{op} wide a count\ntrunc r wide 8\nbit high wide 63\nwrite dst r\nflag CF high\npreserve_flags remaining\nretire");
        let p=compile(&text,8).unwrap();let input=0x8000000000000001u64;
        let expected=if op=="shl"{input.checked_shl(u32::try_from(count).unwrap_or(u32::MAX)).unwrap_or(0)}else if op=="lshr"{input.checked_shr(u32::try_from(count).unwrap_or(u32::MAX)).unwrap_or(0)}else{((input as i64)>>count.min(63)) as u64};
        let c=run(&p,0,0,0);assert_eq!(c.regs[0],expected as u8 as u32);assert_eq!(c.eflags,(expected>>63) as u32);
        source.push_str(&format!("mod p{id}{{ {} }}\n",emit_rust(&p)));checks.push_str(&format!("{{let mut r=[0u32;8];let mut f=0;p{id}::execute(&mut r,&mut f,(0,0),(1,0)).unwrap();assert_eq!(r[0],{}u32);assert_eq!(f,{}u32);}}\n",expected as u8,expected>>63));id+=1;
    }}
    source.push_str(&format!("fn main(){{{checks}}}"));let path=dir.join("proof.rs");fs::write(&path,source).unwrap();let exe=dir.join(if cfg!(windows){"proof.exe"}else{"proof"});
    let output=Command::new(std::env::var_os("RUSTC").unwrap_or_else(||"rustc".into())).arg("--edition=2021").arg(&path).arg("-o").arg(&exe).output().unwrap();assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));assert!(Command::new(&exe).status().unwrap().success());fs::remove_dir_all(dir).unwrap();
}

#[test]
fn independent_source_widths_validate_views_before_any_effect(){
    let p=compile(&instantiate(description("movsx8").unwrap(),32),32).unwrap();
    assert_eq!(p.source_width,8);
    let mut c=Cpu::default();c.regs[0]=0x12348055;c.eflags=0xad7;c.undefined_flags=ARITH_FLAGS;
    let dst=View{register:0,low:0,width:32};let src=View{register:0,low:8,width:8};
    execute(&p,&mut c,dst,src).unwrap();assert_eq!(c.regs[0],0xffffff80);assert_eq!((c.eflags,c.undefined_flags),(0xad7,ARITH_FLAGS));
    let before=c.clone();
    for bad in [View{register:0,low:8,width:32},View{register:8,low:0,width:8},View{register:0,low:25,width:8}]{assert!(execute(&p,&mut c,dst,bad).is_err());assert_eq!(c,before);}
    for text in ["read a src 7\npreserve_flags all\nretire","read a src 8\nread b src 16\npreserve_flags all\nretire","read a dst 8\npreserve_flags all\nretire"]{assert!(compile(text,32).is_err(),"{text}");}
    for name in ["movsx16","movzx16"]{assert_eq!(compile(&instantiate(description(name).unwrap(),16),16).unwrap().source_width,16);assert!(compile(&instantiate(description(name).unwrap(),8),8).is_err());}
}
