use super::*;
fn decode(bytes:&[u8])->Result<Decoded,GuestError>{let (image,ram)=Image::load(bytes).unwrap();image.decode(&ram,ENTRY)}
#[test]
fn image_rejects_empty_and_oversized_inputs() {assert!(Image::load(&[]).is_err());assert!(Image::load(&vec![0;3841]).is_err());}
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
    for bytes in [vec![0xb8,1],vec![0x67,0x89,4],vec![0x90],vec![0xf0,0x01,0xc0],vec![0xe6,0x80],vec![0x66,0x66,0xf4]] {
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
