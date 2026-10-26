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
