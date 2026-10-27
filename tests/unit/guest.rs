use super::*;
fn decode(bytes:&[u8])->Result<Decoded,GuestError>{let (image,ram)=Image::load(bytes).unwrap();image.decode(&ram,ENTRY)}
#[test]
fn image_rejects_empty_and_oversized_inputs() {assert!(Image::load(&[]).is_err());assert!(Image::load(&vec![0;3841]).is_err());}
#[test]
fn direction_opcodes_and_ignored_size_segment_prefixes(){
    use super::super::Cpu;
    for (opcode,name,set) in [(0xfc,"CLD",0),(0xfd,"STD",0x400)]{
        for prefix in [vec![],vec![0x66],vec![0x67],vec![0x2e],vec![0x66,0x67]]{
            let mut bytes=prefix;bytes.push(opcode);let d=decode(&bytes).unwrap();
            assert_eq!(d.length,bytes.len() as u32);
            assert!(d.label.contains(name));
            for f in [0,0x400,0xad7,u32::MAX]{
                let mut c=Cpu{regs:[0x12345678;8],eflags:f,undefined_flags:u32::MAX,eip:ENTRY};
                let mut want=c.clone();want.eflags=(f&!0x400)|set;want.undefined_flags&=!0x400;want.eip=ENTRY+bytes.len() as u32;
                execute_one(&bytes,&mut c,0,false).unwrap();assert_eq!(c,want);
            }
        }
    }
    assert!(decode(&[0xf0,0xfc]).is_err());assert!(decode(&[0xf0,0xfd]).is_err());
}
#[test]
fn decode_immediates_and_high_byte_views() {
    for (bytes,register,low,w,len) in [(vec![0xb4,0x7f],0,8,8,2),(vec![0xbb,1,2],3,0,16,3),(vec![0x66,0xbf,1,2,3,4],7,0,32,6)] {
        let d=decode(&bytes).unwrap();assert_eq!(d.length,len);
        if let Operation::Pil{dst,flow,..}=d.operation {assert_eq!((dst.register,dst.low,dst.width),(register,low,w));assert_eq!(flow.next_eip,ENTRY+len);}else{panic!("expected PIL");}
    }
}
#[test]
fn decode_modrm_and_sib_signed_displacements() {
    for (bytes,expected) in [
        (vec![0x89,0x46,0xff],EffectiveAddress::Bits16{form:Address16::Bp,displacement:0xffff}),
        (vec![0x89,0x06,0x34,0x12],EffectiveAddress::Bits16{form:Address16::Direct,displacement:0x1234}),
        (vec![0x67,0x66,0x89,0x44,0xb3,0xfc],EffectiveAddress::Bits32{base:Some(Gpr::Ebx),index:Some(Gpr::Esi),scale:4,displacement:0xffff_fffc}),
        (vec![0x67,0x89,0x04,0xad,0x34,0x12,0,0],EffectiveAddress::Bits32{base:None,index:Some(Gpr::Ebp),scale:4,displacement:0x1234}),
    ] {
        if let Operation::Pil{address:Some(a),..}=decode(&bytes).unwrap().operation {assert_eq!(a.effective,expected);}else{panic!("expected memory operand");}
    }
}
#[test]
fn truncated_and_unsupported_instructions_are_diagnostics() {
    for bytes in [vec![0xb8,1],vec![0x67,0x89,4],vec![0x62],vec![0xf0,0x01,0xc0],vec![0xe6,0x80],vec![0x66,0x66,0xf4]] {
        assert!(matches!(decode(&bytes),Err(GuestError::Decode(_))),"{bytes:?}");
    }
}
#[test]
fn decoder_checks_cs_cache_before_reading_ram() {
    let (mut image,ram)=Image::load(&[0xb8,1,0]).unwrap();image.segments[1].limit=ENTRY;
    assert!(matches!(image.decode(&ram,ENTRY),Err(GuestError::Execution(ExecError::GeneralProtection{eip:ENTRY,error_code:0}))));
}
#[test]
fn all_short_conditions_and_register_setcc_decode() {
    for cc in 0..16 {
        let d=decode(&[0x70+cc,0xfe]).unwrap();
        if let Operation::Pil{flow,..}=d.operation {assert_eq!(flow.displacement,-2);assert_eq!(flow.displacement_bits,8);}else{panic!("branch");}
        assert!(decode(&[0x0f,0x90+cc,0xc4]).is_ok());
    }
}

