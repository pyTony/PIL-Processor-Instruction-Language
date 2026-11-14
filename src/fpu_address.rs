//! Reuse the common effective-address and segment-cache model for x87 memory
//! operands. Prefixes, SIB, displacement and the SS/DS default all matter.
use crate::{Cpu, address::{Address16, AddressContext, EffectiveAddress, Gpr, MemoryContext,
    SegmentRegister}, flow::ExecError, guest::Image};
use super::Instruction;

fn gpr(n:u8)->Gpr{
    [Gpr::Eax,Gpr::Ecx,Gpr::Edx,Gpr::Ebx,Gpr::Esp,Gpr::Ebp,Gpr::Esi,Gpr::Edi][usize::from(n)]
}
/// Resolve one already-decoded ESC memory operand without re-fetching code.
/// Data access permissions, segment bounds and paging are checked by caller.
pub fn resolve(instruction:&Instruction,cpu:&Cpu,image:&Image)
    ->Result<(u32,MemoryContext),ExecError>{
    let bytes=&instruction.bytes[..instruction.length as usize];
    let mut at=0usize;
    let mut addr32=image.segments[SegmentRegister::Cs.index()].default_32();
    let mut override_segment=None;
    while let Some(&b)=bytes.get(at){
        match b {
            0x66=>{},
            0x67=>addr32=!addr32,
            0x26=>override_segment=Some(SegmentRegister::Es),
            0x2e=>override_segment=Some(SegmentRegister::Cs),
            0x36=>override_segment=Some(SegmentRegister::Ss),
            0x3e=>override_segment=Some(SegmentRegister::Ds),
            0x64=>override_segment=Some(SegmentRegister::Fs),
            0x65=>override_segment=Some(SegmentRegister::Gs),
            _=>break,
        }
        at+=1;
    }
    if bytes.get(at)!=Some(&instruction.opcode){return Err(ExecError::InvalidContext);}
    at+=1;
    let m=*bytes.get(at).ok_or(ExecError::InvalidContext)?;
    if Some(m)!=instruction.modrm||m>>6==3{return Err(ExecError::InvalidContext);}
    at+=1;
    let mode=m>>6;let rm=m&7;
    let mut read=|n:usize|->Result<u32,ExecError>{
        if at+n>bytes.len(){return Err(ExecError::InvalidContext);}
        let mut value=0u32;
        for i in 0..n {value|=u32::from(bytes[at+i])<<(i*8);}
        at+=n;Ok(value)
    };
    let effective=if !addr32{
        let direct=mode==0&&rm==6;
        let form=if direct{Address16::Direct}else{
            [Address16::BxSi,Address16::BxDi,Address16::BpSi,Address16::BpDi,
             Address16::Si,Address16::Di,Address16::Bp,Address16::Bx][usize::from(rm)]
        };
        let displacement=match mode {
            0 if direct=>read(2)? as u16,
            1=>read(1)? as u8 as i8 as i16 as u16,
            2=>read(2)? as u16,
            _=>0,
        };
        EffectiveAddress::Bits16{form,displacement}
    }else{
        let(base,index,scale)=if rm==4{
            let sib=read(1)? as u8;
            let b=sib&7;let i=(sib>>3)&7;
            (if mode==0&&b==5{None}else{Some(gpr(b))},
             if i==4{None}else{Some(gpr(i))},1u8<<(sib>>6))
        }else{
            (if mode==0&&rm==5{None}else{Some(gpr(rm))},None,1)
        };
        let displacement=match mode{
            0 if base.is_none()=>read(4)?,
            1=>read(1)? as u8 as i8 as i32 as u32,
            2=>read(4)?,
            _=>0,
        };
        EffectiveAddress::Bits32{base,index,scale,displacement}
    };
    if at!=bytes.len(){return Err(ExecError::InvalidContext);}
    AddressContext{effective,override_segment,segments:image.segments}.resolve(&cpu.regs)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{fpu::decode, address::{SegmentAccess,SegmentCache}, guest::Image};
    fn case(bytes:&[u8],cpu:Cpu,image:&Image)->(u32,MemoryContext){
        let ins=decode(|at|bytes.get(at as usize).copied().ok_or(())).unwrap().unwrap();
        resolve(&ins,&cpu,image).unwrap()
    }
    #[test]
    fn addressing_16bit_bp_defaults_ss_and_segment_override(){
        let(image,_)=Image::load(&[0x90]).unwrap();
        let mut cpu=Cpu::default();cpu.regs[5]=0x300;cpu.regs[6]=0x20;
        let (offset,mem)=case(&[0xdd,0x42,0xf0],cpu.clone(),&image); // [BP+SI-16]
        assert_eq!(offset,0x310);assert_eq!(mem.segment,SegmentRegister::Ss);
        let (offset,mem)=case(&[0x26,0xdd,0x42,0xf0],cpu,&image);
        assert_eq!(offset,0x310);assert_eq!(mem.segment,SegmentRegister::Es);
    }
    #[test]
    fn addressing_32bit_sib_no_base_and_displacement(){
        let(image,_)=Image::load(&[0x90]).unwrap();
        let mut cpu=Cpu::default();cpu.regs[1]=0x10;
        // [ECX*4+0x1000], 67h address override, SIB 8D: scale4,ECX,no base
        let (offset,mem)=case(&[0x67,0xdd,0x04,0x8d,0x00,0x10,0,0],cpu,&image);
        assert_eq!(offset,0x1040);assert_eq!(mem.segment,SegmentRegister::Ds);
    }
    #[test]
    fn protected_default_32bit_addressing_and_67_override(){
        let(mut image,_)=Image::load(&[0x90]).unwrap();
        image.segments[SegmentRegister::Cs.index()]=SegmentCache{
            base:0,limit:u32::MAX,
            access:SegmentAccess::Code{readable:true,conforming:false,big:true,dpl:0}
        };
        let mut cpu=Cpu::default();cpu.regs[3]=0x10003;
        let ins=super::super::decode_with_default(
            |at|[0xdd,0x03].get(at as usize).copied().ok_or(()),true
        ).unwrap().unwrap();
        let (offset,_)=resolve(&ins,&cpu,&image).unwrap();
        assert_eq!(offset,0x10003);
        let ins=super::super::decode_with_default(
            |at|[0x67,0xdd,0x07].get(at as usize).copied().ok_or(()),true
        ).unwrap().unwrap();
        let (offset,_)=resolve(&ins,&cpu,&image).unwrap();
        assert_eq!(offset,3);
    }
}
