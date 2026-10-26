use super::*;
use Gpr::*;
use SegmentRegister::*;

fn data(limit:u32)->SegmentCache {SegmentCache{base:0,limit,access:SegmentAccess::Data{writable:true,expand_down:false,big:false}}}
fn mc(segment:SegmentRegister,cache:SegmentCache)->MemoryContext {MemoryContext{segment,cache}}
#[test]
fn all_16bit_forms_use_low_halves_and_correct_default_segment() {
    let regs=[0,0,0,0xaaaa_1000,0,0xbbbb_2000,0xcccc_3000,0xdddd_4000];
    for (form,offset,segment) in [
        (Address16::BxSi,0x4007,Ds),(Address16::BxDi,0x5007,Ds),
        (Address16::BpSi,0x5007,Ss),(Address16::BpDi,0x6007,Ss),
        (Address16::Si,0x3007,Ds),(Address16::Di,0x4007,Ds),
        (Address16::Bp,0x2007,Ss),(Address16::Bx,0x1007,Ds),(Address16::Direct,7,Ds),
    ] {assert_eq!(EffectiveAddress::Bits16{form,displacement:7}.resolve(&regs,None),Ok(ResolvedAddress{offset,segment}));}
}
#[test]
fn address_overflow_and_negative_normalized_displacement_wrap() {
    let mut regs=[0;8];regs[3]=0xffff;regs[6]=2;
    assert_eq!(EffectiveAddress::Bits16{form:Address16::BxSi,displacement:0xffff}.resolve(&regs,None).unwrap().offset,0);
    regs[0]=u32::MAX;regs[1]=2;
    assert_eq!(EffectiveAddress::Bits32{base:Some(Eax),index:Some(Ecx),scale:8,displacement:u32::MAX}.resolve(&regs,None).unwrap().offset,14);
}
#[test]
fn every_32bit_base_index_and_scale_obeys_segment_rule() {
    let regs=[3,5,7,11,13,17,19,23];
    let all=[None,Some(Eax),Some(Ecx),Some(Edx),Some(Ebx),Some(Esp),Some(Ebp),Some(Esi),Some(Edi)];
    for base in all {for index in all {for scale in [1,2,4,8] {
        let r=EffectiveAddress::Bits32{base,index,scale,displacement:0xffff_fffd}.resolve(&regs,None);
        if index==Some(Esp) || (index.is_none()&&scale!=1) {assert_eq!(r,Err(ExecError::InvalidContext));continue;}
        let expected=(u64::from(base.map_or(0,|b|regs[b.index()]))+u64::from(index.map_or(0,|i|regs[i.index()]))*u64::from(scale)+0xffff_fffd)&0xffff_ffff;
        assert_eq!(r,Ok(ResolvedAddress{offset:expected as u32,segment:if matches!(base,Some(Ebp|Esp)){Ss}else{Ds}}));
    }}}
}
#[test]
fn overrides_cover_all_six_segments_including_index_only_ebp() {
    let regs=[0;8];
    for segment in [Es,Cs,Ss,Ds,Fs,Gs] {
        let a=EffectiveAddress::Bits32{base:None,index:Some(Ebp),scale:4,displacement:9};
        assert_eq!(a.resolve(&regs,None).unwrap().segment,Ds);
        assert_eq!(a.resolve(&regs,Some(segment)).unwrap().segment,segment);
        let b=EffectiveAddress::Bits16{form:Address16::Bp,displacement:0};
        assert_eq!(b.resolve(&regs,Some(segment)).unwrap().segment,segment);
    }
    for scale in [0,3,16,255] {assert!(EffectiveAddress::Bits32{base:None,index:Some(Eax),scale,displacement:0}.resolve(&regs,None).is_err());}
}
#[test]
fn ordinary_real_mode_cache_and_full_operand_limit_check() {
    let cache=SegmentCache::real_mode(0xffff);
    assert_eq!(mc(Ds,cache).translate(0xffff,MemWidth::Byte,false,7),Ok(0x10ffef));
    assert_eq!(mc(Ds,cache).translate(0xffff,MemWidth::Word,false,7),Err(ExecError::GeneralProtection{eip:7,error_code:0}));
    assert_eq!(mc(Ss,cache).translate(0xffff,MemWidth::Word,true,7),Err(ExecError::StackSegment{eip:7,error_code:0}));
}
#[test]
fn data_and_code_permissions_and_null_cache() {
    for write in [false,true] {
        let readonly=SegmentCache{access:SegmentAccess::Data{writable:false,expand_down:false,big:false},..data(9)};
        assert_eq!(mc(Ds,readonly).translate(1,MemWidth::Byte,write,7).is_ok(),!write);
        for readable in [false,true] {
            let code=SegmentCache{access:SegmentAccess::Code{readable},..data(9)};
            assert_eq!(mc(Cs,code).translate(1,MemWidth::Byte,write,7).is_ok(),readable&&!write);
        }
        let null=SegmentCache{access:SegmentAccess::Unusable,..data(9)};
        assert_eq!(mc(Fs,null).translate(0,MemWidth::Byte,write,7),Err(ExecError::GeneralProtection{eip:7,error_code:0}));
        assert_eq!(mc(Ss,readonly).translate(1,MemWidth::Byte,write,7),Err(ExecError::InvalidContext));
    }
}
#[test]
fn expand_down_limits_use_cache_big_bit_not_address_size() {
    for big in [false,true] {
        let cache=SegmentCache{access:SegmentAccess::Data{writable:true,expand_down:true,big},..data(0xffef)};
        let c=mc(Ss,cache);
        assert!(c.translate(0xffef,MemWidth::Byte,false,7).is_err());
        assert!(c.translate(0xfff0,MemWidth::Dword,true,7).is_ok());
        assert_eq!(c.translate(0xffff,MemWidth::Word,true,7).is_ok(),big);
        assert_eq!(c.translate(0x10000,MemWidth::Byte,true,7).is_ok(),big);
    }
}
#[test]
fn highest_offset_cannot_wrap_operand_span_but_base_addition_wraps() {
    let cache=SegmentCache{base:u32::MAX,..data(u32::MAX)};
    assert_eq!(mc(Ds,cache).translate(2,MemWidth::Word,false,7),Ok(1));
    assert!(mc(Ds,cache).translate(u32::MAX,MemWidth::Byte,false,7).is_ok());
    assert!(mc(Ds,cache).translate(u32::MAX,MemWidth::Word,false,7).is_err());
    assert!(mc(Ds,cache).translate(u32::MAX-2,MemWidth::Dword,false,7).is_err());
}
#[test]
fn address_context_selects_only_requested_cache() {
    let mut segments=[SegmentCache::real_mode(1);6];segments[Fs.index()]=SegmentCache::real_mode(2);
    let context=AddressContext{effective:EffectiveAddress::Bits16{form:Address16::Direct,displacement:3},override_segment:Some(Fs),segments};
    let (offset,c)=context.resolve(&[0;8]).unwrap();assert_eq!(offset,3);assert_eq!(c.translate(offset,MemWidth::Dword,true,7),Ok(35));
}