fn execute_one(bytes:&[u8],cpu:&mut super::super::Cpu,ram_value:u32,readonly:bool)->Result<super::super::memory::Ram,ExecError>{
    let (image,mut ram)=Image::load(bytes).unwrap();ram.store(0x500,MemWidth::Dword,ram_value).unwrap();if readonly{ram.protect(0x500,4).unwrap();}
    let d=image.decode(&ram,ENTRY).unwrap();
    if let Operation::Pil{program,dst,src,flow,address}=d.operation {
        if let Some(a)=address {super::super::execute_segmented(&program,cpu,dst,src,flow,&mut ram,a)?;}
        else{super::super::execute_flow(&program,cpu,dst,src,flow)?;}
    }else{panic!("PIL expected");}Ok(ram)
}
#[test]
fn new_logic_unary_and_nop_have_independent_flag_expectations(){
    use super::super::Cpu;
    for (bytes,a,b,result,flags,undefined) in [
        (vec![0x09,0xc8],0x8000,1,0x8001,0x282,16),
        (vec![0x31,0xc8],0xffff,0xffff,0,0x246,16),
        (vec![0x85,0xc8],0xffff,0,0xffff,0x246,16),
        (vec![0xf7,0xd0],0x1234,0,0xedcb,0xad7,1),
        (vec![0xf7,0xd8],0x8000,0,0x8000,0xa87,0),
        (vec![0x40],0x7fff,0,0x8000,0xa97,1),
        (vec![0x48],0x8000,0,0x7fff,0xa17,1),
        (vec![0x90],0x1234,0,0x1234,0xad7,1),
    ]{
        let mut c=Cpu::default();c.eip=ENTRY;c.eflags=0xad7;c.undefined_flags=1;c.regs[0]=0xabcd0000|a;c.regs[1]=b;
        execute_one(&bytes,&mut c,0,false).unwrap();
        assert_eq!(c.regs[0],0xabcd0000|result,"{bytes:02x?}");assert_eq!(c.eflags,flags,"{bytes:02x?}");assert_eq!(c.undefined_flags,undefined,"{bytes:02x?}");
    }
}
#[test]
fn immediate_sign_extension_and_high_byte_binding(){
    use super::super::Cpu;
    for (bytes,input,output) in [
        (vec![0x83,0xc0,0xff],0x12340001,0x12340000),
        (vec![0x66,0x83,0xc0,0xff],1,0),
        (vec![0x80,0xf4,0xff],0x12345678,0x1234a978),
        (vec![0x0d,0,0x80],0,0x8000),
        (vec![0x66,0x35,0xff,0xff,0xff,0xff],0x12345678,0xedcba987),
    ]{let mut c=Cpu::default();c.eip=ENTRY;c.regs[0]=input;execute_one(&bytes,&mut c,0,false).unwrap();assert_eq!(c.regs[0],output);}
}
#[test]
fn arithmetic_both_memory_directions_and_immediate_groups(){
    use super::super::Cpu;
    for (group,input,operand,expected) in [(0,7,3,10),(1,4,3,7),(2,7,3,11),(3,7,3,3),(4,7,3,3),(5,7,3,4),(6,7,3,4),(7,7,3,7)] {
        for reverse in [false,true] {
            let bytes=[group*8+if reverse{3}else{1},0x07];let mut c=Cpu::default();c.eip=ENTRY;c.eflags=0x203;c.regs[3]=0x400;c.regs[0]=if reverse{input}else{operand};
            let ram=execute_one(&bytes,&mut c,if reverse{operand}else{input},false).unwrap();
            assert_eq!(if reverse{c.regs[0]}else{ram.load(0x500,MemWidth::Word).unwrap()},expected,"group{group},reverse{reverse}");
        }
        let bytes=[0x83,0x07|(group<<3),operand as u8];let mut c=Cpu::default();c.eip=ENTRY;c.eflags=0x203;c.regs[3]=0x400;
        let ram=execute_one(&bytes,&mut c,input,false).unwrap();assert_eq!(ram.load(0x500,MemWidth::Word).unwrap(),expected);
    }
}
#[test]
fn readonly_test_cmp_and_source_reads_succeed_while_rmw_faults_preserve_cpu(){
    use super::super::Cpu;
    for bytes in [vec![0x85,0x07],vec![0xf7,0x07,0xff,0xff],vec![0x83,0x3f,1],vec![0x33,0x07]]{
        let mut c=Cpu::default();c.eip=ENTRY;c.regs[3]=0x400;assert!(execute_one(&bytes,&mut c,42,true).is_ok());
    }
    for bytes in [vec![0x09,0x07],vec![0x83,0x37,1],vec![0xf7,0x17],vec![0xf7,0x1f],vec![0xff,0x07],vec![0xff,0x0f]]{
        let mut c=Cpu::default();c.eip=ENTRY;c.regs[3]=0x400;c.eflags=0xad7;c.undefined_flags=0x10;let before=c.clone();
        assert!(execute_one(&bytes,&mut c,42,true).is_err());assert_eq!(c,before,"{bytes:02x?}");
    }
}
#[test]
fn new_decode_forms_are_bounded_and_reserved_extensions_stay_unsupported(){
    for bytes in [vec![0x81,0xc0,1],vec![0x83,0x86,0],vec![0xf7,0xc0,1],vec![0xff,0xd8],vec![0xf6,0xc8],vec![0xfe,0xd0],vec![0x82,0xc0,1]] {assert!(matches!(decode(&bytes),Err(GuestError::Decode(_))),"{bytes:02x?}");}
    for prefix in [vec![],vec![0x66]] {for opcode in 0x40..=0x4f {let mut bytes=prefix.clone();bytes.push(opcode);assert!(decode(&bytes).is_ok());}}
}

