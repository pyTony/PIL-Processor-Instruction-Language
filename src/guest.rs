//! Small monitor profile for executable test images, not a general PC loader.
use super::{compile, description, instantiate, Program, View};
use super::address::*;
use super::flow::{ExecError, FlowContext};
use super::memory::{MemWidth, Ram};

pub const ENTRY:u32=0x100;
pub const RAM_BYTES:usize=4096;
pub fn selftest_bytes()->Result<Vec<u8>,String> {
    include_str!("../programs/selftest.hex").split_whitespace().map(|s|u8::from_str_radix(s,16).map_err(|e|e.to_string())).collect()
}
pub fn hello_bytes()->Result<Vec<u8>,String> {
    include_str!("../programs/hello.hex").split_whitespace().map(|s|u8::from_str_radix(s,16).map_err(|e|e.to_string())).collect()
}
// Original demonstration image: aliases, preserved flags, LEA and equal-width moves.
pub fn extension_bytes()->Vec<u8>{vec![
    0x66,0xb8,0x80,0,0,0, // EAX=80
    0x66,0x0f,0xb6,0xd8, // MOVZX EBX,AL
    0x66,0x0f,0xbe,0xc8, // MOVSX ECX,AL
    0x98,0x66,0x98,0x66,0x99, // CBW,CWDE,CDQ
    0xbf,0,0,0x8d,0x71,5, // DI=0; LEA SI,[BX+DI+5]
    0x0f,0xbf,0xf8, // MOVSX DI,AX
    0x0f,0xb7,0xef, // MOVZX BP,DI
    0x0f,0xbf,0xff,0x0f,0xb7,0xff, // DI self aliases
    0x89,0x07, // MOV word DS:[BX],AX -> linear180
    0x0f,0xbf,0x2f,0x0f,0xb7,0x27, // word memory sources -> BP,SP
    0xf4,
]}
pub const LOOP_ITERATIONS:u32=65536;
pub const FACTORIALS:[u32;12]=[1,2,6,24,120,720,5040,40320,362880,3628800,39916800,479001600];
pub fn factorial_bytes()->Result<Vec<u8>,String> {
    include_str!("../programs/factorials.hex").split_whitespace().map(|s|u8::from_str_radix(s,16).map_err(|e|e.to_string())).collect()
}
pub fn loop_bytes(iterations:u32)->Result<Vec<u8>,String> {
    if !(1..=100000).contains(&iterations){return Err("loop iterations must be 1..100000".into());}
    let mut bytes:Vec<u8>=include_str!("../programs/loop.hex").split_whitespace().map(|s|u8::from_str_radix(s,16).map_err(|e|e.to_string())).collect::<Result<_,_>>()?;
    bytes[2..6].copy_from_slice(&iterations.to_le_bytes());Ok(bytes)
}
const CONDITIONS:[&str;16]=["o","no","b","ae","e","ne","be","a","s","ns","p","np","l","ge","le","g"];
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum GuestError { Decode(String), Execution(ExecError) }
impl From<ExecError> for GuestError {fn from(e:ExecError)->Self{Self::Execution(e)}}
#[derive(Clone,Debug)]
pub enum Operation {
    Pil{program:Program,dst:View,src:View,flow:FlowContext,address:Option<AddressContext>},
    Input{next:u32}, Output{next:u32}, Halt,
    /// INT n, INT3 or INTO (`overflow`): a software interrupt to `vector`
    /// that returns to `next`. Delivery belongs to the runner (interrupt.rs).
    Interrupt{vector:u8,overflow:bool,next:u32},
}
#[derive(Clone,Debug)]
pub struct Decoded {pub label:String,pub length:u32,pub operation:Operation}
pub const SYSTEM_RANGES:usize=10;
#[derive(Clone,Debug,PartialEq,Eq)]
/// `fetch_base+start..fetch_base+end` is the program; `system` holds host-provided
/// code ranges (such as Dine's case-map routine and interrupt entries) and waiting
/// parent programs in EXEC that may run interrupt handlers.
pub struct Image {pub fetch_base:u32,pub start:u32,pub end:u32,pub system:[(u32,u32);SYSTEM_RANGES],pub segments:[SegmentCache;6],pub selectors:[u16;6],pub msw:u16,pub real_mode:bool,pub ss_shadow:u8,pub sti_shadow:u8,pub tables:[(u16,u32);2],pub controls:[u32;2],pub flag_checkpoint:Option<(u32,u32,u32)>,pub ldt_tr:[crate::address::SystemSegment;2],pub paging:bool}
impl Image {
    // Keep the imported physical code window fixed when CS changes.
    pub fn contains_offset(&self,offset:u32)->bool{
        let at=u64::from(self.segments[1].base)+u64::from(offset);
        self.contains_linear(at)
    }
    fn contains_linear(&self,at:u64)->bool{
        (at>=u64::from(self.fetch_base)+u64::from(self.start)&&at<u64::from(self.fetch_base)+u64::from(self.end))
            ||self.system.iter().any(|&(s,e)|at>=u64::from(s)&&at<u64::from(e))
    }
    pub fn retired(&mut self){let mut s=self.segment_state();s.retired();self.set_segment_state(s);}
    pub fn segment_state(&self)->SegmentState{SegmentState{controls:self.controls,msw:self.msw,segments:self.segments,selectors:self.selectors,real_mode:self.real_mode,ss_shadow:self.ss_shadow,sti_shadow:self.sti_shadow,tables:self.tables,flag_checkpoint:self.flag_checkpoint,ldt_tr:self.ldt_tr,paging:self.paging}}
    pub fn set_segment_state(&mut self,s:SegmentState){self.controls=s.controls;self.tables=s.tables;self.msw=s.msw;self.segments=s.segments;self.selectors=s.selectors;self.real_mode=s.real_mode;self.ss_shadow=s.ss_shadow;self.sti_shadow=s.sti_shadow;self.flag_checkpoint=s.flag_checkpoint;self.ldt_tr=s.ldt_tr;self.paging=s.paging;}

