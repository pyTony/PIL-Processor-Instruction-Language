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
}
#[derive(Clone,Debug)]
pub struct Decoded {pub label:String,pub length:u32,pub operation:Operation}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Image {pub fetch_base:u32,pub start:u32,pub end:u32,pub segments:[SegmentCache;6],pub selectors:[u16;6],pub msw:u16,pub real_mode:bool,pub ss_shadow:u8,pub sti_shadow:u8,pub flag_checkpoint:Option<(u32,u32,u32)>}
impl Image {
    // Keep the imported physical code window fixed when CS changes.
    pub fn contains_offset(&self,offset:u32)->bool{
        let at=u64::from(self.segments[1].base)+u64::from(offset);
        at>=u64::from(self.fetch_base)+u64::from(self.start)&&at<u64::from(self.fetch_base)+u64::from(self.end)
    }
    pub fn retired(&mut self){let mut s=self.segment_state();s.retired();self.set_segment_state(s);}
    pub fn segment_state(&self)->SegmentState{SegmentState{msw:self.msw,segments:self.segments,selectors:self.selectors,real_mode:self.real_mode,ss_shadow:self.ss_shadow,sti_shadow:self.sti_shadow,flag_checkpoint:self.flag_checkpoint}}
    pub fn set_segment_state(&mut self,s:SegmentState){self.msw=s.msw;self.segments=s.segments;self.selectors=s.selectors;self.real_mode=s.real_mode;self.ss_shadow=s.ss_shadow;self.sti_shadow=s.sti_shadow;self.flag_checkpoint=s.flag_checkpoint;}