#[test]
fn imul_binary_operands_aliases_and_signed_immediates(){
    use super::super::Cpu;
    for (bytes,a,b,result,overflow) in [
        (vec![0x0f,0xaf,0xc1],0xfffe,3,0xfffa,false),
        (vec![0x0f,0xaf,0xc0],0x8000,0,0,true),
        (vec![0x66,0x0f,0xaf,0xc1],0x80000000,0xffffffff,0x80000000,true),
        (vec![0x6b,0xc1,0xfe],123,3,0xfffa,false),
        (vec![0x69,0xc1,0,0x80],123,1,0x8000,false),
        (vec![0x66,0x6b,0xc1,0x80],123,0x01000000,0x80000000,false),
        (vec![0x66,0x69,0xc1,0xff,0xff,0xff,0xff],123,0x80000000,0x80000000,true),
    ]{
        let mut c=Cpu::default();c.eip=ENTRY;c.eflags=0xad7;c.regs[0]=a;c.regs[1]=b;
        execute_one(&bytes,&mut c,0,false).unwrap();assert_eq!(c.regs[0],result,"{bytes:02x?}");assert_eq!(c.eflags,0x202|if overflow{0x801}else{0});assert_eq!(c.undefined_flags,0xd4);
    }
}
#[test]
fn imul_memory_source_is_readonly_and_faults_preserve_cpu(){
    use super::super::Cpu;
    for bytes in [vec![0x0f,0xaf,0x07],vec![0x6b,0x07,0xfe],vec![0x69,0x07,0xfe,0xff]] {
        let mut c=Cpu::default();c.eip=ENTRY;c.regs[0]=0xfffe;c.regs[3]=0x400;
        let ram=execute_one(&bytes,&mut c,3,true).unwrap();assert_eq!(c.regs[0],0xfffa);assert_eq!(ram.load(0x500,MemWidth::Word).unwrap(),3);
    }
    let mut c=Cpu::default();c.eip=ENTRY;c.regs[0]=0x8000;c.regs[3]=0xffff;let before=c.clone();
    assert!(execute_one(&[0x0f,0xaf,0x07],&mut c,0,false).is_err());assert_eq!(c,before);
}
#[test]
fn imul_fetch_lengths_and_unimplemented_implicit_forms_are_diagnostics(){
    for bytes in [vec![0x0f,0xaf],vec![0x69,0xc0,1],vec![0x66,0x6b,0x86,0],vec![0xf6,0xe8],vec![0xf7,0xe8]] {assert!(matches!(decode(&bytes),Err(GuestError::Decode(_))),"{bytes:02x?}");}
    let (mut image,ram)=Image::load(&[0x69,0xc0,1,0]).unwrap();image.segments[1].limit=ENTRY+2;
    assert!(matches!(image.decode(&ram,ENTRY),Err(GuestError::Execution(ExecError::GeneralProtection{eip:ENTRY,error_code:0}))));
}