    pub fn load(bytes:&[u8])->Result<(Self,Ram),String> {
        if bytes.is_empty() || bytes.len()>RAM_BYTES-ENTRY as usize {return Err("image must contain 1..3840 bytes".into());}
        let mut ram=Ram::new(RAM_BYTES);ram.write_bytes(ENTRY,bytes).map_err(|e|format!("{e:?}"))?;
        let mut segments=[SegmentCache::real_mode(0);6];
        // Ordinary real-mode CS permits data reads and writes too.
        // Protected-mode callers supply Code caches with their own permissions.
        segments[SegmentRegister::Ds.index()]=SegmentCache::real_mode(0x10);
        segments[SegmentRegister::Ss.index()]=SegmentCache::real_mode(0x20);
        segments[SegmentRegister::Fs.index()]=SegmentCache::real_mode(0x30);
        Ok((Self{fetch_base:0,start:ENTRY,end:ENTRY+bytes.len() as u32,system:[(0,0);SYSTEM_RANGES],segments,selectors:[0,0,0x20,0x10,0x30,0],msw:0x10,real_mode:true,ss_shadow:0,sti_shadow:0,tables:[(0,0),crate::interrupt::RESET_IDTR],controls:[0;2],flag_checkpoint:None,ldt_tr:Default::default(),paging:false},ram))
    }
    pub fn fetch_signature(&self,ram:&Ram,eip:u32,length:u32)->Result<[u8;15],GuestError> {
        if length==0 || length>15{return Err(GuestError::Decode("invalid cached instruction length".into()));}
        let cs=self.segments[SegmentRegister::Cs.index()];
        let last_off=u64::from(eip)+u64::from(length)-1;
        let lin_start=u64::from(cs.base)+u64::from(eip);
        let lin_end=lin_start+u64::from(length);
        let physical=if lin_start<=u64::from(u32::MAX){u64::from(ram.physical_address(lin_start as u32))}else{lin_start};
        let physical_end=physical+u64::from(length);
        let img_start=u64::from(self.fetch_base)+u64::from(self.start);
        let img_end=u64::from(self.fetch_base)+u64::from(self.end);
        let readable=match cs.access {
            SegmentAccess::Data{expand_down:false,..}=>true,
            SegmentAccess::Code{readable,..}=>readable,
            _=>false,
        };
        if readable && last_off<=u64::from(cs.limit) && ((physical>=img_start && physical_end<=img_end)||self.system.iter().any(|&(s,e)|physical>=u64::from(s)&&physical_end<=u64::from(e))) && lin_end<=0x1_0000_0000 {
            if let Some(slice)=ram.contiguous_slice(lin_start as u32,length as usize) {
                let mut bytes=[0u8;15];
                bytes[..length as usize].copy_from_slice(slice);
                return Ok(bytes);
            }
        }
        let mut reader=Reader{image:self,ram,start:eip,pos:eip};let mut bytes=[0u8;15];
        for byte in &mut bytes[..length as usize] {*byte=reader.byte()?;}
        Ok(bytes)
    }
    pub fn decode(&self,ram:&Ram,eip:u32)->Result<Decoded,GuestError> {
        let mut r=Reader{image:self,ram,start:eip,pos:eip};
        // A code segment with the D bit set defaults to 32-bit operands and
        // addresses; 66h and 67h select the other size.
        let big=self.segments[SegmentRegister::Cs.index()].default_32();
        let mut wide=big;let mut addr32=big;let mut override_segment=None;let mut repeat=None;let mut locked=false;
        let mut opcode=loop {
            let b=r.byte()?;
            match b {
                // The 386 accepts repeated and multiple prefixes: the last
                // segment override and the last repeat prefix take effect.
                0xf2|0xf3=>repeat=Some(b),
                0xf0=>locked=true,
                0x66=>wide=!big,0x67=>addr32=!big,
                0x26|0x2e|0x36|0x3e|0x64|0x65=>override_segment=Some(match b{0x26=>SegmentRegister::Es,0x2e=>SegmentRegister::Cs,0x36=>SegmentRegister::Ss,0x3e=>SegmentRegister::Ds,0x64=>SegmentRegister::Fs,_=>SegmentRegister::Gs}),
                _=>break b,
            }
        };
        // On the 386, F2 with MOVS, STOS, LODS, INS and OUTS repeats by count
        // like F3 and does not test ZF (SingleStepTests/80386 confirms it).
        if repeat.is_some()&&!matches!(opcode,0x6c..=0x6f|0xa4..=0xa7|0xaa..=0xaf){return Err(GuestError::Decode("unsupported repeat-prefix/instruction combination".into()));}
        // LOCK is valid only on a read-modify-write memory destination; the
        // single-executor RAM model makes every instruction indivisible, so it
        // changes nothing else. XCHG checks its memory operand itself.
        if locked&&!matches!(opcode,0x86|0x87){
            let mut peek=Reader{image:self,ram,start:eip,pos:r.pos};
            let (op,m)=if opcode==0x0f{let second=peek.byte()?;(0x0f00|u16::from(second),peek.byte()?)}else{(u16::from(opcode),peek.byte()?)};
            let ext=(m>>3)&7;
            let lockable=m>>6!=3&&match op{
                0x00..=0x31=>op&6==0&&op&0x38!=0x38,
                0x80..=0x83=>ext!=7,0xf6|0xf7=>matches!(ext,2|3),0xfe|0xff=>ext<2,
                0x0fab|0x0fb3|0x0fbb=>true,0x0fba=>ext>=5,_=>false,
            };
            if !lockable{return Err(GuestError::Decode("LOCK requires a read-modify-write memory destination".into()));}
        }
        // Normalize alternative register encodings while retaining consumed bytes.
        let mut push_selector=None;let mut pop_selector=None;let mut call_indirect=None;let mut memory_pop=None;let mut memory_push=None;let mut far_indirect=None;let mut jump_indirect=None;let mut far_jump=None;
        if matches!(opcode,0xff|0x8f){
            let m=r.byte()?;let extension=(m>>3)&7;
            if opcode==0xff&&extension==2{call_indirect=Some(m);opcode=0xe8;}
            else if opcode==0xff&&extension==4{jump_indirect=Some(m);}
            else if opcode==0xff&&extension==5{if m>>6==3{return Err(GuestError::Decode("far JMP requires a memory pointer".into()));}far_jump=Some(m);opcode=0xea;}
            else if opcode==0xff&&extension==3{if m>>6==3{return Err(GuestError::Decode("far CALL requires a memory pointer".into()));}far_indirect=Some(m);opcode=0x9a;}
            else if opcode==0x8f || extension==6{
                if opcode==0x8f && extension!=0{return Err(GuestError::Decode("unsupported POP group extension".into()));}
                if m>>6!=3{
                    if opcode==0x8f{memory_pop=Some(m);}else{memory_push=Some(m);}
                }else{opcode=(if opcode==0x8f{0x58}else{0x50})|(m&7);}
            }else{r.pos-=1;}
        }
        if opcode==0x0f{
            let second=r.byte()?;
            if matches!(second,0xa1|0xa9){pop_selector=Some(if second==0xa1{SegmentRegister::Fs}else{SegmentRegister::Gs});opcode=0x07;}
            else if matches!(second,0xa0|0xa8){push_selector=Some(if second==0xa0{SegmentRegister::Fs}else{SegmentRegister::Gs});opcode=0x06;}
            else{r.pos-=1;}
        }
        let mut w=if wide {32}else{16};let mut dst=view(0,w);let mut src=dst;let mut address=None;let mut disp=0;let mut disp_bits=8;
        let (label,program)=match opcode {
            0xff if jump_indirect.is_some()=>{
                let m=jump_indirect.unwrap();dst=view((m&7)as usize,w);src=dst;
                let (target,label)=if m>>6!=3{let effective=r.effective(m,addr32)?;address=Some(AddressContext{effective,override_segment,segments:self.segments});(format!("read_address operand\nload target_value operand {w}"),operand_name(dst,address))}
                    else{(format!("read target_value dst {w}"),register_name(dst).into())};
                let text=include_str!("../isa/i386/jmp_indirect.pil").replace("$TARGET",&target);
                (format!("JMP {label}"),compile(&instantiate(text,w),w))
            }
            0xff if memory_push.is_some()=>{
                let m=memory_push.unwrap();
                let effective=r.effective(m,addr32)?;address=Some(AddressContext{effective,override_segment,segments:self.segments});
                let stack_bits=if matches!(self.segments[2].access,SegmentAccess::Data{big:true,..}){32}else{16};
                let text=description("push_memory").map_err(GuestError::Decode)?.replace("$S",&stack_bits.to_string()).replace("$BYTES",&(w/8).to_string());
                (format!("PUSH {}",operand_name(dst,address)),compile(&instantiate(&text,w),w))
            }
            0x8f=>{
                let m=memory_pop.ok_or_else(||GuestError::Decode("invalid memory POP".into()))?;
                let effective=r.effective(m,addr32)?;address=Some(AddressContext{effective,override_segment,segments:self.segments});
                let stack_bits=if matches!(self.segments[2].access,SegmentAccess::Data{big:true,..}){32}else{16};
                let esp_based=matches!(effective,EffectiveAddress::Bits32{base:Some(Gpr::Esp),..});
                // Intel documents this wrap case as processor-family-specific.
                // Reject it until a legacy386 hardware vector fixes the policy.
                let guard=if stack_bits==16&&esp_based{"ult wrapped new_sp old_sp\nnot allowed wrapped\nrequire allowed"}else{""};
                let text=include_str!("../isa/i386/pop_memory.pil").replace("$S",&stack_bits.to_string()).replace("$BYTES",&(w/8).to_string()).replace("$HIGH_MASK",if stack_bits==16{"4294901760"}else{"0"}).replace("$WRAP_GUARD",guard);
                (format!("POP {}",operand_name(dst,address)),compile(&instantiate(text,w),w))
            }
            0xc8=>{
                let size=r.unsigned(16)?;let level=r.byte()?&31;
                let stack_bits:u8=if matches!(self.segments[2].access,SegmentAccess::Data{big:true,..}){32}else{16};
                let n=u32::from(w/8);let level32=u32::from(level);
                // Level L copies L-1 enclosing frame pointers, then pushes the new one.
                // Each pointer is read after the earlier pushes are staged, as the
                // 386 does, so a chain that overlaps the new frame reads them.
                let mut stores=String::new();
                for k in 1..level32{
                    stores+=&format!("const back{k} $S {}\nsub link{k} chain_bp back{k}\nzext link_at{k} link{k} 32\nload_segment copy{k} 2 link_at{k} $W\n",k*n);
                    stores+=&format!("const down{k} $S {}\nsub slot{k} frame down{k}\nzext slot_at{k} slot{k} 32\nstore_segment 2 slot_at{k} copy{k}\n",k*n);
                }
                let frame_value=match w.cmp(&stack_bits){std::cmp::Ordering::Greater=>"zext frame_value frame $W",std::cmp::Ordering::Less=>"trunc frame_value frame $W",_=>"zext frame_value frame $W"};
                if level32>0{stores+=&format!("{frame_value}\nconst down_last $S {}\nsub last frame down_last\nzext last_at last 32\nstore_segment 2 last_at frame_value\n",level32*n);}
                let pointer=if w>stack_bits{"zext frame_pointer frame 32"}else{"zext frame_pointer frame $S"};
                let text=include_str!("../isa/i386/enter.pil").replace("$COPY_STORES",stores.trim_end()).replace("$FRAME_POINTER",pointer).replace("$PUSHED",&(level32*n).to_string()).replace("$SIZE",&size.to_string()).replace("$S",&stack_bits.to_string()).replace("$BYTES",&(w/8).to_string());
                (format!("ENTER 0x{size:04x},{level}"),compile(&instantiate(&text,w),w))
            }
            0xd7=>{
                let seg=override_segment.unwrap_or(SegmentRegister::Ds).index();
                let text=include_str!("../isa/i386/xlat.pil").replace("$A",if addr32{"32"}else{"16"}).replace("$SEG",&seg.to_string());
                ("XLAT".into(),compile(&text,w))
            }
            0xc9=>{
                let stack_bits=if matches!(self.segments[2].access,SegmentAccess::Data{big:true,..}){32}else{16};
                let text=description("leave").map_err(GuestError::Decode)?.replace("$S",&stack_bits.to_string()).replace("$BYTES",&(w/8).to_string());
                ("LEAVE".into(),compile(&instantiate(text,w),w))
            }
            0xe0..=0xe2=>{
                disp=r.byte()? as i8 as i32;
                let (name,condition)=match opcode{
                    0xe0=>("LOOPNE","read_flag z ZF\nnot nz z\nand take more nz"),
                    0xe1=>("LOOPE","read_flag z ZF\nand take more z"),
                    _=>("LOOP","or take more more"),
                };
                let text=include_str!("../isa/i386/loop.pil").replace("$A",if addr32{"32"}else{"16"}).replace("$CONDITION",condition);
                (format!("{name} 0x{:08x}",r.pos.wrapping_add(disp as u32)&super::mask(w)as u32),compile(&instantiate(text,w),w))
            }
            0xe3=>{
                disp=r.byte()? as i8 as i32;
                let name=if addr32{"JECXZ"}else{"JCXZ"};
                let text=include_str!("../isa/i386/jcxz.pil").replace("$A",if addr32{"32"}else{"16"});
                (format!("{name} 0x{:08x}",r.pos.wrapping_add(disp as u32)&super::mask(w)as u32),compile(&instantiate(text,w),w))
            }
            0x9c|0x9d=>{
                let stack_bits=if matches!(self.segments[2].access,SegmentAccess::Data{big:true,..}){32}else{16};
                let name=if opcode==0x9c{"pushf"}else{"popf"};
                let text=description(name).map_err(GuestError::Decode)?.replace("$S",&stack_bits.to_string()).replace("$BYTES",&(w/8).to_string());
                (format!("{}{}",name.to_ascii_uppercase(),if w==32{"D"}else{""}),compile(&instantiate(text,w),w))
            }
            0xfa|0xfb=>{
                let name=if opcode==0xfa{"cli"}else{"sti"};
                (name.to_ascii_uppercase(),compile(&description(name).map_err(GuestError::Decode)?,w))
            }
            0xea=>{
                let (target,label)=if let Some(m)=far_jump{
                    let effective=r.effective(m,addr32)?;address=Some(AddressContext{effective,override_segment,segments:self.segments});
                    let seg=effective.resolve(&[0;8],override_segment)?.segment.index();
                    (far_pointer(seg,w,addr32),operand_name(dst,address))
                }else{
                    let offset=r.unsigned(w)?;let selector=r.unsigned(16)?;
                    (format!("const target_value {w} {offset}\nconst selector 16 {selector}"),format!("{selector:04x}:{offset:08x}"))
                };
                let text=include_str!("../isa/i386/jmp_far.pil").replace("$FAR_TARGET",&target);
                (format!("JMP FAR {label}"),compile(&instantiate(&text,w),w))
            }
            0x9a=>{
                let stack_bits=if matches!(self.segments[2].access,SegmentAccess::Data{big:true,..}){32}else{16};
                let (target,label)=if let Some(m)=far_indirect{
                    let effective=r.effective(m,addr32)?;address=Some(AddressContext{effective,override_segment,segments:self.segments});
                    let seg=effective.resolve(&[0;8],override_segment)?.segment.index();
                    (far_pointer(seg,w,addr32),operand_name(dst,address))
                }else{
                    let offset=r.unsigned(w)?;let selector=r.unsigned(16)?;
                    (format!("const target_value {w} {offset}\nconst selector 16 {selector}"),format!("{selector:04x}:{offset:08x}"))
                };
                let text=description("call_far").map_err(GuestError::Decode)?.replace("$FAR_TARGET",&target).replace("$S",&stack_bits.to_string()).replace("$BYTES",&(w/8).to_string()).replace("$FRAME",&(w/4).to_string());
                (format!("CALL FAR {label}"),compile(&instantiate(text,w),w))
            }
            0xe8=>{
                let stack_bits=if matches!(self.segments[2].access,SegmentAccess::Data{big:true,..}){32}else{16};
                let (target,label)=if let Some(m)=call_indirect{
                    dst=view((m&7)as usize,w);src=dst;
                    if m>>6!=3{let effective=r.effective(m,addr32)?;address=Some(AddressContext{effective,override_segment,segments:self.segments});(format!("read_address operand\nload target_value operand {w}"),operand_name(dst,address))}
                    else{(format!("read target_value dst {w}"),register_name(dst).into())}
                }else{
                    disp_bits=w;let raw=r.unsigned(w)?;disp=if w==16{raw as i16 as i32}else{raw as i32};
                    (format!("read_disp delta {w}\nadd target_value return_offset delta"),format!("0x{:08x}",r.pos.wrapping_add(disp as u32)&super::mask(w)as u32))
                };
                let text=include_str!("../isa/i386/call.pil").replace("$TARGET",&target).replace("$S",&stack_bits.to_string()).replace("$BYTES",&(w/8).to_string());
                (format!("CALL {label}"),compile(&instantiate(text,w),w))
            }
            0xca|0xcb=>{
                let cleanup=if opcode==0xca{r.unsigned(16)?}else{0};let stack_bits=if matches!(self.segments[2].access,SegmentAccess::Data{big:true,..}){32}else{16};
                let text=include_str!("../isa/i386/retf.pil").replace("$CLEANUP",&cleanup.to_string()).replace("$S",&stack_bits.to_string()).replace("$BYTES",&(w/8).to_string()).replace("$FRAME",&(w/4).to_string());
                (if opcode==0xca{format!("RETF 0x{cleanup:04x}")}else{"RETF".into()},compile(&instantiate(text,w),w))
            }
            0xcf=>{
                let stack_bits=if matches!(self.segments[2].access,SegmentAccess::Data{big:true,..}){32}else{16};
                // VM sits in bit 17, so only IRETD's flags dword can carry it.
                let vm_check=if w==32{"bit vm packed 17\nand vm_return vm ring0"}else{"const vm_return 1 0"};
                let text=description(if self.real_mode||self.segment_state().v86(){"iret"}else{"iret_protected"}).map_err(GuestError::Decode)?.replace("$VM_CHECK",vm_check).replace("$S",&stack_bits.to_string()).replace("$BYTES",&(w/8).to_string()).replace("$FRAME",&(w/8*3).to_string());
                (if w==32{"IRETD".into()}else{"IRET".into()},compile(&instantiate(text,w),w))
            }
            0xc2|0xc3=>{
                let cleanup=if opcode==0xc2{r.unsigned(16)?}else{0};let stack_bits=if matches!(self.segments[2].access,SegmentAccess::Data{big:true,..}){32}else{16};
                let text=include_str!("../isa/i386/ret.pil").replace("$CLEANUP",&cleanup.to_string()).replace("$S",&stack_bits.to_string()).replace("$BYTES",&(w/8).to_string());
                (if opcode==0xc2{format!("RET 0x{cleanup:04x}")}else{"RET".into()},compile(&instantiate(text,w),w))
            }
            0x6c..=0x6f|0xa4|0xa5|0xaa|0xab|0xac|0xad=>{
                if opcode&1==0{w=8;}dst=view(0,w);src=dst;let a=if addr32{32}else{16};
                let name=match opcode{0x6c|0x6d=>"ins",0x6e|0x6f=>"outs",0xa4|0xa5=>"movs",0xaa|0xab=>"stos",_=>"lods"};
                let body=match name{"ins"=>include_str!("../isa/i386/ins.pil"),"outs"=>include_str!("../isa/i386/outs.pil"),"movs"=>include_str!("../isa/i386/movs.pil"),"stos"=>include_str!("../isa/i386/stos.pil"),_=>include_str!("../isa/i386/lods.pil")};
                let body=body.replace("$SEG",&override_segment.unwrap_or(SegmentRegister::Ds).index().to_string()).replace("$A",&a.to_string()).replace("$BYTES",&(w/8).to_string());
                let text=if repeat.is_some(){super::repeat_description(&body,a)}else{body};
                let mnemonic=format!("{}{}",name.to_ascii_uppercase(),match w{8=>"B",16=>"W",_=>"D"});
                (if repeat.is_some(){format!("REP {mnemonic}")}else{mnemonic},compile(&instantiate(text,w),w))
            }
            0xa6|0xa7|0xae|0xaf=>{
                if opcode&1==0{w=8;}dst=view(0,w);src=dst;let a=if addr32{32}else{16};let name=if opcode<0xae{"cmps"}else{"scas"};
                let body=description(name).map_err(GuestError::Decode)?.replace("$SEG",&override_segment.unwrap_or(SegmentRegister::Ds).index().to_string()).replace("$A",&a.to_string()).replace("$BYTES",&(w/8).to_string());
                let text=if let Some(prefix)=repeat{super::compare_repeat_description(&body,a,prefix==0xf3)}else{body};
                let mnemonic=format!("{}{}",name.to_ascii_uppercase(),match w{8=>"B",16=>"W",_=>"D"});
                (if let Some(prefix)=repeat{format!("{} {mnemonic}",if prefix==0xf3{"REPE"}else{"REPNE"})}else{mnemonic},compile(&instantiate(text,w),w))
            }
            0xa0..=0xa3=>{
                if opcode&1==0{w=8;}dst=view(0,w);src=dst;
                let offset=r.unsigned(if addr32{32}else{16})?;
                let effective=if addr32{EffectiveAddress::Bits32{base:None,index:None,scale:1,displacement:offset}}else{EffectiveAddress::Bits16{form:Address16::Direct,displacement:offset as u16}};
                address=Some(AddressContext{effective,override_segment,segments:self.segments});
                let load=opcode&2==0;let text=instantiate(description(if load{"load_ram"}else{"store_ram"}).map_err(GuestError::Decode)?,w);
                let operand=operand_name(dst,address);
                (if load{format!("MOV {},{operand}",register_name(dst))}else{format!("MOV {operand},{}",register_name(src))},compile(&text,w))
            }
            0xc6|0xc7=>{
                if opcode==0xc6{w=8;}
                let m=r.byte()?;if (m>>3)&7!=0{return Err(GuestError::Decode("MOV immediate requires group extension0".into()));}
                dst=view((m&7)as usize,w);src=dst;
                let memory=m>>6!=3;
                if memory{let effective=r.effective(m,addr32)?;address=Some(AddressContext{effective,override_segment,segments:self.segments});}
                let immediate=r.unsigned(w)?;
                let text=if memory{format!("read_address addr\nconst v {w} {immediate}\nstore addr v\npreserve_flags all\nretire")}else{format!("const v {w} {immediate}\nwrite dst v\npreserve_flags all\nretire")};
                (format!("MOV {},0x{immediate:0digits$x}",operand_name(dst,address),digits=(w/4)as usize),compile(&text,w))
            }
            0x63=>{
                // ARPL r/m16,r16: #UD in real mode.
                if self.real_mode||self.segment_state().v86(){return Err(GuestError::Decode("ARPL (63h) is #UD in real and virtual-8086 mode".into()));}
                let m=r.byte()?;dst=view((m&7)as usize,16);src=view(((m>>3)&7)as usize,16);
                let (input,output)=if m>>6!=3{address=Some(AddressContext{effective:r.effective(m,addr32)?,override_segment,segments:self.segments});("read_address destination\nload selector destination 16","store_if destination result raise")}else{("read selector dst 16","write dst result")};
                let text=include_str!("../isa/i386/arpl.pil").replace("$INPUT",input).replace("$OUTPUT",output);
                (format!("ARPL {},{}",operand_name(dst,address),register_name(src)),compile(&text,16))
            }
            0x62=>{
                let m=r.byte()?;if m>>6==3{return Err(GuestError::Decode("BOUND requires a memory bounds pair".into()));}
                dst=view(((m>>3)&7)as usize,w);src=dst;
                address=Some(AddressContext{effective:r.effective(m,addr32)?,override_segment,segments:self.segments});
                (format!("BOUND {},{}",register_name(dst),operand_name(src,address)),compile(&instantiate(include_str!("../isa/i386/bound.pil").replace("$OFFSET_MASK",if addr32{"4294967295"}else{"65535"}),w),w))
            }
            0xc4|0xc5=>{
                let name=if opcode==0xc4{"LES"}else{"LDS"};
                let m=r.byte()?;if m>>6==3{return Err(GuestError::Decode(format!("{name} requires a memory far pointer")));}
                dst=view(((m>>3)&7)as usize,w);src=dst;
                let effective=r.effective(m,addr32)?;
                address=Some(AddressContext{effective,override_segment,segments:self.segments});
                let text=include_str!("../isa/i386/les.pil").replace("$SEG",&effective.resolve(&[0;8],override_segment)?.segment.index().to_string()).replace("$OFFSET_MASK",if addr32{"4294967295"}else{"65535"}).replace("$BYTES",&(w/8).to_string()).replace("$DEST",if opcode==0xc4{"0"}else{"3"});
                (format!("{name} {},{}",register_name(dst),operand_name(src,address)),compile(&instantiate(text,w),w))
            }
            0x8c|0x8e=>{
                let m=r.byte()?;let segment=((m>>3)&7)as usize;
                if segment>=6 || (opcode==0x8e&&segment==1){return Err(GuestError::Decode("invalid segment MOV encoding".into()));}
                let reg_dest_32 = opcode==0x8c && wide && m>>6==3;
                w = if reg_dest_32 { 32 } else { 16 };
                dst = view((m&7)as usize, w);
                src = dst;
                let seg=["ES","CS","SS","DS","FS","GS"][segment];
                let mut text=if opcode==0x8c{
                    if reg_dest_32 {
                        format!("read_segment selector {segment}\nzext selector_32 selector 32\nwrite dst selector_32\npreserve_flags all\nretire")
                    } else {
                        format!("read_segment selector {segment}\nwrite dst selector\npreserve_flags all\nretire")
                    }
                } else {
                    format!("read selector src 16\nwrite_segment {segment} selector\npreserve_flags all\nretire")
                };
                if m>>6!=3{let effective=r.effective(m,addr32)?;address=Some(AddressContext{effective,override_segment,segments:self.segments});
                    text=if opcode==0x8c{text.replace("write dst selector","read_address addr\nstore addr selector")}else{text.replace("read selector src 16","read_address addr\nload selector addr 16")};}
                let operand=operand_name(dst,address);
                (if opcode==0x8c{format!("MOV {operand},{seg}")}else{format!("MOV {seg},{operand}")},compile(&text,w))
            }
            0x07|0x17|0x1f=>{
                let segment=pop_selector.unwrap_or(match opcode{0x07=>SegmentRegister::Es,0x17=>SegmentRegister::Ss,_=>SegmentRegister::Ds});
                let stack_bits=if matches!(self.segments[2].access,SegmentAccess::Data{big:true,..}){32}else{16};
                address=Some(AddressContext{effective:EffectiveAddress::Bits16{form:Address16::Direct,displacement:0},override_segment:Some(SegmentRegister::Ss),segments:self.segments});
                let text=include_str!("../isa/i386/pop_segment.pil").replace("$SEG",&segment.index().to_string()).replace("$S",&stack_bits.to_string()).replace("$BYTES",&(w/8).to_string());
                (format!("POP {segment:?}").to_ascii_uppercase(),compile(&instantiate(text,w),w))
            }
            0x50..=0x5f|0x68|0x6a|0x06|0x0e|0x16|0x1e=>{
                let stack_bits=match self.segments[SegmentRegister::Ss.index()].access {
                    SegmentAccess::Data{big:true,..}=>32,_=>16,
                };
                // Stack addresses always use SS, independent of 67 or overrides.
                address=Some(AddressContext{effective:EffectiveAddress::Bits16{form:Address16::Direct,displacement:0},override_segment:Some(SegmentRegister::Ss),segments:self.segments});
                let pop=(0x58..=0x5f).contains(&opcode);let reg=(opcode&7)as usize;
                let name=if pop{"pop"}else{"push"};
                let mut text=description(name).map_err(GuestError::Decode)?
                    .replace("$S",&stack_bits.to_string()).replace("$BYTES",&(w/8).to_string()).replace("$REG",&reg.to_string());
                let operand=if (0x50..=0x5f).contains(&opcode){dst=view(reg,w);register_name(dst).to_string()}
                else {
                    let (v,label)=if opcode==0x68{let v=r.unsigned(w)?;(v,format!("0x{v:x}"))}
                    else if opcode==0x6a{let v=r.byte()? as i8 as i32 as u32 & super::mask(w)as u32;(v,format!("0x{v:x}"))}
                    else {let seg=push_selector.unwrap_or(match opcode{0x06=>SegmentRegister::Es,0x0e=>SegmentRegister::Cs,0x16=>SegmentRegister::Ss,_=>SegmentRegister::Ds});(u32::from(self.selectors[seg.index()]),format!("{seg:?}").to_ascii_uppercase())};
                    if matches!(opcode,0x06|0x0e|0x16|0x1e){let seg=push_selector.unwrap_or(match opcode{0x06=>SegmentRegister::Es,0x0e=>SegmentRegister::Cs,0x16=>SegmentRegister::Ss,_=>SegmentRegister::Ds});text=text.replace("read data dst $W",&format!("read_segment selector {}\nzext data selector $W",seg.index()));}
                    else{text=text.replace("read data dst $W",&format!("const data $W {v}"));}label
                };
                (format!("{} {operand}",name.to_ascii_uppercase()),compile(&instantiate(text,w),w))
            }
            0x60|0x61=>{
                let stack_bits=if matches!(self.segments[SegmentRegister::Ss.index()].access,SegmentAccess::Data{big:true,..}){32}else{16};
                address=Some(AddressContext{effective:EffectiveAddress::Bits16{form:Address16::Direct,displacement:0},override_segment:Some(SegmentRegister::Ss),segments:self.segments});
                let file=if opcode==0x60{include_str!("../isa/i386/pusha.pil")}else{include_str!("../isa/i386/popa.pil")};
                let (keep,take)=match (stack_bits,w){(16,32)=>("0","4294901760"),(16,_)=>("4294901760","0"),_=>("0","0")};
                let text=file.replace("$KEEP_ESP",keep).replace("$TAKE_ESP",take).replace("$S",&stack_bits.to_string()).replace("$BYTES",&(w/8).to_string());
                (format!("{}{}",if opcode==0x60{"PUSHA"}else{"POPA"},if w==32{"D"}else{""}),compile(&instantiate(text,w),w))
            }
            0xb0..=0xbf=>{
                if opcode<0xb8 {w=8;}
                dst=view((opcode&7) as usize,w);src=dst;
                let value=r.unsigned(w)?;
                (format!("MOV {},0x{value:0digits$x}",register_name(dst),digits=(w/4) as usize),compile(&format!("const v {w} {value}\nwrite dst v\npreserve_flags all\nretire"),w))
            }
            0x70..=0x7f=>{
                disp=r.byte()? as i8 as i32;
                let name=format!("j{}",CONDITIONS[(opcode&15) as usize]);
                (format!("{} 0x{:08x}",name.to_ascii_uppercase(),r.pos.wrapping_add(disp as u32)&super::mask(w) as u32),compile(&instantiate(description(&name).map_err(GuestError::Decode)?,w),w))
            }
            0xeb|0xe9=>{
                disp_bits=if opcode==0xeb{8}else{w};
                let raw=r.unsigned(disp_bits)?;disp=match disp_bits{8=>raw as i8 as i32,16=>raw as i16 as i32,_=>raw as i32};
                (format!("JMP 0x{:08x}",r.pos.wrapping_add(disp as u32)&super::mask(w) as u32),compile(&instantiate(description("jmp").map_err(GuestError::Decode)?,w),w))
            }
            0x0f=>{
                let second=r.byte()?;
                if matches!(second,0x20|0x22){
                    let m=r.byte()?;let control=((m>>3)&7)as usize;
                    if m>>6!=3||!matches!(control,0|2|3){return Err(GuestError::Decode("MOV control requires register operands and CR0, CR2 or CR3".into()));}
                    dst=view((m&7)as usize,32);src=dst;
                    let file=if second==0x20{include_str!("../isa/i386/read_control.pil")}else{include_str!("../isa/i386/write_control.pil")};
                    let text=file.replace("$CR",&control.to_string());
                    (if second==0x20{format!("MOV {},CR{control}",register_name(dst))}else{format!("MOV CR{control},{}",register_name(src))},compile(&text,32))
                }else if second==0x01&&{let mut p=Reader{image:self,ram,start:eip,pos:r.pos};matches!((p.byte()?>>3)&7,0|1)}{
                    let m=r.byte()?;let table=(m>>3)&7;
                    if m>>6==3{return Err(GuestError::Decode("SGDT/SIDT require memory".into()));}
                    let effective=r.effective(m,addr32)?;
                    let seg=effective.resolve(&[0;8],override_segment)?.segment.index();
                    address=Some(AddressContext{effective,override_segment,segments:self.segments});
                    let text=include_str!("../isa/i386/store_table.pil").replace("$SEG",&seg.to_string()).replace("$TABLE",&table.to_string());
                    (format!("{} {}",if table==0{"SGDT"}else{"SIDT"},operand_name(view(0,w),address).split_once(' ').map_or(String::new(),|(_,a)|a.to_string())),compile(&text,w))
                }else if second==0x00{
                    // SLDT, STR, LLDT, LTR, VERR and VERW are #UD in real mode.
                    let m=r.byte()?;let ext=(m>>3)&7;
                    if self.real_mode||self.segment_state().v86(){return Err(GuestError::Decode("0F00 group (SLDT/STR/LLDT/LTR/VERR/VERW) is #UD in real and virtual-8086 mode".into()));}
                    if ext>5{return Err(GuestError::Decode("unsupported 0F00 group extension".into()));}
                    let name=["SLDT","STR","LLDT","LTR","VERR","VERW"][ext as usize];
                    let memory=m>>6!=3;
                    if memory{address=Some(AddressContext{effective:r.effective(m,addr32)?,override_segment,segments:self.segments});}
                    let input=if memory{"read_address source\nload selector source 16"}else{"read selector src 16"};
                    if ext<2{
                        // A memory destination is always a word.
                        if memory||!wide{w=16;}
                        dst=view((m&7)as usize,w);src=dst;
                        let output=if memory{"read_address destination\nstore destination selector".to_string()}else if w==32{"zext whole selector 32\nwrite dst whole".to_string()}else{"write dst selector".to_string()};
                        let text=include_str!("../isa/i386/store_system.pil").replace("$WHICH",&ext.to_string()).replace("$OUTPUT",&output);
                        (format!("{name} {}",operand_name(dst,address)),compile(&text,w))
                    }else{
                        dst=view((m&7)as usize,16);src=dst;
                        let text=if ext<4{include_str!("../isa/i386/load_system.pil").replace("$WHICH",&(ext-2).to_string()).replace("$INPUT",input)}
                            else{format!("{}{}",include_str!("../isa/i386/descriptor_rights.pil").replace("$INPUT",input),if ext==4{include_str!("../isa/i386/verr.pil")}else{include_str!("../isa/i386/verw.pil")})};
                        (format!("{name} {}",operand_name(src,address)),compile(&text,16))
                    }
                }else if matches!(second,0x02|0x03){
                    // LAR and LSL: #UD in real mode.
                    let name=if second==0x02{"LAR"}else{"LSL"};
                    let m=r.byte()?;
                    if self.real_mode||self.segment_state().v86(){return Err(GuestError::Decode(format!("{name} is #UD in real and virtual-8086 mode")));}
                    dst=view(((m>>3)&7)as usize,w);src=view((m&7)as usize,16);
                    let source_label=if m>>6!=3{address=Some(AddressContext{effective:r.effective(m,addr32)?,override_segment,segments:self.segments});let label=operand_name(src,address);src=dst;label}else{register_name(src).into()};
                    let input=if address.is_some(){"read_address source\nload selector source 16"}else{"read selector src 16"};
                    let result=if w==32{"write_if dst rights valid"}else{"trunc rights16 rights 16\nwrite_if dst rights16 valid"};
                    let mut tail=(if second==0x02{include_str!("../isa/i386/lar.pil")}else{include_str!("../isa/i386/lsl.pil")}).replace("$RESULT",result);
                    if second==0x02&&w==16{tail=tail.replace("const rights_mask 32 16776960","const rights_mask 32 65280");}
                    let text=format!("{}{}",include_str!("../isa/i386/descriptor_rights.pil").replace("$INPUT",input),tail);
                    (format!("{name} {},{}",register_name(dst),source_label),compile(&text,w))
                }else if second==0x01{
                    let m=r.byte()?;
                    if matches!((m>>3)&7,2|3){
                        if m>>6==3{return Err(GuestError::Decode("LGDT/LIDT require memory".into()));}
                        let effective=r.effective(m,addr32)?;
                        let seg=effective.resolve(&[0;8],override_segment)?.segment.index();
                        address=Some(AddressContext{effective,override_segment,segments:self.segments});
                        let table=((m>>3)&7)-2;
                        let text=include_str!("../isa/i386/load_table.pil").replace("$SEG",&seg.to_string()).replace("$TABLE",&table.to_string()).replace("$MASK",if wide{"4294967295"}else{"16777215"});
                        (if table==0{"LGDT".into()}else{"LIDT".into()},compile(&text,w))
                    }else{
                    if (m>>3)&7==6{
                        w=16;dst=view((m&7)as usize,w);src=dst;
                        let mut text=include_str!("../isa/i386/lmsw.pil").to_string();
                        if m>>6!=3{address=Some(AddressContext{effective:r.effective(m,addr32)?,override_segment,segments:self.segments});text=text.replace("read input src 16","read_address source\nload input source 16");}
                        (format!("LMSW {}",operand_name(src,address)),compile(&text,w))
                    }else{
                    if (m>>3)&7!=4{return Err(GuestError::Decode("unsupported 0F01 group extension".into()));}
                    // For r32 choose the full modeled CR0 for the undefined
                    // upper half. Memory and 16-bit registers get the MSW.
                    let whole=wide&&m>>6==3;
                    w=if whole{32}else{16};dst=view((m&7)as usize,w);src=dst;
                    let mut text=description("smsw").map_err(GuestError::Decode)?;
                    if whole{text=text.replace("require_width 16","require_width 32").replace("read_msw status","read_msw32 status");}
                    if m>>6!=3{address=Some(AddressContext{effective:r.effective(m,addr32)?,override_segment,segments:self.segments});text=text.replace("write dst status","read_address destination\nstore destination status");}
                    (format!("SMSW {}",operand_name(dst,address)),compile(&text,w))
                    }
                    }
                }else if second==0x06{
                    ("CLTS".into(),compile(include_str!("../isa/i386/clts.pil"),w))
                }else if (0x80..=0x8f).contains(&second){
                    disp_bits=w;let raw=r.unsigned(w)?;disp=if w==16{raw as i16 as i32}else{raw as i32};
                    let name=format!("j{}",CONDITIONS[(second&15) as usize]);
                    (format!("{} 0x{:08x}",name.to_ascii_uppercase(),r.pos.wrapping_add(disp as u32)&super::mask(w) as u32),compile(&instantiate(description(&name).map_err(GuestError::Decode)?,w),w))
                }else if matches!(second,0xb6|0xb7|0xbe|0xbf){
                    let source_width=if second&1==0{8}else{16};
                    let m=r.byte()?;dst=view(((m>>3)&7)as usize,w);src=view((m&7)as usize,source_width);
                    let name=if second&8==0{"movzx"}else{"movsx"};
                    let mut text=description(&format!("{name}{source_width}")).map_err(GuestError::Decode)?;
                    let source_label=if m>>6!=3{
                        let effective=r.effective(m,addr32)?;address=Some(AddressContext{effective,override_segment,segments:self.segments});
                        let label=operand_name(src,address);src=dst;
                        text=text.replace(&format!("read a src {source_width}"),&format!("read_address addr\nload a addr {source_width}"));label
                    }else{register_name(src).into()};
                    (format!("{} {},{}",name.to_ascii_uppercase(),register_name(dst),source_label),compile(&instantiate(text,w),w))
                }else if matches!(second,0xa3|0xab|0xb3|0xbb|0xba){
                    // Register-offset memory forms address a bit string: the signed
                    // offset selects the word or dword at EA+(offset>>4)*2 or
                    // EA+(offset>>5)*4, wrapped to the address size.
                    let m=r.byte()?;let memory=m>>6!=3;
                    let kind=if second==0xba{(m>>3)&7}else{4+((second>>3)&3)};
                    if kind<4{return Err(GuestError::Decode("unsupported 0FBA group extension (BT/BTS/BTR/BTC /4-/7 only)".into()));}
                    let name=["BT","BTS","BTR","BTC"][(kind-4)as usize];
                    dst=view((m&7)as usize,w);src=view(((m>>3)&7)as usize,w);
                    if memory{address=Some(AddressContext{effective:r.effective(m,addr32)?,override_segment,segments:self.segments});}
                    let (offset,offset_label)=if second==0xba{let imm=r.byte()?;(format!("const b $W {imm}"),format!("0x{imm:02x}"))}else{("read b src $W".into(),register_name(src).into())};
                    let string=format!("read_address base\nread b src $W\nsext offset b 32\nconst unit_bits 32 {}\nashr units offset unit_bits\nconst unit_scale 32 {}\nshl unit_bytes units unit_scale\nadd at base unit_bytes\nconst address_mask 32 {}\nand addr at address_mask\nload a addr $W",if w==32{5}else{4},if w==32{2}else{1},if addr32{u32::MAX}else{0xffff});
                    let (operand,write)=if memory&&second!=0xba{(string.as_str(),"store addr r")}else if memory{("read_address addr\nload a addr $W","store addr r")}else{("read a dst $W","write dst r")};
                    let offset=if memory&&second!=0xba{String::new()}else{offset};
                    let update=match kind{5=>format!("or r a selected\n{write}"),6=>format!("not cleared selected\nand r a cleared\n{write}"),7=>format!("xor r a selected\n{write}"),_=>String::new()};
                    let text=include_str!("../isa/i386/bt.pil").replace("$OPERAND",operand).replace("$OFFSET",&offset).replace("$UPDATE",&update);
                    (format!("{name} {},{offset_label}",operand_name(dst,address)),compile(&instantiate(text,w),w))
                }else if matches!(second,0xa4|0xa5|0xac|0xad){
                    let m=r.byte()?;let memory=m>>6!=3;
                    dst=view((m&7)as usize,w);src=view(((m>>3)&7)as usize,w);
                    if memory{address=Some(AddressContext{effective:r.effective(m,addr32)?,override_segment,segments:self.segments});}
                    let (count,count_label)=if second&1==0{let imm=r.byte()?;(format!("const raw 8 {imm}"),format!("0x{imm:02x}"))}else{("read_gpr raw 1 8".into(),"CL".into())};
                    let (operand,write)=if memory{("read_address addr\nload a addr $W","store addr r")}else{("read a dst $W","write dst r")};
                    let (name,text)=if second<0xa8{("SHLD",include_str!("../isa/i386/shld.pil"))}else{("SHRD",include_str!("../isa/i386/shrd.pil"))};
                    // A 16-bit double shift works on 48 bits on the 386: dst:src:src (SHLD), src:src:dst (SHRD).
                    let (join_b,join_low,join_top,join_high)=if w==16{("16","65535","48","281470681743360")}else{("0","0","64","0")};
                    let text=text.replace("$OPERAND",operand).replace("$COUNT",&count).replace("$WRITE",write).replace("$JOINB",join_b).replace("$JOINLOW",join_low).replace("$JOINTOP",join_top).replace("$JOINHIGH",join_high);
                    (format!("{name} {},{},{count_label}",operand_name(dst,address),register_name(src)),compile(&instantiate(text,w),w))
                }else if matches!(second,0xbc|0xbd){
                    let m=r.byte()?;dst=view(((m>>3)&7)as usize,w);src=view((m&7)as usize,w);
                    let read=if m>>6!=3{address=Some(AddressContext{effective:r.effective(m,addr32)?,override_segment,segments:self.segments});src=dst;"read_address addr\nload a addr $W"}else{"read a src $W"};
                    let (name,text)=if second==0xbc{("BSF",include_str!("../isa/i386/bsf.pil"))}else{("BSR",include_str!("../isa/i386/bsr.pil"))};
                    (format!("{name} {},{}",register_name(dst),operand_name(src,address)),compile(&instantiate(text.replace("$READ",read),w),w))
                }else if second==0xaf {
                    let m=r.byte()?;let reg=view(((m>>3)&7) as usize,w);let rm=view((m&7) as usize,w);
                    dst=reg;src=rm;
                    if m>>6!=3{let effective=r.effective(m,addr32)?;address=Some(AddressContext{effective,override_segment,segments:self.segments});src=reg;}
                    let text=operand_text("imul",w,address.is_some(),true,None)?;
                    (format!("IMUL {},{}",register_name(dst),operand_name(src,address)),compile(&text,w))
                }else if matches!(second,0xb2|0xb4|0xb5){
                    let name=match second{0xb2=>"LSS",0xb4=>"LFS",_=>"LGS"};
                    let m=r.byte()?;if m>>6==3{return Err(GuestError::Decode(format!("{name} requires a memory far pointer")));}
                    dst=view(((m>>3)&7)as usize,w);src=dst;
                    let effective=r.effective(m,addr32)?;
                    address=Some(AddressContext{effective,override_segment,segments:self.segments});
                    let dest_reg=match second{0xb2=>"2",0xb4=>"4",_=>"5"};
                    let text=include_str!("../isa/i386/les.pil").replace("$SEG",&effective.resolve(&[0;8],override_segment)?.segment.index().to_string()).replace("$OFFSET_MASK",if addr32{"4294967295"}else{"65535"}).replace("$BYTES",&(w/8).to_string()).replace("$DEST",dest_reg);
                    (format!("{name} {},{}",register_name(dst),operand_name(src,address)),compile(&instantiate(text,w),w))
                }else{
                if !(0x90..=0x9f).contains(&second) {return Err(GuestError::Decode(format!("unsupported opcode 0f {second:02x}")));}
                let m=r.byte()?;
                // The reg field of SETcc is ignored.
                w=8;dst=view((m&7) as usize,w);src=dst;
                let name=format!("set{}",CONDITIONS[(second&15) as usize]);
                let mut text=description(&name).map_err(GuestError::Decode)?;
                if m>>6!=3{address=Some(AddressContext{effective:r.effective(m,addr32)?,override_segment,segments:self.segments});text=text.replace("write dst result","read_address destination\nstore destination result");}
                (format!("{} {}",name.to_ascii_uppercase(),operand_name(dst,address)),compile(&instantiate(text,w),w))
                }
            }
            0x8d=>{
                let m=r.byte()?;if m>>6==3{return Err(GuestError::Decode("LEA requires an address expression, not a register source".into()));}
                dst=view(((m>>3)&7)as usize,w);src=dst;
                address=Some(AddressContext{effective:r.effective(m,addr32)?,override_segment,segments:self.segments});
                let operand=operand_name(dst,address);let expression=&operand[operand.find('[').expect("effective address")..];
                (format!("LEA {},{expression}",register_name(dst)),compile(&instantiate(description("lea").map_err(GuestError::Decode)?,w),w))
            }
            0x98=>{
                let source_width=w/2;dst=view(0,w);src=view(0,source_width);
                (if w==16{"CBW"}else{"CWDE"}.into(),compile(&instantiate(description(&format!("movsx{source_width}")).map_err(GuestError::Decode)?,w),w))
            }
            0x99=>{
                dst=view(2,w);src=view(0,w);
                (if w==16{"CWD"}else{"CDQ"}.into(),compile(&instantiate(description("cwd").map_err(GuestError::Decode)?,w),w))
            }
            0x69|0x6b=>{
                let m=r.byte()?;dst=view(((m>>3)&7) as usize,w);src=view((m&7) as usize,w);
                let memory=m>>6!=3;
                if memory{let effective=r.effective(m,addr32)?;address=Some(AddressContext{effective,override_segment,segments:self.segments});src=dst;}
                let immediate=if opcode==0x6b{(r.byte()? as i8 as i32 as u32)&super::mask(w) as u32}else{r.unsigned(w)?};
                let mut text=description("imul").map_err(GuestError::Decode)?
                    .replace("read a dst $W","read a src $W")
                    .replace("read b src $W",&format!("const b $W {immediate}"));
                if memory{text=text.replace("read a src $W","read_address addr\nload a addr $W");}
                (format!("IMUL {},{},0x{immediate:0digits$x}",register_name(dst),operand_name(src,address),digits=(w/4) as usize),compile(&instantiate(text,w),w))
            }
            0xc0|0xc1|0xd0|0xd1|0xd2|0xd3=>{
                if opcode&1==0{w=8;}
                let m=r.byte()?;let name=match (m>>3)&7{0=>"rol",1=>"ror",2=>"rcl",3=>"rcr",4|6=>"shl",5=>"shr",_=>"sar"};
                dst=view((m&7) as usize,w);src=view(1,w);
                let memory=m>>6!=3;
                if memory{let effective=r.effective(m,addr32)?;address=Some(AddressContext{effective,override_segment,segments:self.segments});dst=view(0,w);}
                let immediate=match opcode{0xc0|0xc1=>Some(u32::from(r.byte()?)),0xd0|0xd1=>Some(1),_=>None};
                let text=operand_text(name,w,memory,false,immediate)?;
                (format!("{} {},{}",name.to_ascii_uppercase(),operand_name(dst,address),immediate.map(|v|format!("0x{v:02x}")).unwrap_or_else(||"CL".into())),compile(&text,w))
            }
            0x27|0x2f=>{
                w=8;dst=view(0,w);src=dst;
                let name=if opcode==0x27{"daa"}else{"das"};
                (name.to_ascii_uppercase(),compile(&description(name).map_err(GuestError::Decode)?,w))
            }
            0x37|0x3f=>{
                w=16;dst=view(0,w);src=dst;
                let name=if opcode==0x37{"aaa"}else{"aas"};
                (name.to_ascii_uppercase(),compile(&description(name).map_err(GuestError::Decode)?,w))
            }
            0xd4|0xd5=>{
                // AAM/AAD imm8 (D4 0A and D5 0A are the decimal forms).
                w=16;dst=view(0,w);src=dst;
                let base=r.byte()?;let name=if opcode==0xd4{"aam"}else{"aad"};
                let text=description(name).map_err(GuestError::Decode)?.replace("read base src 8",&format!("const base 8 {base}"));
                (format!("{} 0x{base:02x}",name.to_ascii_uppercase()),compile(&text,w))
            }
            0x9e|0x9f=>{
                w=8;dst=view(4,w);src=dst;
                let name=if opcode==0x9f{"lahf"}else{"sahf"};
                (name.to_ascii_uppercase(),compile(&description(name).map_err(GuestError::Decode)?,w))
            }
            0xf5|0xf8|0xf9|0xfc|0xfd=>{
                let name=match opcode{0xf5=>"cmc",0xf8=>"clc",0xf9=>"stc",0xfc=>"cld",_=>"std"};
                (name.to_ascii_uppercase(),compile(&instantiate(description(name).map_err(GuestError::Decode)?,w),w))
            }
            0x86|0x87=>{
                if opcode==0x86{w=8;}
                let m=r.byte()?;
                dst=view((m&7)as usize,w);src=view(((m>>3)&7)as usize,w);
                let memory=m>>6!=3;
                if locked&&!memory{return Err(GuestError::Decode("LOCK XCHG requires a memory operand".into()));}
                let text=if memory{
                    address=Some(AddressContext{effective:r.effective(m,addr32)?,override_segment,segments:self.segments});
                    dst=src;
                    include_str!("../isa/i386/xchg_ram.pil").to_string()
                }else{description("xchg").map_err(GuestError::Decode)?};
                (format!("XCHG {},{}",operand_name(dst,address),register_name(src)),compile(&instantiate(text,w),w))
            }
            0x91..=0x97=>{
                dst=view(0,w);src=view((opcode&7)as usize,w);
                (format!("XCHG {},{}",register_name(dst),register_name(src)),compile(&instantiate(description("xchg").map_err(GuestError::Decode)?,w),w))
            }
            0x90=>("NOP".into(),compile(&description("nop").map_err(GuestError::Decode)?,w)),
            0x9b=>("FWAIT".into(),compile(include_str!("../isa/i386/fwait.pil"),w)),
            0xd6=>{w=8;dst=view(0,w);src=dst;("SALC".into(),compile(include_str!("../isa/i386/salc.pil"),w))}
            0x40..=0x4f=>{
                dst=view((opcode&7) as usize,w);src=dst;
                let name=if opcode<0x48{"inc"}else{"dec"};
                (format!("{} {}",name.to_ascii_uppercase(),register_name(dst)),compile(&instantiate(description(name).map_err(GuestError::Decode)?,w),w))
            }
            // AL/AX/EAX immediate arithmetic and TEST.
            0x04|0x05|0x0c|0x0d|0x14|0x15|0x1c|0x1d|0x24|0x25|0x2c|0x2d|0x34|0x35|0x3c|0x3d|0xa8|0xa9=>{
                if opcode&1==0{w=8;}dst=view(0,w);src=dst;
                let name=if opcode>=0xa8{"test"}else{alu_name((opcode>>3)&7)};
                let value=r.unsigned(w)?;
                let text=operand_text(name,w,false,false,Some(value))?;
                (format!("{} {},0x{value:0digits$x}",name.to_ascii_uppercase(),register_name(dst),digits=(w/4) as usize),compile(&text,w))
            }
            0x00..=0x03|0x08..=0x0b|0x10..=0x13|0x18..=0x1b|0x20..=0x23|0x28..=0x2b|0x30..=0x33|0x38..=0x3b|0x84|0x85|0x88..=0x8b|0x80..=0x83|0xf6|0xf7|0xfe|0xff=>{
                if opcode&1==0{w=8;}
                let m=r.byte()?;let extension=(m>>3)&7;
                let reg=view(extension as usize,w);let rm=view((m&7) as usize,w);
                let group1=matches!(opcode,0x80..=0x83);
                let group3=matches!(opcode,0xf6|0xf7);
                let group45=matches!(opcode,0xfe|0xff);
                if group3 && extension == 4 {
                    let memory = m >> 6 != 3;
                    dst = view(0, w);
                    src = if memory { let effective = r.effective(m, addr32)?; address = Some(AddressContext { effective, override_segment, segments: self.segments }); dst } else { rm };
                    (format!("MUL {}", operand_name(src, address)), compile(&mul_pil(w, memory), w))
                } else if group3 && extension == 5 {
                    let memory = m >> 6 != 3;
                    dst = view(0, w);
                    src = if memory { let effective = r.effective(m, addr32)?; address = Some(AddressContext { effective, override_segment, segments: self.segments }); dst } else { rm };
                    (format!("IMUL {}", operand_name(src, address)), compile(&imul1_pil(w, memory), w))
                } else if group3 && extension == 6 {
                    let memory = m >> 6 != 3;
                    dst = view(0, w);
                    src = if memory { let effective = r.effective(m, addr32)?; address = Some(AddressContext { effective, override_segment, segments: self.segments }); dst } else { rm };
                    let text = div_pil(w, memory);
                    (format!("DIV {}", operand_name(src, address)), compile(&text, w))
                } else if group3 && extension == 7 {
                    let memory = m >> 6 != 3;
                    dst = view(0, w);
                    src = if memory { let effective = r.effective(m, addr32)?; address = Some(AddressContext { effective, override_segment, segments: self.segments }); dst } else { rm };
                    (format!("IDIV {}", operand_name(src, address)), compile(&idiv_pil(w, memory), w))
                } else {
                let name=if group1{alu_name(extension)}else if group3 {
                    match extension{0|1=>"test",2=>"not",3=>"neg",_=>return Err(GuestError::Decode("unsupported group 3 extension".into()))}
                }else if group45 {
                    match extension{0=>"inc",1=>"dec",_=>return Err(GuestError::Decode("unsupported group 4/5 extension".into()))}
                }else if opcode>=0x88{"mov"}else if opcode>=0x84{"test"}else{alu_name((opcode>>3)&7)};
                let reverse=!group1&&!group3&&!group45&&opcode!=0x84&&opcode!=0x85&&opcode&2!=0;
                let memory=m>>6!=3;
                if memory {
                    let effective=r.effective(m,addr32)?;
                    address=Some(AddressContext{effective,override_segment,segments:self.segments});
                    dst=reg;src=reg;
                }else{(dst,src)=if reverse{(reg,rm)}else{(rm,reg)};}
                let immediate=if opcode==0x83{Some((r.byte()? as i8 as i32 as u32)&super::mask(w) as u32)}
                    else if group1||(group3&&extension<2){Some(r.unsigned(w)?)}else{None};
                let text=operand_text(name,w,memory,reverse,immediate)?;
                let left=operand_name(dst,if reverse{None}else{address});
                let right=if let Some(value)=immediate{format!("0x{value:0digits$x}",digits=(w/4) as usize)}else{operand_name(src,if reverse{address}else{None})};
                let label=if matches!(name,"not"|"neg"|"inc"|"dec"){format!("{} {left}",name.to_ascii_uppercase())}else{format!("{} {left},{right}",name.to_ascii_uppercase())};
                (label,compile(&text,w))
                }
            }
            0xe4..=0xe7|0xec..=0xef=>{
                // Port E9h in a monitor-sized image is the debugger's mock serial
                // port. Any other port is an open bus: IN reads all ones and OUT
                // goes nowhere, as SingleStepTests/80386 records. Dine answers its
                // own device ports before the decoder runs.
                if opcode&1==0{w=8;}dst=view(0,w);src=dst;
                let port=if opcode<0xe8{Some(r.byte()?)}else{None};
                if matches!((opcode,port),(0xe4|0xe6,Some(0xe9)))&&self.end<=RAM_BYTES as u32{
                    return Ok(Decoded{label:if opcode==0xe4{"IN AL,0xe9"}else{"OUT 0xe9,AL"}.into(),length:r.pos-eip,operation:if opcode==0xe4{Operation::Input{next:r.pos}}else{Operation::Output{next:r.pos}}});
                }
                let input=opcode&2==0;
                let port_name=port.map(|p|format!("0x{p:02x}")).unwrap_or_else(||"DX".into());
                let label=if input{format!("IN {},{port_name}",register_name(dst))}else{format!("OUT {port_name},{}",register_name(dst))};
                let text=if input{include_str!("../isa/i386/in_open_bus.pil")}else{include_str!("../isa/i386/out_open_bus.pil")};
                let port_text=match port{Some(p)=>format!("const port 16 {p}"),None=>"read_gpr port 2 16".into()};
                (label,compile(&instantiate(text.replace("$IO_PORT",&port_text),w),w))
            }
            0xcc|0xcd|0xce=>{
                let vector=match opcode{0xcc=>3,0xce=>4,_=>r.byte()?};
                let label=match opcode{0xcc=>"INT3".to_string(),0xce=>"INTO".to_string(),_=>format!("INT 0x{vector:02x}")};
                return Ok(Decoded{label,length:r.pos-eip,operation:Operation::Interrupt{vector,overflow:opcode==0xce,next:r.pos}});
            }
            // HLT is privileged: above CPL 0 in protected mode it is #GP(0).
            0xf4 if !self.real_mode&&self.segment_state().cpl()!=0=>("HLT".into(),compile("require_ring0\npreserve_flags all\nretire\n",w)),
            0xf4=>return Ok(Decoded{label:"HLT (monitor stop)".into(),length:r.pos-eip,operation:Operation::Halt}),
            _=>return Err(GuestError::Decode(format!("unsupported opcode {opcode:02x} at {eip:08x}"))),
        };
        if !(0x70..=0x7f).contains(&opcode) && !matches!(opcode,0xeb|0xe9|0xe8|0xe0..=0xe3|0x0f) {disp_bits=32;}
        if (opcode==0x0f && disp==0 && disp_bits==8) || call_indirect.is_some() {disp_bits=32;}
        let flow=FlowContext{next_eip:r.pos,displacement:disp,displacement_bits:disp_bits,code_limit:self.segments[SegmentRegister::Cs.index()].limit};
        Ok(Decoded{label,length:r.pos-eip,operation:Operation::Pil{program:program.map_err(GuestError::Decode)?,dst,src,flow,address}})
    }
}
pub(crate) fn register_name(view:View)->&'static str {
    const BYTE:[&str;8]=["AL","CL","DL","BL","AH","CH","DH","BH"];
    const WORD:[&str;8]=["AX","CX","DX","BX","SP","BP","SI","DI"];
    const DWORD:[&str;8]=["EAX","ECX","EDX","EBX","ESP","EBP","ESI","EDI"];
    match view.width{8=>BYTE[view.register+if view.low==8{4}else{0}],16=>WORD[view.register],_=>DWORD[view.register]}
}
pub(crate) fn operand_name(view:View,address:Option<AddressContext>)->String {
    let Some(address)=address else{return register_name(view).into();};
    let mut terms=Vec::new();let displacement;
    match address.effective {
        EffectiveAddress::Bits16{form,displacement:d}=>{
            let base=match form{Address16::BxSi=>"BX+SI",Address16::BxDi=>"BX+DI",Address16::BpSi=>"BP+SI",Address16::BpDi=>"BP+DI",Address16::Si=>"SI",Address16::Di=>"DI",Address16::Bp=>"BP",Address16::Bx=>"BX",Address16::Direct=>""};
            if !base.is_empty(){terms.push(base.to_owned());}displacement=i64::from(d as i16);
            if terms.is_empty(){terms.push(format!("0x{d:04x}"));}
            else if displacement!=0{terms.push(format!("{}{:#x}",if displacement<0{"-"}else{"+"},displacement.unsigned_abs()));}
        }
        EffectiveAddress::Bits32{base,index,scale,displacement:d}=>{
            if let Some(base)=base{terms.push(format!("{base:?}").to_ascii_uppercase());}
            if let Some(index)=index{let index=format!("{index:?}").to_ascii_uppercase();let prefix=if terms.is_empty(){""}else{"+"};terms.push(if scale==1{format!("{prefix}{index}")}else{format!("{prefix}{index}*{scale}")});}
            displacement=i64::from(d as i32);
            if terms.is_empty(){terms.push(format!("0x{d:08x}"));}
            else if displacement!=0{terms.push(format!("{}{:#x}",if displacement<0{"-"}else{"+"},displacement.unsigned_abs()));}
        }
    }
    let segment=address.effective.resolve(&[0;8],address.override_segment).expect("decoded effective address").segment;
    let segment=format!("{segment:?}").to_ascii_uppercase();let size=match view.width{8=>"byte",16=>"word",_=>"dword"};
    format!("{size} {segment}:[{}]",terms.concat())
}
/// Reads a far pointer for CALL FAR/JMP FAR m16:16 or m16:32. Each field is
/// checked on its own and the selector offset wraps at the address size, as
/// the 386 does (a 16-bit EA of FFFEh takes the selector from 0000h).
fn far_pointer(seg:usize,w:u8,addr32:bool)->String{
    let (bytes,mask)=(w/8,if addr32{u32::MAX}else{0xffff});
    format!("read_address operand\nconst slot 32 {bytes}\nadd selector_end operand slot\nconst offset_mask 32 {mask}\nand selector_at selector_end offset_mask\nzext wide_offset operand 64\ncheck_segment {seg} wide_offset {bytes}\nzext wide_selector selector_at 64\ncheck_segment {seg} wide_selector 2\nload target_value operand {w}\nload selector selector_at 16")
}
fn alu_name(group:u8)->&'static str { ["add","or","adc","sbb","and","sub","xor","cmp"][group as usize] }
// Bind operand locations before validating tokens; flag semantics remain in PIL.
fn operand_text(name:&str,w:u8,memory:bool,reverse:bool,immediate:Option<u32>)->Result<String,GuestError> {
    if name=="mov" {
        return Ok(if memory {instantiate(description(if reverse{"load_ram"}else{"store_ram"}).map_err(GuestError::Decode)?,w)}
            else{format!("read v src {w}\nwrite dst v\npreserve_flags all\nretire")});
    }
    let mut text=description(name).map_err(GuestError::Decode)?;
    if memory {
        if reverse {text=text.replace("read b src $W","read_address addr\nload b addr $W");}
        else {text=text.replace("read a dst $W","read_address addr\nload a addr $W")
                .replace("read b dst $W","read_address addr\nload b addr $W")
                .replace("write dst r","store addr r");}
    }
    if let Some(value)=immediate {text=text.replace("read b src $W",&format!("const b $W {value}"));}
    Ok(instantiate(text,w))
}
fn view(register:usize,width:u8)->View {
    if width==8 && register>=4 {View{register:register-4,low:8,width}}else{View{register,low:0,width}}
}
// Unsigned MUL: the double-width product goes to AX, DX:AX or EDX:EAX.
// CF=OF=1 when the upper half is non-zero; SF/ZF/AF/PF are undefined.
fn mul_pil(w: u8, memory: bool) -> String {
    let read_source = if memory { format!("read_address addr\nload source addr {w}") } else { format!("read source src {w}") };
    let d = w * 2;
    let write = match w {
        8 => "write_gpr 0 product".to_string(),
        _ => format!("trunc low product {w}\nconst shift {d} {w}\nlshr high_wide product shift\ntrunc high high_wide {w}\nwrite_gpr 0 low\nwrite_gpr 2 high"),
    };
    format!("read_gpr acc 0 {w}\n{read_source}\nzext acc_wide acc {d}\nzext source_wide source {d}\nmul product acc_wide source_wide\nconst upper_shift {d} {w}\nlshr upper product upper_shift\nzero fits upper\nnot overflow fits\n{write}\nflag CF overflow\nflag OF overflow\nundef_flag SF\nundef_flag ZF\nundef_flag AF\nundef_flag PF\nretire")
}
/// One-operand IMUL: AL/AX/EAX times r/m into AX, DX:AX or EDX:EAX. CF=OF=1
/// when the signed product does not fit the sign-extended low half.
fn imul1_pil(w: u8, memory: bool) -> String {
    let read_source = if memory { format!("read_address addr\nload source addr {w}") } else { format!("read source src {w}") };
    let d = w * 2;
    let write = match w {
        8 => "write_gpr 0 product".to_string(),
        _ => format!("const shift {d} {w}\nlshr high_wide product shift\ntrunc high high_wide {w}\nwrite_gpr 0 low\nwrite_gpr 2 high"),
    };
    format!("read_gpr acc 0 {w}\n{read_source}\nsext acc_wide acc {d}\nsext source_wide source {d}\nmul product acc_wide source_wide\ntrunc low product {w}\nsext restored low {d}\nxor lost product restored\nzero fits lost\nnot overflow fits\n{write}\nflag CF overflow\nflag OF overflow\nundef_flag SF\nundef_flag ZF\nundef_flag AF\nundef_flag PF\nretire")
}
fn div_pil(w: u8, memory: bool) -> String {
    let read_divisor = if memory { format!("read_address addr\nload divisor addr {w}") } else { format!("read divisor src {w}") };
    match w {
        8 => format!("read_gpr ax 0 16\n{read_divisor}\nzero div_zero divisor\nnot div_ok div_zero\nraise_unless div_ok 0\nzext divisor_16 divisor 16\ndiv q_16 ax divisor_16\nrem r_16 ax divisor_16\ntrunc q_8 q_16 8\ntrunc r_8 r_16 8\nzext q_check q_8 16\nult overflow q_check q_16\nnot no_overflow overflow\nraise_unless no_overflow 0\nzext q_wide q_8 16\nzext r_wide r_8 16\nconst shift 16 8\nshl r_high r_wide shift\nor ax_new r_high q_wide\nwrite_gpr 0 ax_new\nundef_flag CF\nundef_flag OF\nundef_flag SF\nundef_flag ZF\nundef_flag AF\nundef_flag PF\nretire"),
        16 => format!("read_gpr ax 0 16\nread_gpr dx 2 16\nzext ax_32 ax 32\nzext dx_32 dx 32\nconst shift 32 16\nshl dx_high dx_32 shift\nor dividend dx_high ax_32\n{read_divisor}\nzero div_zero divisor\nnot div_ok div_zero\nraise_unless div_ok 0\nzext divisor_32 divisor 32\ndiv q_32 dividend divisor_32\nrem r_32 dividend divisor_32\ntrunc q_16 q_32 16\ntrunc r_16 r_32 16\nzext q_check q_16 32\nult overflow q_check q_32\nnot no_overflow overflow\nraise_unless no_overflow 0\nwrite_gpr 0 q_16\nwrite_gpr 2 r_16\nundef_flag CF\nundef_flag OF\nundef_flag SF\nundef_flag ZF\nundef_flag AF\nundef_flag PF\nretire"),
        32 => format!("read_gpr eax 0 32\nread_gpr edx 2 32\nzext eax_64 eax 64\nzext edx_64 edx 64\nconst shift 64 32\nshl edx_high edx_64 shift\nor dividend edx_high eax_64\n{read_divisor}\nzero div_zero divisor\nnot div_ok div_zero\nraise_unless div_ok 0\nzext divisor_64 divisor 64\ndiv q_64 dividend divisor_64\nrem r_64 dividend divisor_64\ntrunc q_32 q_64 32\ntrunc r_32 r_64 32\nzext q_check q_32 64\nult overflow q_check q_64\nnot no_overflow overflow\nraise_unless no_overflow 0\nwrite_gpr 0 q_32\nwrite_gpr 2 r_32\nundef_flag CF\nundef_flag OF\nundef_flag SF\nundef_flag ZF\nundef_flag AF\nundef_flag PF\nretire"),
        _ => unreachable!(),
    }
}
/// IDIV: signed division of AX, DX:AX or EDX:EAX through magnitudes. A zero
/// divisor or a quotient outside the signed operand range is #DE (`require`).
/// The remainder takes the dividend's sign; flags are undefined.
fn idiv_pil(w: u8, memory: bool) -> String {
    let read_divisor = if memory { format!("read_address addr\nload divisor addr {w}") } else { format!("read divisor src {w}") };
    let d = w * 2;
    let dividend = match w {
        8 => "read_gpr dividend 0 16".to_string(),
        _ => format!("read_gpr low 0 {w}\nread_gpr high 2 {w}\nzext low_wide low {d}\nzext high_wide high {d}\nconst shift {d} {w}\nshl high_up high_wide shift\nor dividend high_up low_wide"),
    };
    let write = match w {
        8 => "zext q_wide q_w 16\nzext r_wide r_w 16\nconst shift 16 8\nshl r_high r_wide shift\nor ax_new r_high q_wide\nwrite_gpr 0 ax_new".to_string(),
        _ => "write_gpr 0 q_w\nwrite_gpr 2 r_w".to_string(),
    };
    let top = d - 1;
    format!("{dividend}\n{read_divisor}\nzero div_zero divisor\nnot div_ok div_zero\nraise_unless div_ok 0\nsext d_wide divisor {d}\nconst none {d} 0\nbit n_sign dividend {top}\nbit d_sign d_wide {top}\nsub n_neg none dividend\nselect n_abs n_sign n_neg dividend\nsub d_neg none d_wide\nselect d_abs d_sign d_neg d_wide\ndiv q_abs n_abs d_abs\nrem r_abs n_abs d_abs\nxor q_sign n_sign d_sign\nsub q_neg none q_abs\nselect q q_sign q_neg q_abs\nsub r_neg none r_abs\nselect r n_sign r_neg r_abs\ntrunc q_w q {w}\nsext q_check q_w {d}\nxor q_lost q_check q\nzero q_fits q_lost\nraise_unless q_fits 0\ntrunc r_w r {w}\n{write}\nundef_flag CF\nundef_flag OF\nundef_flag SF\nundef_flag ZF\nundef_flag AF\nundef_flag PF\nretire")
}
fn gpr(n:u8)->Gpr {[Gpr::Eax,Gpr::Ecx,Gpr::Edx,Gpr::Ebx,Gpr::Esp,Gpr::Ebp,Gpr::Esi,Gpr::Edi][n as usize]}
struct Reader<'a> {image:&'a Image,ram:&'a Ram,start:u32,pos:u32}
impl Reader<'_> {
    fn byte(&mut self)->Result<u8,GuestError> {
        if self.pos-self.start>=15 {return Err(GuestError::Decode("instruction exceeds 15 bytes".into()));}
        // Image extent is a monitor safety boundary, not an architectural fault.
        let c=MemoryContext{segment:SegmentRegister::Cs,cache:self.image.segments[SegmentRegister::Cs.index()]};
        let linear=c.translate(self.pos,MemWidth::Byte,false,self.start)?;
        let bus=self.ram.bus_address(linear,false).map_err(|f|GuestError::Execution(f.error(self.start)))?;
        if !self.image.contains_linear(u64::from(bus)) {return Err(GuestError::Decode(format!("fetch outside loaded image at {:08x}",self.pos)));}
        let value=self.ram.load(linear,MemWidth::Byte).map_err(|f|GuestError::Execution(f.error(self.start)))?;
        self.pos+=1;Ok(value as u8)
    }
    fn unsigned(&mut self,width:u8)->Result<u32,GuestError> {
        let mut n=0;for shift in (0..width).step_by(8) {n|=u32::from(self.byte()?)<<shift;}Ok(n)
    }
    fn effective(&mut self,m:u8,addr32:bool)->Result<EffectiveAddress,GuestError> {
        let mode=m>>6;let rm=m&7;
        if !addr32 {
            let direct=mode==0&&rm==6;
            let form=if direct{Address16::Direct}else{[Address16::BxSi,Address16::BxDi,Address16::BpSi,Address16::BpDi,Address16::Si,Address16::Di,Address16::Bp,Address16::Bx][rm as usize]};
            let displacement=if direct||mode==2 {self.unsigned(16)? as u16}else if mode==1{self.byte()? as i8 as i16 as u16}else{0};
            Ok(EffectiveAddress::Bits16{form,displacement})
        } else {
            let (base,index,scale)=if rm==4 {
                let sib=self.byte()?;let b=sib&7;let i=(sib>>3)&7;
                (if mode==0&&b==5{None}else{Some(gpr(b))},if i==4{None}else{Some(gpr(i))},if i==4{1}else{1<<(sib>>6)})
            }else{(if mode==0&&rm==5{None}else{Some(gpr(rm))},None,1)};
            let displacement=if mode==2||(mode==0&&base.is_none()){self.unsigned(32)?}else if mode==1{self.byte()? as i8 as i32 as u32}else{0};
            Ok(EffectiveAddress::Bits32{base,index,scale,displacement})
        }
    }
}
#[cfg(test)]
#[path="../tests/unit/guest.rs"]
mod tests;

