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
pub struct Image {pub end:u32,pub segments:[SegmentCache;6]}
impl Image {
    pub fn load(bytes:&[u8])->Result<(Self,Ram),String> {
        if bytes.is_empty() || bytes.len()>RAM_BYTES-ENTRY as usize {return Err("image must contain 1..3840 bytes".into());}
        let mut ram=Ram::new(RAM_BYTES);ram.write_bytes(ENTRY,bytes).map_err(|e|format!("{e:?}"))?;
        let mut segments=[SegmentCache::real_mode(0);6];
        segments[SegmentRegister::Cs.index()].access=SegmentAccess::Code{readable:true};
        segments[SegmentRegister::Ds.index()]=SegmentCache::real_mode(0x10);
        segments[SegmentRegister::Ss.index()]=SegmentCache::real_mode(0x20);
        segments[SegmentRegister::Fs.index()]=SegmentCache::real_mode(0x30);
        Ok((Self{end:ENTRY+bytes.len() as u32,segments},ram))
    }
    pub fn fetch_signature(&self,ram:&Ram,eip:u32,length:u32)->Result<[u8;15],GuestError> {
        if length==0 || length>15{return Err(GuestError::Decode("invalid cached instruction length".into()));}
        let mut reader=Reader{image:self,ram,start:eip,pos:eip};let mut bytes=[0u8;15];
        for byte in &mut bytes[..length as usize] {*byte=reader.byte()?;}
        Ok(bytes)
    }
    pub fn decode(&self,ram:&Ram,eip:u32)->Result<Decoded,GuestError> {
        let mut r=Reader{image:self,ram,start:eip,pos:eip};
        let mut wide=false;let mut addr32=false;let mut override_segment=None;
        let opcode=loop {
            let b=r.byte()?;
            match b {
                0x66 if !wide=>wide=true,0x67 if !addr32=>addr32=true,
                0x26|0x2e|0x36|0x3e|0x64|0x65 if override_segment.is_none()=>override_segment=Some(match b{0x26=>SegmentRegister::Es,0x2e=>SegmentRegister::Cs,0x36=>SegmentRegister::Ss,0x3e=>SegmentRegister::Ds,0x64=>SegmentRegister::Fs,_=>SegmentRegister::Gs}),
                _=>break b,
            }
        };
        let mut w=if wide {32}else{16};let mut dst=view(0,w);let mut src=dst;let mut address=None;let mut disp=0;let mut disp_bits=8;
        let (label,program)=match opcode {
            0xb0..=0xbf=>{
                if opcode<0xb8 {w=8;}
                dst=view((opcode&7) as usize,w);src=dst;
                let value=r.unsigned(w)?;
                ("MOV immediate".to_owned(),compile(&format!("const v {w} {value}\nwrite dst v\npreserve_flags all\nretire"),w))
            }
            0x70..=0x7f=>{
                disp=r.byte()? as i8 as i32;
                let name=format!("j{}",CONDITIONS[(opcode&15) as usize]);
                (name.clone(),compile(&instantiate(description(&name).map_err(GuestError::Decode)?,w),w))
            }
            0x0f=>{
                let second=r.byte()?;
                if second==0xaf {
                    let m=r.byte()?;let reg=view(((m>>3)&7) as usize,w);let rm=view((m&7) as usize,w);
                    dst=reg;src=rm;
                    if m>>6!=3{let effective=r.effective(m,addr32)?;address=Some(AddressContext{effective,override_segment,segments:self.segments});src=reg;}
                    let text=operand_text("imul",w,address.is_some(),true,None)?;
                    ("IMUL".into(),compile(&text,w))
                }else{
                if !(0x90..=0x9f).contains(&second) {return Err(GuestError::Decode(format!("unsupported opcode 0f {second:02x}")));}
                let m=r.byte()?;
                if m>>6!=3 || (m>>3)&7!=0 {return Err(GuestError::Decode("SETcc profile supports register destination with /0 only".into()));}
                w=8;dst=view((m&7) as usize,w);src=dst;
                let name=format!("set{}",CONDITIONS[(second&15) as usize]);
                (name.clone(),compile(&instantiate(description(&name).map_err(GuestError::Decode)?,w),w))
                }
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
                ("IMUL immediate".into(),compile(&instantiate(text,w),w))
            }
            0x90=>("NOP".into(),compile(&description("nop").map_err(GuestError::Decode)?,w)),
            0x40..=0x4f=>{
                dst=view((opcode&7) as usize,w);src=dst;
                let name=if opcode<0x48{"inc"}else{"dec"};
                (name.to_ascii_uppercase(),compile(&instantiate(description(name).map_err(GuestError::Decode)?,w),w))
            }
            // AL/AX/EAX immediate arithmetic and TEST.
            0x04|0x05|0x0c|0x0d|0x14|0x15|0x1c|0x1d|0x24|0x25|0x2c|0x2d|0x34|0x35|0x3c|0x3d|0xa8|0xa9=>{
                if opcode&1==0{w=8;}dst=view(0,w);src=dst;
                let name=if opcode>=0xa8{"test"}else{alu_name((opcode>>3)&7)};
                let value=r.unsigned(w)?;
                let text=operand_text(name,w,false,false,Some(value))?;
                (format!("{} immediate",name.to_ascii_uppercase()),compile(&text,w))
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
                (name.to_ascii_uppercase(),compile(&text,w))
            }
            0xe4|0xe6=>{
                let port=r.byte()?;if port!=0xe9{return Err(GuestError::Decode(format!("unsupported monitor port {port:02x}")));}
                return Ok(Decoded{label:if opcode==0xe4{"IN AL,0xe9"}else{"OUT 0xe9,AL"}.into(),length:r.pos-eip,operation:if opcode==0xe4{Operation::Input{next:r.pos}}else{Operation::Output{next:r.pos}}});
            }
            0xf4=>return Ok(Decoded{label:"HLT (monitor stop)".into(),length:r.pos-eip,operation:Operation::Halt}),
            _=>return Err(GuestError::Decode(format!("unsupported opcode {opcode:02x} at {eip:08x}"))),
        };
        if !(0x70..=0x7f).contains(&opcode) {disp_bits=32;}
        let flow=FlowContext{next_eip:r.pos,displacement:disp,displacement_bits:disp_bits,code_limit:self.segments[SegmentRegister::Cs.index()].limit};
        Ok(Decoded{label,length:r.pos-eip,operation:Operation::Pil{program:program.map_err(GuestError::Decode)?,dst,src,flow,address}})
    }
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
        if self.pos<ENTRY || self.pos>=self.image.end {return Err(GuestError::Decode(format!("fetch outside loaded image at {:08x}",self.pos)));}
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