#[test]
fn shift_count_forms_cl_aliases_and_high_bytes(){
    use super::super::Cpu;
    for (bytes,input,cl,expected) in [
        (vec![0xd0,0xe4],0x12348078,0,0x12340078),
        (vec![0xd3,0xe1],0,0x12340001,0x12340002),
        (vec![0x66,0xc1,0xf8,8],0xfffffe01,0,0xfffffffe),
        (vec![0xc1,0xe8,32],0x12348001,0,0x12348001),
        (vec![0x66,0xd3,0xe8],0x80000000,255,1),
    ]{
        let mut c=Cpu::default();c.eip=ENTRY;c.regs[0]=input;c.regs[1]=cl;
        execute_one(&bytes,&mut c,0,false).unwrap();assert_eq!(if bytes==vec![0xd3,0xe1]{c.regs[1]}else{c.regs[0]},expected,"{bytes:02x?}");
    }
}
#[test]
fn memory_shift_rmw_checks_zero_counts_and_preserves_fault_state(){
    use super::super::Cpu;
    for (bytes,count,expected) in [(vec![0xd1,0x27],1,0x80000002),(vec![0xc1,0x3f,0],0,0x80008001),(vec![0x66,0xd3,0x3f],8,0xff800080)] {
        let mut c=Cpu::default();c.eip=ENTRY;c.regs[1]=count;c.regs[3]=0x400;c.eflags=0xad7;c.undefined_flags=0x8d5;let before=c.clone();
        let ram=execute_one(&bytes,&mut c,0x80008001,false).unwrap();assert_eq!(ram.load(0x500,MemWidth::Dword).unwrap(),expected);
        let mut c=before.clone();assert!(execute_one(&bytes,&mut c,0x80008001,true).is_err());assert_eq!(c,before);
    }
}
#[test]
fn unimplemented_rotate_extensions_and_truncated_operands_stay_diagnostics(){
    for bytes in [vec![0xc1,0xe0],vec![0xc0,0x66],vec![0xd1,0xd0],vec![0xd3,0xf0],vec![0xc1,0x87,0]] {assert!(matches!(decode(&bytes),Err(GuestError::Decode(_))),"{bytes:02x?}");}
    assert_eq!(description("shl").unwrap(),description("sal").unwrap());
}

#[test]
fn decoded_labels_show_registers_memory_sizes_segments_and_sib(){
    for (bytes,expected) in [
        (vec![0xb4,0x7f],"MOV AH,0x7f"),
        (vec![0xbb,1,2],"MOV BX,0x0201"),
        (vec![0x66,0xb8,1,0,0,0],"MOV EAX,0x00000001"),
        (vec![0x66,0x01,0xd0],"ADD EAX,EDX"),
        (vec![0x66,0x89,0x05],"MOV dword DS:[DI],EAX"),
        (vec![0x89,0x46,0xff],"MOV word SS:[BP-0x1],AX"),
        (vec![0x64,0x66,0x8b,0x07],"MOV EAX,dword FS:[BX]"),
        (vec![0x67,0x66,0x89,0x44,0xb3,0xfc],"MOV dword DS:[EBX+ESI*4-0x4],EAX"),
        (vec![0x67,0x8b,0x04,0xad,0x34,0x12,0,0],"MOV AX,word DS:[EBP*4+0x1234]"),
        (vec![0x89,0x06,0x34,0x12],"MOV word DS:[0x1234],AX"),
    ]{assert_eq!(decode(&bytes).unwrap().label,expected);}
}
#[test]
fn decoded_labels_cover_branches_unary_shifts_and_multiply(){
    for (bytes,expected) in [
        (vec![0x75,0xfe],"JNE 0x00000100"),
        (vec![0x0f,0x92,0xc4],"SETB AH"),
        (vec![0x83,0xc0,0xff],"ADD AX,0xffff"),
        (vec![0xf7,0x1f],"NEG word DS:[BX]"),
        (vec![0xd3,0xf8],"SAR AX,CL"),
        (vec![0xc1,0x27,8],"SHL word DS:[BX],0x08"),
        (vec![0x0f,0xaf,0xc1],"IMUL AX,CX"),
        (vec![0x66,0x6b,0x07,0xff],"IMUL EAX,dword DS:[BX],0xffffffff"),
        (vec![0x40],"INC AX"),
        (vec![0x90],"NOP"),
    ]{assert_eq!(decode(&bytes).unwrap().label,expected);}
}