    pub fn load(bytes:&[u8])->Result<(Self,Ram),String> {
        if bytes.is_empty() || bytes.len()>RAM_BYTES-ENTRY as usize {return Err("image must contain 1..3840 bytes".into());}
        let mut ram=Ram::new(RAM_BYTES);ram.write_bytes(ENTRY,bytes).map_err(|e|format!("{e:?}"))?;
        let mut segments=[SegmentCache::real_mode(0);6];
        // Ordinary real-mode CS permits data reads and writes too.
        // Protected-mode callers supply Code caches with their own permissions.
        segments[SegmentRegister::Ds.index()]=SegmentCache::real_mode(0x10);
        segments[SegmentRegister::Ss.index()]=SegmentCache::real_mode(0x20);
        segments[SegmentRegister::Fs.index()]=SegmentCache::real_mode(0x30);
        Ok((Self{fetch_base:0,start:ENTRY,end:ENTRY+bytes.len() as u32,segments,selectors:[0,0,0x20,0x10,0x30,0],msw:0x10,real_mode:true,ss_shadow:0,sti_shadow:0,flag_checkpoint:None},ram))
    }
    pub fn fetch_signature(&self,ram:&Ram,eip:u32,length:u32)->Result<[u8;15],GuestError> {
        if length==0 || length>15{return Err(GuestError::Decode("invalid cached instruction length".into()));}
        let mut reader=Reader{image:self,ram,start:eip,pos:eip};let mut bytes=[0u8;15];
        for byte in &mut bytes[..length as usize] {*byte=reader.byte()?;}
        Ok(bytes)
    }
    pub fn decode(&self,ram:&Ram,eip:u32)->Result<Decoded,GuestError> {
        let mut r=Reader{image:self,ram,start:eip,pos:eip};
        let mut wide=false;let mut addr32=false;let mut override_segment=None;let mut repeat=None;
        let mut opcode=loop {
            let b=r.byte()?;
            match b {
                0xf2|0xf3 if repeat.is_none()=>repeat=Some(b),
                0x66 if !wide=>wide=true,0x67 if !addr32=>addr32=true,
                0x26|0x2e|0x36|0x3e|0x64|0x65 if override_segment.is_none()=>override_segment=Some(match b{0x26=>SegmentRegister::Es,0x2e=>SegmentRegister::Cs,0x36=>SegmentRegister::Ss,0x3e=>SegmentRegister::Ds,0x64=>SegmentRegister::Fs,_=>SegmentRegister::Gs}),
                _=>break b,
            }
        };
        if let Some(prefix)=repeat{if !matches!(opcode,0xa4|0xa5|0xaa|0xab|0xac|0xad|0xa6|0xa7|0xae|0xaf)||(prefix==0xf2&&!matches!(opcode,0xa6|0xa7|0xae|0xaf)){return Err(GuestError::Decode("unsupported repeat-prefix/instruction combination".into()));}}
        // Normalize alternative register encodings while retaining consumed bytes.
        let mut push_selector=None;let mut pop_selector=None;let mut call_indirect=None;let mut memory_pop=None;let mut memory_push=None;
        if matches!(opcode,0xff|0x8f){
            let m=r.byte()?;let extension=(m>>3)&7;
            if opcode==0xff&&extension==2{call_indirect=Some(m);opcode=0xe8;}
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
            0xff if memory_push.is_some()=>{
                let m=memory_push.unwrap();
                let effective=r.effective(m,addr32)?;address=Some(AddressContext{effective,override_segment,segments:self.segments});
                let stack_bits=if matches!(self.segments[2].access,SegmentAccess::Data{big:true,..}){32}else{16};
                let text=description("push_memory").map_err(GuestError::Decode)?.replace("$S",&stack_bits.to_string()).replace("$BYTES",&(w/8).to_string());
                (format!("PUSH {}",operand_name(dst,address)),compile(&instantiate(text,w),w))
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
            0xc9=>{
                let stack_bits=if matches!(self.segments[2].access,SegmentAccess::Data{big:true,..}){32}else{16};
                let text=description("leave").map_err(GuestError::Decode)?.replace("$S",&stack_bits.to_string()).replace("$BYTES",&(w/8).to_string());
                ("LEAVE".into(),compile(&instantiate(text,w),w))
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
            0xc2|0xc3=>{
                let cleanup=if opcode==0xc2{r.unsigned(16)?}else{0};let stack_bits=if matches!(self.segments[2].access,SegmentAccess::Data{big:true,..}){32}else{16};
                let text=include_str!("../isa/i386/ret.pil").replace("$CLEANUP",&cleanup.to_string()).replace("$S",&stack_bits.to_string()).replace("$BYTES",&(w/8).to_string());
                (if opcode==0xc2{format!("RET 0x{cleanup:04x}")}else{"RET".into()},compile(&instantiate(text,w),w))
            }
            0xa4|0xa5|0xaa|0xab|0xac|0xad=>{
                if opcode&1==0{w=8;}dst=view(0,w);src=dst;let a=if addr32{32}else{16};
                let name=match opcode{0xa4|0xa5=>"movs",0xaa|0xab=>"stos",_=>"lods"};
                let body=match name{"movs"=>include_str!("../isa/i386/movs.pil"),"stos"=>include_str!("../isa/i386/stos.pil"),_=>include_str!("../isa/i386/lods.pil")};
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
            0x8c|0x8e=>{
                let m=r.byte()?;let segment=((m>>3)&7)as usize;
                if segment>=6 || (opcode==0x8e&&segment==1){return Err(GuestError::Decode("invalid segment MOV encoding".into()));}
                if opcode==0x8e&&!self.real_mode{return Err(GuestError::Decode("segment loads require real-mode context".into()));}
                if opcode==0x8c&&wide&&m>>6==3{return Err(GuestError::Decode("386 MOV r32,Sreg upper-half policy is not admitted".into()));}
                w=16;dst=view((m&7)as usize,w);src=dst;
                let seg=["ES","CS","SS","DS","FS","GS"][segment];
                let mut text=if opcode==0x8c{format!("read_segment selector {segment}\nwrite dst selector\npreserve_flags all\nretire")}
                    else{format!("read selector src 16\nwrite_segment {segment} selector\npreserve_flags all\nretire")};
                if m>>6!=3{let effective=r.effective(m,addr32)?;address=Some(AddressContext{effective,override_segment,segments:self.segments});
                    text=if opcode==0x8c{text.replace("write dst selector","read_address addr\nstore addr selector")}else{text.replace("read selector src 16","read_address addr\nload selector addr 16")};}
                let operand=operand_name(dst,address);
                (if opcode==0x8c{format!("MOV {operand},{seg}")}else{format!("MOV {seg},{operand}")},compile(&text,w))
            }
            0x07|0x17|0x1f=>{
                if !self.real_mode{return Err(GuestError::Decode("segment loads require real-mode context".into()));}
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
                if second==0x01{
                    let m=r.byte()?;if (m>>3)&7!=4{return Err(GuestError::Decode("unsupported 0F01 group extension (SMSW /4 only)".into()));}
                    if wide&&m>>6==3{return Err(GuestError::Decode("SMSW r32 upper-half policy is outside the 386 word profile".into()));}
                    w=16;dst=view((m&7)as usize,w);src=dst;
                    let mut text=description("smsw").map_err(GuestError::Decode)?;
                    if m>>6!=3{address=Some(AddressContext{effective:r.effective(m,addr32)?,override_segment,segments:self.segments});text=text.replace("write dst status","read_address destination\nstore destination status");}
                    (format!("SMSW {}",operand_name(dst,address)),compile(&text,w))
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
                }else if second==0xaf {
                    let m=r.byte()?;let reg=view(((m>>3)&7) as usize,w);let rm=view((m&7) as usize,w);
                    dst=reg;src=rm;
                    if m>>6!=3{let effective=r.effective(m,addr32)?;address=Some(AddressContext{effective,override_segment,segments:self.segments});src=reg;}
                    let text=operand_text("imul",w,address.is_some(),true,None)?;
                    (format!("IMUL {},{}",register_name(dst),operand_name(src,address)),compile(&text,w))
                }else{
                if !(0x90..=0x9f).contains(&second) {return Err(GuestError::Decode(format!("unsupported opcode 0f {second:02x}")));}
                let m=r.byte()?;
                if (m>>3)&7!=0{return Err(GuestError::Decode("SETcc profile requires /0".into()));}
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
                let m=r.byte()?;let name=match (m>>3)&7{0=>"rol",1=>"ror",4=>"shl",5=>"shr",7=>"sar",_=>return Err(GuestError::Decode("unsupported shift/rotate extension".into()))};
                dst=view((m&7) as usize,w);src=view(1,w);
                let memory=m>>6!=3;
                if memory{let effective=r.effective(m,addr32)?;address=Some(AddressContext{effective,override_segment,segments:self.segments});dst=view(0,w);}
                let immediate=match opcode{0xc0|0xc1=>Some(u32::from(r.byte()?)),0xd0|0xd1=>Some(1),_=>None};
                let text=operand_text(name,w,memory,false,immediate)?;
                (format!("{} {},{}",name.to_ascii_uppercase(),operand_name(dst,address),immediate.map(|v|format!("0x{v:02x}")).unwrap_or_else(||"CL".into())),compile(&text,w))
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
                if m>>6!=3{return Err(GuestError::Decode("memory XCHG requires bus-atomic support; register forms only".into()));}
                dst=view((m&7)as usize,w);src=view(((m>>3)&7)as usize,w);
                (format!("XCHG {},{}",register_name(dst),register_name(src)),compile(&instantiate(description("xchg").map_err(GuestError::Decode)?,w),w))
            }
            0x91..=0x97=>{
                dst=view(0,w);src=view((opcode&7)as usize,w);
                (format!("XCHG {},{}",register_name(dst),register_name(src)),compile(&instantiate(description("xchg").map_err(GuestError::Decode)?,w),w))
            }
            0x90=>("NOP".into(),compile(&description("nop").map_err(GuestError::Decode)?,w)),
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
            0x00..=0x03|0x08..=0x0b|0x10..=0x13|0x18..=0x1b|0x20..=0x23|0x28..=0x2b|0x30..=0x33|0x38..=0x3b|0x84|0x85|0x88..=0x8b|0x80|0x81|0x83|0xf6|0xf7|0xfe|0xff=>{
                if opcode&1==0{w=8;}
                let m=r.byte()?;let extension=(m>>3)&7;
                let reg=view(extension as usize,w);let rm=view((m&7) as usize,w);
                let group1=matches!(opcode,0x80|0x81|0x83);
                let group3=matches!(opcode,0xf6|0xf7);
                let group45=matches!(opcode,0xfe|0xff);
                let name=if group1{alu_name(extension)}else if group3 {
                    match extension{0=>"test",2=>"not",3=>"neg",_=>return Err(GuestError::Decode("unsupported group 3 extension".into()))}
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
                    else if group1||(group3&&extension==0){Some(r.unsigned(w)?)}else{None};
                let text=operand_text(name,w,memory,reverse,immediate)?;
                let left=operand_name(dst,if reverse{None}else{address});
                let right=if let Some(value)=immediate{format!("0x{value:0digits$x}",digits=(w/4) as usize)}else{operand_name(src,if reverse{address}else{None})};
                let label=if matches!(name,"not"|"neg"|"inc"|"dec"){format!("{} {left}",name.to_ascii_uppercase())}else{format!("{} {left},{right}",name.to_ascii_uppercase())};
                (label,compile(&text,w))
            }
            0xe4|0xe6=>{
                let port=r.byte()?;if port!=0xe9{return Err(GuestError::Decode(format!("unsupported monitor port {port:02x}")));}
                return Ok(Decoded{label:if opcode==0xe4{"IN AL,0xe9"}else{"OUT 0xe9,AL"}.into(),length:r.pos-eip,operation:if opcode==0xe4{Operation::Input{next:r.pos}}else{Operation::Output{next:r.pos}}});
            }
            0xf4=>return Ok(Decoded{label:"HLT (monitor stop)".into(),length:r.pos-eip,operation:Operation::Halt}),
            _=>return Err(GuestError::Decode(format!("unsupported opcode {opcode:02x} at {eip:08x}"))),
        };
        if !(0x70..=0x7f).contains(&opcode) && !matches!(opcode,0xeb|0xe9|0xe8|0xe3|0x0f) {disp_bits=32;}
        if opcode==0x0f && disp==0 && disp_bits==8 {disp_bits=32;}
        let flow=FlowContext{next_eip:r.pos,displacement:disp,displacement_bits:disp_bits,code_limit:self.segments[SegmentRegister::Cs.index()].limit};
        Ok(Decoded{label,length:r.pos-eip,operation:Operation::Pil{program:program.map_err(GuestError::Decode)?,dst,src,flow,address}})
    }
}
fn register_name(view:View)->&'static str {
    const BYTE:[&str;8]=["AL","CL","DL","BL","AH","CH","DH","BH"];
    const WORD:[&str;8]=["AX","CX","DX","BX","SP","BP","SI","DI"];
    const DWORD:[&str;8]=["EAX","ECX","EDX","EBX","ESP","EBP","ESI","EDI"];
    match view.width{8=>BYTE[view.register+if view.low==8{4}else{0}],16=>WORD[view.register],_=>DWORD[view.register]}
}
fn operand_name(view:View,address:Option<AddressContext>)->String {
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
fn gpr(n:u8)->Gpr {[Gpr::Eax,Gpr::Ecx,Gpr::Edx,Gpr::Ebx,Gpr::Esp,Gpr::Ebp,Gpr::Esi,Gpr::Edi][n as usize]}
struct Reader<'a> {image:&'a Image,ram:&'a Ram,start:u32,pos:u32}
impl Reader<'_> {
    fn byte(&mut self)->Result<u8,GuestError> {
        if self.pos-self.start>=15 {return Err(GuestError::Decode("instruction exceeds 15 bytes".into()));}
        // Image extent is a monitor safety boundary, not an architectural fault.
        if !self.image.contains_offset(self.pos) {return Err(GuestError::Decode(format!("fetch outside loaded image at {:08x}",self.pos)));}
        let c=MemoryContext{segment:SegmentRegister::Cs,cache:self.image.segments[SegmentRegister::Cs.index()]};
        let linear=c.translate(self.pos,MemWidth::Byte,false,self.start)?;
        let value=self.ram.load(linear,MemWidth::Byte).map_err(|f|GuestError::Execution(ExecError::MemoryAccess{eip:self.start,address:f.address,bytes:1,write:false}))?;
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