#[test]
fn relative_jump_and_all_near_conditions_decode_at_both_widths(){
    for w in [16,32]{for cc in 0..16{
        let mut bytes=Vec::new();if w==32{bytes.push(0x66);}bytes.extend([0x0f,0x80+cc]);bytes.extend(vec![0xff;(w/8) as usize]);
        let d=decode(&bytes).unwrap();assert_eq!(d.length,bytes.len() as u32);
        if let Operation::Pil{flow,..}=d.operation{assert_eq!(flow.displacement,-1);assert_eq!(flow.displacement_bits,w);}else{panic!("PIL");}
    }}
    for(bytes,target)in [(vec![0xeb,0xfe],ENTRY),(vec![0xe9,0xfd,0xff],ENTRY),(vec![0x66,0xe9,0xfa,0xff,0xff,0xff],ENTRY)]{assert_eq!(decode(&bytes).unwrap().label,format!("JMP 0x{target:08x}"));}
    for bytes in [vec![0xe9,0],vec![0x0f,0x84,0],vec![0x66,0xe9,1,2,3]]{assert!(matches!(decode(&bytes),Err(GuestError::Decode(_))));}
}
#[test]
fn near_branch_targets_preserve_flags_and_fault_before_retirement(){
    use super::super::Cpu;
    for(bytes,flags,target)in [(vec![0xe9,0,0],0xad7,ENTRY+3),(vec![0x0f,0x84,2,0],0x202,ENTRY+4),(vec![0x0f,0x84,2,0],0x242,ENTRY+6)]{
        let mut c=Cpu::default();c.eip=ENTRY;c.eflags=flags;c.undefined_flags=0x10;execute_one(&bytes,&mut c,0,false).unwrap();assert_eq!(c.eip,target);assert_eq!(c.eflags,flags);assert_eq!(c.undefined_flags,0x10);
    }
    let(mut image,ram)=Image::load(&[0xe9,0xff,0x7f]).unwrap();image.segments[1].limit=ENTRY+2;
    let d=image.decode(&ram,ENTRY).unwrap();let mut c=Cpu::default();c.eip=ENTRY;let old=c.clone();if let Operation::Pil{program,dst,src,flow,..}=d.operation{assert!(super::super::execute_flow(&program,&mut c,dst,src,flow).is_err());}else{panic!("PIL");}assert_eq!(c,old);
}

#[test]
fn extending_moves_exhaust_bytes_and_word_boundaries_preserving_flags(){
    use super::super::Cpu;
    for w in [16,32]{for signed in [false,true]{for source_width in [8,16]{
        if source_width>w{continue;}
        let mut values:Vec<u32>=if source_width==8{(0..=255).collect()}else{vec![0,1,0x7fff,0x8000,0xffff]};values.push(0x12348080);
        for value in values{
            let mut bytes=Vec::new();if w==32{bytes.push(0x66);}bytes.extend([0x0f,if signed{0xbe}else{0xb6}+if source_width==16{1}else{0},0xc1]);
            let mut c=Cpu::default();c.eip=ENTRY;c.regs[0]=0xabcd5678;c.regs[1]=value;c.eflags=0xad7;c.undefined_flags=0x8d5;
            execute_one(&bytes,&mut c,0,false).unwrap();
            let expected=if signed{if source_width==8{value as i8 as i32 as u32}else{value as i16 as i32 as u32}}else{value&super::super::mask(source_width)as u32};
            assert_eq!(c.regs[0],if w==16{0xabcd0000|(expected&0xffff)}else{expected});assert_eq!(c.regs[1],value);
            assert_eq!((c.eflags,c.undefined_flags),(0xad7,0x8d5));
        }
    }}}
}
#[test]
fn extension_high_bytes_and_accumulator_aliases_read_original_registers(){
    use super::super::Cpu;
    for(bytes,input,result)in [(vec![0x0f,0xb6,0xc4],0x12348055,0x12340080),(vec![0x66,0x0f,0xbe,0xc4],0x12348055,0xffffff80),(vec![0x98],0x12340080,0x1234ff80),(vec![0x66,0x98],0xabcd8000,0xffff8000)]{
        let mut c=Cpu::default();c.eip=ENTRY;c.regs[0]=input;c.eflags=0xad7;c.undefined_flags=16;
        execute_one(&bytes,&mut c,0,false).unwrap();assert_eq!(c.regs[0],result);assert_eq!((c.eflags,c.undefined_flags),(0xad7,16));
    }
    for w in [16,32]{for value in [0,1,0x7fff,0x8000,0x7fffffff,0x80000000,u32::MAX]{
        let mut bytes=Vec::new();if w==32{bytes.push(0x66);}bytes.push(0x99);
        let mut c=Cpu::default();c.eip=ENTRY;c.regs[0]=value;c.regs[2]=0x12345678;c.eflags=0xad7;c.undefined_flags=16;
        execute_one(&bytes,&mut c,0,false).unwrap();let negative=value&(1<<(w-1))!=0;
        assert_eq!(c.regs[2],if w==16{0x12340000|if negative{0xffff}else{0}}else if negative{u32::MAX}else{0});assert_eq!(c.regs[0],value);assert_eq!((c.eflags,c.undefined_flags),(0xad7,16));
    }}
}
#[test]
fn extending_memory_moves_are_readonly_and_use_source_access_width(){
    use super::super::Cpu;
    for (bytes,value,result)in [(vec![0x0f,0xbe,0x07],0x80,0xabcdff80),(vec![0x0f,0xbf,0x07],0x8001,0xabcd8001),(vec![0x0f,0xb7,0x07],0x8001,0xabcd8001),(vec![0x66,0x0f,0xb6,0x07],0xff,255),(vec![0x66,0x0f,0xbf,0x07],0x8001,0xffff8001)]{
        let mut c=Cpu::default();c.eip=ENTRY;c.regs[0]=0xabcd1234;c.regs[3]=0x400;
        let ram=execute_one(&bytes,&mut c,value,true).unwrap();assert_eq!(c.regs[0],result);assert_eq!(ram.load(0x500,MemWidth::Dword).unwrap(),value);
    }
    // Exactly one byte left in DS and RAM: byte source succeeds; word source faults.
    for word in [false,true]{
        let (mut image,mut ram)=Image::load(&[0x66,0x0f,if word{0xb7}else{0xb6},0x07]).unwrap();
        image.segments[SegmentRegister::Ds.index()].base=0;image.segments[SegmentRegister::Ds.index()].limit=4095;
        ram.store(4095,MemWidth::Byte,0x80).unwrap();let mut c=Cpu::default();c.eip=ENTRY;c.regs[3]=4095;c.regs[0]=0x1234;let old=c.clone();let old_ram=ram.clone();
        if let Operation::Pil{program,dst,src,flow,address:Some(a)}=image.decode(&ram,ENTRY).unwrap().operation{
            let result=super::super::execute_segmented(&program,&mut c,dst,src,flow,&mut ram,a);
            if word{assert!(result.is_err());assert_eq!(c,old);}else{result.unwrap();assert_eq!(c.regs[0],0x80);}
        }else{panic!("memory PIL");}assert_eq!(ram,old_ram);
    }
}
#[test]
fn lea_all_operand_address_sizes_wrap_without_segment_or_ram_access(){
    use super::super::Cpu;
    for wide in [false,true]{for addr32 in [false,true]{
        let mut bytes=vec![0x64];if wide{bytes.push(0x66);}if addr32{bytes.push(0x67);}
        bytes.extend(if addr32{vec![0x8d,0x44,0x8b,0xfc]}else{vec![0x8d,0x40,0xff]});
        let (mut image,mut ram)=Image::load(&bytes).unwrap();for i in [0,2,3,4,5]{image.segments[i].access=SegmentAccess::Unusable;image.segments[i].base=0xffffffff;image.segments[i].limit=0;}
        ram.protect(0,RAM_BYTES).unwrap();let old_ram=ram.clone();let mut c=Cpu::default();c.eip=ENTRY;c.regs[0]=0xabcd1234;c.regs[3]=0xfffffff0;c.regs[1]=8;c.regs[6]=0x12340030;c.eflags=0xad7;c.undefined_flags=0x8d5;
        let offset=if addr32{12}else{0x1f};
        if let Operation::Pil{program,dst,src,flow,address:Some(a)}=image.decode(&ram,ENTRY).unwrap().operation{super::super::execute_segmented(&program,&mut c,dst,src,flow,&mut ram,a).unwrap();}else{panic!("LEA PIL");}
        assert_eq!(c.regs[0],if wide{offset}else{0xabcd0000|offset});assert_eq!((c.eflags,c.undefined_flags),(0xad7,0x8d5));assert_eq!(ram,old_ram);
    }}
    // Address32 -> destination16 truncation is independent of wrap at address width.
    let mut c=Cpu::default();c.eip=ENTRY;c.regs[3]=0x12348000;c.regs[0]=0xabcd1234;
    execute_one(&[0x67,0x8d,0x03],&mut c,0,false).unwrap();assert_eq!(c.regs[0],0xabcd8000);
}
#[test]
fn extension_and_lea_disassembly_and_invalid_encodings(){
    for(bytes,label)in [(vec![0x66,0x0f,0xbe,0xc4],"MOVSX EAX,AH"),(vec![0x0f,0xb6,0x07],"MOVZX AX,byte DS:[BX]"),(vec![0x66,0x0f,0xbf,0x46,0xff],"MOVSX EAX,word SS:[BP-0x1]"),(vec![0x64,0x67,0x66,0x8d,0x44,0xb3,0xfc],"LEA EAX,[EBX+ESI*4-0x4]"),(vec![0x0f,0xbf,0xf8],"MOVSX DI,AX"),(vec![0x0f,0xbf,0xff],"MOVSX DI,DI"),(vec![0x0f,0xb7,0xff],"MOVZX DI,DI"),(vec![0x98],"CBW"),(vec![0x66,0x98],"CWDE"),(vec![0x99],"CWD"),(vec![0x66,0x99],"CDQ")]{assert_eq!(decode(&bytes).unwrap().label,label);}
    for bytes in [vec![0x0f,0xb6],vec![0x66,0x0f,0xbf,0x86,0],vec![0x8d,0xc0],vec![0x8d,0x06,0]]{assert!(matches!(decode(&bytes),Err(GuestError::Decode(_))),"{bytes:?}");}
}

#[test]
fn rotate_count_forms_high_bytes_and_cl_aliases(){
    use super::super::Cpu;
    for(bytes,a,ecx,register,expected)in [
        (vec![0xd0,0xc4],0xabcd8155,0,0,0xabcd0355),
        (vec![0xd0,0xcc],0xabcd8155,0,0,0xabcdc055),
        (vec![0xd3,0xc1],0,0x12340001,1,0x12340002),
        (vec![0xd3,0xc9],0,0x12340001,1,0x12348000),
        (vec![0x66,0xc1,0xc8,8],0x80000001,0,0,0x01800000),
        (vec![0x66,0xc1,0xc0,0],0x12348001,0,0,0x12348001),
        (vec![0xc1,0xc0,8],0x12348001,0,0,0x12340180),
    ]{
        let mut c=Cpu::default();c.eip=ENTRY;c.regs[0]=a;c.regs[1]=ecx;
        execute_one(&bytes,&mut c,0,false).unwrap();assert_eq!(c.regs[register],expected,"{bytes:02x?}");
    }
}
#[test]
fn rotate_memory_rmw_checks_permissions_even_for_zero_and_full_turns(){
    use super::super::Cpu;
    for(bytes,count,expected)in [
        (vec![0xd1,0x07],1,0x80000103),
        (vec![0xc0,0x0f,8],0,0x80008081),
        (vec![0x66,0xc1,0x0f,4],0,0x18000808),
        (vec![0xd2,0x07],32,0x80008081),
    ]{
        let mut c=Cpu::default();c.eip=ENTRY;c.regs[1]=count;c.regs[3]=0x400;c.eflags=0xad7;c.undefined_flags=0x8d5;
        let before=c.clone();let ram=execute_one(&bytes,&mut c,0x80008081,false).unwrap();assert_eq!(ram.load(0x500,MemWidth::Dword).unwrap(),expected);
        let mut fault=before.clone();assert!(execute_one(&bytes,&mut fault,0x80008081,true).is_err());assert_eq!(fault,before);
    }
}
#[test]
fn carry_controls_decode_with_operand_prefix_and_preserve_untouched_state(){
    use super::super::Cpu;
    for(op,carry)in [(0xf8,0),(0xf9,1),(0xf5,0)]{for prefix in [false,true]{
        let mut bytes=if prefix{vec![0x66]}else{Vec::new()};bytes.push(op);
        let mut c=Cpu{regs:[0xabcdef01;8],eflags:0xad7,undefined_flags:0x8d5,eip:ENTRY};
        execute_one(&bytes,&mut c,0,false).unwrap();assert_eq!(c.regs,[0xabcdef01;8]);
        assert_eq!((c.eflags,c.undefined_flags,c.eip),((0xad7&!1)|carry,0x8d4,ENTRY+bytes.len()as u32));
    }}
}

#[test]
fn cmc_involution_prefixes_readonly_ram_and_fetch_faults(){
    use super::super::Cpu;
    for prefix in [vec![],vec![0x66],vec![0x67],vec![0x64],vec![0x66,0x67]]{
        let mut bytes=prefix;bytes.push(0xf5);
        assert_eq!(decode(&bytes).unwrap().label,"CMC");
        for f in [0x202,0x203,0xffff_fffe,u32::MAX]{for u in [0,1,0x8d5]{
            let mut c=Cpu{regs:[0x12345678;8],eflags:f,undefined_flags:u,eip:ENTRY};
            let ram=execute_one(&bytes,&mut c,0x89abcdef,true).unwrap();
            assert_eq!(c.regs,[0x12345678;8]);
            assert_eq!((c.eflags,c.undefined_flags,c.eip),(f^1,u&!1,ENTRY+bytes.len()as u32));
            assert_eq!(ram.load(0x500,MemWidth::Dword).unwrap(),0x89abcdef);
            c.eip=ENTRY;execute_one(&bytes,&mut c,0,true).unwrap();
            assert_eq!((c.eflags,c.undefined_flags),(f,u&!1));
        }}
    }
    for bytes in [vec![0xf0,0xf5],vec![0x66,0x66,0xf5]]{assert!(matches!(decode(&bytes),Err(GuestError::Decode(_))));}
    let (mut image,ram)=Image::load(&[0x66,0xf5]).unwrap();image.segments[1].limit=ENTRY;
    assert!(matches!(image.decode(&ram,ENTRY),Err(GuestError::Execution(ExecError::GeneralProtection{eip:ENTRY,error_code:0}))));
}

#[test]
fn exchange_register_forms_aliases_accumulator_and_memory_diagnostics(){
    use super::super::Cpu;
    for(bytes,expected0,expected1)in [
        (vec![0x86,0xc8],0x12345601,0xabcdef78),
        (vec![0x86,0xe0],0x12347856,0xabcdef01),
        (vec![0x86,0xc0],0x12345678,0xabcdef01),
        (vec![0x87,0xc8],0x1234ef01,0xabcd5678),
        (vec![0x91],0x1234ef01,0xabcd5678),
        (vec![0x66,0x91],0xabcdef01,0x12345678),
        (vec![0x90],0x12345678,0xabcdef01),
    ]{
        let mut c=Cpu{regs:[0x12345678,0xabcdef01,2,3,4,5,6,7],eflags:0xad7,undefined_flags:0x8d5,eip:ENTRY};
        execute_one(&bytes,&mut c,0,false).unwrap();assert_eq!((c.regs[0],c.regs[1]),(expected0,expected1),"{bytes:02x?}");assert_eq!((c.eflags,c.undefined_flags),(0xad7,0x8d5));
    }
    for bytes in [vec![0x87,0x07],vec![0x86,0x07],vec![0xf0,0x87,0xc8],vec![0x87]]{assert!(matches!(decode(&bytes),Err(GuestError::Decode(_))),"{bytes:02x?}");}
}

#[test]
fn byte_flag_transfers_bind_ah_ignore_size_prefixes_and_check_fetch(){
    use super::super::Cpu;
    for opcode in [0x9e,0x9f]{for prefix in [vec![],vec![0x66],vec![0x67],vec![0x64],vec![0x66,0x67]]{
        let mut bytes=prefix;bytes.push(opcode);assert_eq!(decode(&bytes).unwrap().label,if opcode==0x9f{"LAHF"}else{"SAHF"});
        for byte in 0..256{for f in [0x202,0xad7]{
            let mut c=Cpu{regs:[0x12340078|(byte<<8),1,2,3,4,5,6,7],eflags:f,undefined_flags:0x8d5,eip:ENTRY};let mut want=c.clone();want.eip+=bytes.len()as u32;
            if opcode==0x9f{want.regs[0]=(want.regs[0]&!0xff00)|(((f&0xd5)|2)<<8);}
            else{want.eflags=(f&!0xd5)|(byte&0xd5);want.undefined_flags&=!0xd5;}
            let ram=execute_one(&bytes,&mut c,0x89abcdef,true).unwrap();assert_eq!(c,want);assert_eq!(ram.load(0x500,MemWidth::Dword).unwrap(),0x89abcdef);
        }}
    }
    for bytes in [vec![0xf0,opcode],vec![0x66,0x66,opcode]]{assert!(matches!(decode(&bytes),Err(GuestError::Decode(_))));}
    let(mut image,ram)=Image::load(&[0x66,opcode]).unwrap();image.segments[1].limit=ENTRY;
    assert!(matches!(image.decode(&ram,ENTRY),Err(GuestError::Execution(ExecError::GeneralProtection{eip:ENTRY,error_code:0}))));
    }
}

