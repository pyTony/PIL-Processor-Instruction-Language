//! Host REPL for PIL demos and a small executable 386 monitor profile.
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, BufRead, Write};
use super::{compile, description, execute_flow, instantiate, number, Cpu, Program, View};
use super::flow::{ExecError, FlowContext};
use super::memory::Ram;
use super::serial::{MockSerial, SerialTransport};

#[path="debug_service.rs"]
mod service;
pub use service::serve;

const MAX_RUN: usize = 10_000;
const DEFAULT_RUN: usize = 100;
const MAX_TIMED:usize=1_000_000;
const REGS: [&str;8] = ["eax","ecx","edx","ebx","esp","ebp","esi","edi"];

#[derive(Clone, Debug)]
enum Action {
    Cpu { program: Program, dst: View, src: View, context: FlowContext },
    CpuMemory {program:Program,dst:View,src:View,context:FlowContext,address:u32},
    MockRx { next: u32 },
    MockTx { next: u32 },
    HarnessHalt,
}
#[derive(Clone, Debug)]
struct Instruction { label: &'static str, action: Action }

#[derive(Clone, Debug, PartialEq, Eq)]
enum Stop {
    Advanced,
    Breakpoint(u32),
    Budget(u32),
    WaitingRx(u32),
    TxBlocked(u32),
    Halt(u32),
    MissingInstruction(u32),
    Fault(ExecError),
    Decode{eip:u32,message:String},
}

struct Debugger {
    cpu: Cpu,
    initial: Cpu,
    ram:Ram,
    initial_ram:Ram,
    instructions: BTreeMap<u32, Instruction>,
    breakpoints: BTreeSet<u32>,
    stopped_at_break: Option<u32>,
    serial: MockSerial,
    demo: String,
    image:Option<super::guest::Image>,
    initial_image:Option<super::guest::Image>,
    tx_events:Vec<u8>,
    cache:super::cache::DecodeCache,
    last_step_access:Option<super::ProbedMemoryAccess>,
    last_data_access:Option<super::ProbedMemoryAccess>,
    pil_cursor:Option<(u32,usize)>,
}

impl Debugger {
    fn new(demo: &str) -> Result<Self,String> {
        let mut d=Self {
            cpu: Cpu::default(), initial: Cpu::default(),
            ram:Ram::new(256),initial_ram:Ram::new(256),
            instructions: BTreeMap::new(), breakpoints: BTreeSet::new(),
            stopped_at_break: None, serial: MockSerial::new(256,256), demo: String::new(),image:None,initial_image:None,tx_events:Vec::new(),cache:super::cache::DecodeCache::default(),
            last_step_access:None,last_data_access:None,pil_cursor:None,
        };
        d.load_demo(demo)?; Ok(d)
    }
    fn load_demo(&mut self, name: &str) -> Result<(),String> {
        if name=="extensions" {return self.load_image(&super::guest::extension_bytes(),"extensions");}
        if name=="mandelbrot" {return self.load_image(&super::mandelbrot::guest_bytes(8,16)?,"mandelbrot");}
        if name=="factorials" {return self.load_image(&super::guest::factorial_bytes()?,"factorials");}
        if name=="loop" {return self.load_image(&super::guest::loop_bytes(super::guest::LOOP_ITERATIONS)?,"loop");}
        if name=="hello" {return self.load_image(&super::guest::hello_bytes()?,"hello");}
        if name=="selftest" {return self.load_image(&super::guest::selftest_bytes()?,"selftest");}
        // Construct first; a failed selection must not destroy the current state.
        let mut code=BTreeMap::new();let mut cpu=Cpu::default();cpu.eflags=0x202;
        match name {
            "counter" => {
                cpu.eip=0x100;cpu.regs[0]=3;cpu.regs[3]=1;
                let dst=View{register:0,low:0,width:32};let src=View{register:3,low:0,width:32};
                code.insert(0x100,Instruction{label:"SUB EAX,EBX",action:Action::Cpu{
                    program:compile(&instantiate(description("sub")?,32),32)?,dst,src,
                    context:FlowContext{next_eip:0x102,displacement:0,displacement_bits:32,code_limit:u32::MAX}}});
                code.insert(0x102,Instruction{label:"JNE 0x100",action:Action::Cpu{
                    program:compile(&instantiate(description("jne")?,32),32)?,dst,src,
                    context:FlowContext{next_eip:0x108,displacement:-8,displacement_bits:32,code_limit:u32::MAX}}});
                code.insert(0x108,Instruction{label:"HARNESS_HALT",action:Action::HarnessHalt});
            }
            "echo" => {
                cpu.eip=0x200;
                code.insert(0x200,Instruction{label:"MOCK_RX AL",action:Action::MockRx{next:0x201}});
                code.insert(0x201,Instruction{label:"MOCK_TX AL",action:Action::MockTx{next:0x202}});
                code.insert(0x202,Instruction{label:"HARNESS_HALT",action:Action::HarnessHalt});
            }
            "memory"=>{
                cpu.eip=0x300;cpu.regs[1]=0x1234_5678;
                let dst=View{register:0,low:0,width:32};let src=View{register:1,low:0,width:32};
                for (ip,next,name,label) in [(0x300,0x302,"store_ram","STORE_RAM [0x10],ECX"),(0x302,0x304,"load_ram","LOAD_RAM EAX,[0x10]")] {
                    code.insert(ip,Instruction{label,action:Action::CpuMemory{
                        program:compile(&instantiate(description(name)?,32),32)?,dst,src,address:0x10,
                        context:FlowContext{next_eip:next,displacement:0,displacement_bits:32,code_limit:u32::MAX}}});
                }
                code.insert(0x304,Instruction{label:"HARNESS_HALT",action:Action::HarnessHalt});
            }
            _ => return Err("demo must be counter, echo, memory, selftest, hello, factorials, loop, mandelbrot or extensions".into()),
        }
        self.cache.clear();self.image=None;self.initial_image=None;self.instructions=code;self.cpu=cpu.clone();self.initial=cpu;
        self.ram=Ram::new(256);self.initial_ram=self.ram.clone();
        self.breakpoints.clear();self.stopped_at_break=None;self.serial.clear();self.tx_events.clear();self.last_step_access=None;self.last_data_access=None;self.pil_cursor=None;self.demo=name.into();
        Ok(())
    }
    fn load_image(&mut self,bytes:&[u8],name:&str)->Result<(),String> {
        let (image,ram)=super::guest::Image::load(bytes)?;
        let mut cpu=Cpu::default();cpu.eip=super::guest::ENTRY;cpu.eflags=0x202;
        self.initial=cpu.clone();self.cpu=cpu;self.initial_ram=ram.clone();self.ram=ram;
        self.cache.clear();self.initial_image=Some(image.clone());self.image=Some(image);self.instructions.clear();self.breakpoints.clear();
        self.stopped_at_break=None;self.serial.clear();self.tx_events.clear();self.last_step_access=None;self.last_data_access=None;self.pil_cursor=None;self.demo=name.into();Ok(())
    }
    fn step_image(&mut self)->Stop {
        let mut lazy=super::LazyFlags::None;
        let stop=self.step_image_lazy(&mut lazy);
        lazy.flush(&mut self.cpu.eflags,&mut self.cpu.undefined_flags);
        stop
    }
    fn step_image_lazy(&mut self,lazy:&mut super::LazyFlags)->Stop {
        use super::guest::{GuestError,Operation};
        let decoded=match self.cache.fetch(self.image.as_ref().unwrap(),&self.ram,self.cpu.eip) {
            Ok(d)=>d,Err(GuestError::Execution(e))=>return Stop::Fault(e),
            Err(GuestError::Decode(message))=>return Stop::Decode{eip:self.cpu.eip,message},
        };
        match &decoded.operation {
            Operation::Pil{program,dst,src,flow,address}=>{
                let mut machine=self.image.as_ref().unwrap().segment_state();
                let result=super::execute_machine_lazy(program,&mut self.cpu,*dst,*src,*flow,&mut self.ram,*address,&mut machine,lazy);
                self.image.as_mut().unwrap().set_segment_state(machine);
                match result{
                    Ok(probed)=>{
                        self.last_step_access=probed;
                        if let Some(a)=probed{if !a.is_stack{self.last_data_access=Some(a);}}
                        Stop::Advanced
                    }
                    Err(e)=>Stop::Fault(e),
                }
            }
            Operation::Input{next}=>{
                let byte=match self.serial.try_read(){Some(b)=>b,None=>return Stop::WaitingRx(self.cpu.eip)};
                self.cpu.regs[0]=(self.cpu.regs[0]&!255)|u32::from(byte);self.cpu.eip = *next;if let Some(image)=self.image.as_mut(){image.retired();}
                self.last_step_access=None;Stop::Advanced
            }
            Operation::Output{next}=>{
                if self.serial.try_write(self.cpu.regs[0] as u8).is_err(){return Stop::TxBlocked(self.cpu.eip);}
                self.tx_events.push(self.cpu.regs[0] as u8);self.cpu.eip = *next;if let Some(image)=self.image.as_mut(){image.retired();}
                self.last_step_access=None;Stop::Advanced
            }
            Operation::Halt=>Stop::Halt(self.cpu.eip),
        }
    }
    fn reset(&mut self) {
        self.cache.clear();
        self.image=self.initial_image.clone();
        self.cpu=self.initial.clone();self.ram=self.initial_ram.clone();self.serial.clear();self.tx_events.clear();self.stopped_at_break=None;
        self.last_step_access=None;self.last_data_access=None;self.pil_cursor=None;
        // Keep breakpoints while resetting the currently loaded demo.
    }
    fn step(&mut self) -> Stop {
        self.stopped_at_break=None;
        self.pil_cursor=None;
        if self.image.is_some(){return self.step_image();}
        let instruction=match self.instructions.get(&self.cpu.eip) {
            Some(i)=>i, None=>return Stop::MissingInstruction(self.cpu.eip),
        };
        match &instruction.action {
            Action::Cpu{program,dst,src,context} => match execute_flow(program,&mut self.cpu,*dst,*src,*context) {
                Ok(())=>{self.last_step_access=None;Stop::Advanced},Err(e)=>Stop::Fault(e),
            },
            Action::CpuMemory{program,dst,src,context,address}=>{
                match super::execute_memory_probed(program,&mut self.cpu,*dst,*src,*context,&mut self.ram,*address) {
                    Ok(probed)=>{
                        self.last_step_access=probed;
                        if let Some(a)=probed{if !a.is_stack{self.last_data_access=Some(a);}}
                        Stop::Advanced
                    }
                    Err(e)=>Stop::Fault(e),
                }
            }
            Action::MockRx{next} => {
                let byte=match self.serial.try_read() {Some(b)=>b,None=>return Stop::WaitingRx(self.cpu.eip)};
                self.cpu.regs[0]=(self.cpu.regs[0]&!255)|u32::from(byte);
                self.cpu.eip = *next;if let Some(image)=self.image.as_mut(){image.retired();}
                self.last_step_access=None;Stop::Advanced
            }
            Action::MockTx{next} => {
                if self.serial.try_write(self.cpu.regs[0] as u8).is_err() {
                    return Stop::TxBlocked(self.cpu.eip);
                }
                self.tx_events.push(self.cpu.regs[0] as u8);self.cpu.eip = *next;if let Some(image)=self.image.as_mut(){image.retired();}
                self.last_step_access=None;Stop::Advanced
            }
            Action::HarnessHalt=>Stop::Halt(self.cpu.eip),
        }
    }
    fn run(&mut self, budget:usize)->Result<(usize,Stop),String> {
        if budget==0 || budget>MAX_RUN {return Err(format!("run budget must be 1..{MAX_RUN}"));}
        let resume=self.stopped_at_break.take().filter(|eip|*eip==self.cpu.eip);
        let mut lazy=super::LazyFlags::None;
        let has_image=self.image.is_some();
        let no_breaks=self.breakpoints.is_empty();
        for done in 0..budget {
            if !no_breaks && self.breakpoints.contains(&self.cpu.eip) && !(done==0 && resume==Some(self.cpu.eip)) {
                lazy.flush(&mut self.cpu.eflags,&mut self.cpu.undefined_flags);
                self.stopped_at_break=Some(self.cpu.eip);
                return Ok((done,Stop::Breakpoint(self.cpu.eip)));
            }
            let stop=if has_image{
                self.stopped_at_break=None;self.pil_cursor=None;
                self.step_image_lazy(&mut lazy)
            }else{
                self.step()
            };
            if stop!=Stop::Advanced {
                lazy.flush(&mut self.cpu.eflags,&mut self.cpu.undefined_flags);
                return Ok((done,stop));
            }
        }
        lazy.flush(&mut self.cpu.eflags,&mut self.cpu.undefined_flags);
        Ok((budget,Stop::Budget(self.cpu.eip)))
    }
    fn run_large(&mut self,budget:usize)->Result<(usize,Stop),String> {
        if budget==0 || budget>MAX_TIMED{return Err(format!("timed budget must be 1..{MAX_TIMED}"));}
        let mut done=0;
        loop {
            let (n,stop)=self.run((budget-done).min(MAX_RUN))?;done+=n;
            match stop {
                Stop::Budget(_) if done<budget=>{},
                other=>return Ok((done,other)),
            }
        }
    }
    fn timed_run(&mut self,budget:usize)->Result<String,String> {
        // Validate before taking/altering any debugger continuation state.
        if budget==0 || budget>MAX_TIMED{return Err(format!("timed budget must be 1..{MAX_TIMED}"));}
        let start=std::time::Instant::now();let (n,stop)=self.run_large(budget)?;let elapsed=start.elapsed();
        let rate=if n==0 || elapsed.as_nanos()==0 {"rate unavailable (no completed steps or clock interval)".into()}
            else {format!("{:.0} guest instructions/s; {:.1} ns/instruction",n as f64/elapsed.as_secs_f64(),elapsed.as_secs_f64()*1e9/n as f64)};
        Ok(format!("{n} completed steps; {}\nelapsed={:.3} ms; {rate}\nHost wall time: PIL token interpreter; fetch validation, cache misses/compilation and dispatch; not 386 cycles.\n{}",Self::stop_text(&stop),elapsed.as_secs_f64()*1000.0,self.cache.summary()))
    }
    fn flags(&self)->String {
        let mut text=format!("EFLAGS={:08x} undefined={:08x}",self.cpu.eflags,self.cpu.undefined_flags);
        for (name,bit) in [("CF",0),("PF",2),("AF",4),("ZF",6),("SF",7),("OF",11)] {
            text.push_str(&format!(" {name}={}{}",(self.cpu.eflags>>bit)&1,
                if self.cpu.undefined_flags&(1<<bit)!=0 {"?"}else{""}));
        }
        text
    }
    fn stop_text(stop:&Stop)->String {
        match stop {
            Stop::Advanced=>"stepped".into(),
            Stop::Decode{eip,message}=>format!("decode stopped at EIP={eip:08x}: {message}; instruction not committed"),
            Stop::Breakpoint(ip)=>format!("breakpoint before EIP={ip:08x}"),
            Stop::Budget(ip)=>format!("budget exhausted at EIP={ip:08x}"),
            Stop::WaitingRx(ip)=>format!("RX empty; waiting at EIP={ip:08x}"),
            Stop::TxBlocked(ip)=>format!("TX full; waiting at EIP={ip:08x}"),
            Stop::Halt(ip)=>format!("harness halted at EIP={ip:08x}"),
            Stop::MissingInstruction(ip)=>format!("no predecoded instruction at EIP={ip:08x}"),
            Stop::Fault(ExecError::StackSegment{eip,error_code})=>format!("#SS({error_code}) at EIP={eip:08x}; instruction not committed"),
            Stop::Fault(ExecError::GeneralProtection{eip,error_code})=>format!("#GP({error_code}) at EIP={eip:08x}; instruction not committed"),
            Stop::Fault(ExecError::MemoryAccess{eip,address,bytes,write})=>format!("RAM {} fault: address={address:08x} bytes={bytes} at EIP={eip:08x}; instruction not committed",if *write {"write"}else{"read"}),
            Stop::Fault(e)=>format!("execution error: {e:?}"),
        }
    }
    fn check_selftest(&mut self)->Result<(),String> {
        let expected=super::guest::selftest_bytes()?;
        if self.image.is_none() || self.initial_ram.bytes().get(super::guest::ENTRY as usize..super::guest::ENTRY as usize+expected.len())!=Some(expected.as_slice()) {
            return Err("no built-in check for this program; supported images: selftest, factorials, loop, mandelbrot, extensions".into());
        }
        if !matches!(self.step_preview(),Some(super::guest::Operation::Halt)) || self.cpu.regs[2]&0xffff!=0x600d {
            return Err(format!("selftest has not passed: DX={:04x}",self.cpu.regs[2]&0xffff));
        }
        for (address,value) in [(0x500,0x12345679),(0x504,0x12345678),(0x600,0x12345678),(0x700,0x12345678)] {
            if self.ram.load(address,super::memory::MemWidth::Dword)!=Ok(value){return Err(format!("selftest RAM mismatch at {address:08x}"));}
        }
        // Check successful output history, so draining serial tx does not invalidate it.
        if self.tx_events!=[0x41] {return Err("selftest expects exactly TX=41; queue RX=41 before running".into());}
        Ok(())
    }
    fn has_image(&self,bytes:&[u8])->bool {
        self.image.as_ref().map_or(false,|i|i.end==super::guest::ENTRY+bytes.len() as u32)
            && self.initial_ram.bytes().get(super::guest::ENTRY as usize..super::guest::ENTRY as usize+bytes.len())==Some(bytes)
    }
    fn check_factorials(&self)->Result<String,String> {
        if !self.has_image(&super::guest::factorial_bytes()?){return Err("factorial check requires the supplied image".into());}
        if !matches!(self.step_preview(),Some(super::guest::Operation::Halt)){return Err("factorial program has not halted".into());}
        for (i,expected) in super::guest::FACTORIALS.iter().enumerate() {
            if self.ram.load(0x500+(i as u32)*4,super::memory::MemWidth::Dword)!=Ok(*expected){return Err(format!("factorial {} result mismatch",i+1));}
        }
        if self.cpu.regs[0]!=479001600 || self.cpu.regs[6]!=13 || self.cpu.regs[1]!=0 || self.cpu.eflags&0x8d5!=0x44{return Err("factorial final registers mismatch".into());}
        Ok("PASS: factorials 1!..12!; 12 RAM results verified; 12!=479001600".into())
    }
    fn check_loop(&self,iterations:u32)->Result<(),String> {
        if !self.has_image(&super::guest::loop_bytes(iterations)?) {return Err("loop check requires the supplied counted-loop image".into());}
        if !matches!(self.step_preview(),Some(super::guest::Operation::Halt)) || self.cpu.regs[0]!=iterations || self.cpu.regs[1]!=0 || self.cpu.eflags&0x8d5!=0x44 {
            return Err("loop result/flags mismatch or program has not halted".into());
        }
        Ok(())
    }
    fn check_current(&mut self)->Result<String,String> {
        if self.has_image(&super::guest::extension_bytes()){
            if !matches!(self.step_preview(),Some(super::guest::Operation::Halt)){return Err("extension demo has not halted".into());}
            if self.cpu.regs!=[0xffffff80,0xffffff80,0xffffffff,0x80,0xff80,0xff80,0x85,0xff80]
                || self.cpu.eflags!=0x202 || self.cpu.undefined_flags!=0 || self.ram.load(0x180,super::memory::MemWidth::Word)!=Ok(0xff80){return Err("extension demo result/flags mismatch".into());}
            return Ok("PASS: extensions; MOVZX/MOVSX, CBW/CWDE/CDQ, LEA, same-width aliases/memory and preserved flags".into());
        }
        if self.has_image(&super::mandelbrot::guest_bytes(8,16)?){
            if !matches!(self.step_preview(),Some(super::guest::Operation::Halt)){return Err("Mandelbrot demo has not halted".into());}
            if self.tx_events!=super::mandelbrot::reference(8,16)?{return Err("Mandelbrot pixels mismatch".into());}
            return Ok("PASS: 8x8 Mandelbrot; all 64 serial pixels match Q12 reference".into());
        }
        if self.has_image(&super::guest::factorial_bytes()?){return self.check_factorials();}
        // The loop image's only variable field is its initial ECX immediate.
        if let Some(data)=self.initial_ram.bytes().get((super::guest::ENTRY+2) as usize..(super::guest::ENTRY+6) as usize) {
            let iterations=u32::from_le_bytes(data.try_into().unwrap());
            if let Ok(bytes)=super::guest::loop_bytes(iterations) {
                if self.has_image(&bytes){self.check_loop(iterations)?;return Ok(format!("PASS: counted loop {iterations} iterations; EAX={iterations}, ECX=0 and flags verified"));}
            }
        }
        self.check_selftest()?;Ok("PASS: selftest DX=600D, RAM signatures and mock TX=41".into())
    }
    fn step_preview(&self)->Option<super::guest::Operation> {
        self.image.as_ref()?.decode(&self.ram,self.cpu.eip).ok().map(|d|d.operation)
    }
    fn listing(&self,start:u32,count:u64)->Result<String,String>{
        if count==0||count>256{return Err("list count must be 1..256 instructions".into());}
        let mut lines=Vec::new();
        if let Some(image)=&self.image{
            if !image.contains_offset(start){return Err("listing address outside loaded image".into());}
            let mut ip=start;
            for _ in 0..count {
                if !image.contains_offset(ip){break;}
                match image.decode(&self.ram,ip){
                    Ok(d)=>{
                        let signature=image.fetch_signature(&self.ram,ip,d.length).map_err(|e|format!("{e:?}"))?;
                        let bytes=signature[..d.length as usize].iter().map(|b|format!("{b:02x}")).collect::<Vec<_>>().join(" ");
                        lines.push(format!("{} {ip:08x}  {bytes:<44} {}",if ip==self.cpu.eip{"*"}else{" "},d.label));
                        ip+=d.length;if matches!(d.operation,super::guest::Operation::Halt){break;}
                    }
                    Err(e)=>{lines.push(format!("  {ip:08x}  {e:?}"));break;}
                }
            }
        }else{
            if !self.instructions.contains_key(&start){return Err("no instruction at listing address".into());}
            for (ip,instruction) in self.instructions.range(start..).take(count as usize){lines.push(format!("{} {ip:08x} {}",if *ip==self.cpu.eip{"*"}else{" "},instruction.label));}
        }
        Ok(lines.join("\n"))
    }
    fn probe_current_instruction(&self)->Option<super::ProbedMemoryAccess>{
        if let Some(image)=&self.image{
            let decoded=image.decode(&self.ram,self.cpu.eip).ok()?;
            match &decoded.operation{
                super::guest::Operation::Pil{program,dst,src,flow,address}=>{
                    let machine=image.segment_state();
                    super::probe_machine_access(program,&self.cpu,*dst,*src,*flow,&self.ram,*address,&machine)
                }
                _=>None,
            }
        }else{
            let instruction=self.instructions.get(&self.cpu.eip)?;
            match &instruction.action{
                Action::CpuMemory{program,dst,src,context,address}=>{
                    super::probe_program_access(program,&self.cpu,*dst,*src,*context,&self.ram,*address,None,None,None)
                }
                _=>None,
            }
        }
    }
    pub fn active_memory_effect(&self)->Option<(super::ProbedMemoryAccess,&'static str,&[u8])>{
        let (access,stage)=if let Some(a)=self.last_step_access.filter(|a|!a.is_stack){
            (a,if a.write{"written"}else{"read"})
        }else if let Some(a)=self.probe_current_instruction().filter(|a|!a.is_stack){
            (a,if a.write{"next_write"}else{"next_read"})
        }else if let Some(a)=self.last_data_access{
            (a,if a.write{"written"}else{"read"})
        }else if let Some(a)=self.last_step_access{
            (a,if a.write{"written"}else{"read"})
        }else if let Some(a)=self.probe_current_instruction(){
            (a,if a.write{"next_write"}else{"next_read"})
        }else{
            return None;
        };
        let start=usize::try_from(access.address).ok()?;
        let end=start.checked_add(usize::from(access.bytes))?;
        let slice=self.ram.bytes().get(start..end)?;
        Some((access,stage,slice))
    }
    pub fn format_memory_effect(&self)->Option<String>{
        let (access,stage,raw)=self.active_memory_effect()?;
        let hex_bytes=raw.iter().map(|b|format!("{b:02x}")).collect::<Vec<_>>().join(" ");
        let ascii_bar:String=raw.iter().map(|&b|if (0x20..=0x7e).contains(&b){char::from(b)}else{'.'}).collect();
        let char_lits=raw.iter().map(|&b|if (0x20..=0x7e).contains(&b){format!("'{}'",char::from(b))}else{format!("'\\x{b:02x}'")}).collect::<Vec<_>>().join(" ");
        let (uval,sval,hex_val)=match raw.len(){
            1=>{let u=u64::from(raw[0]);let s=i64::from(raw[0] as i8);(u,s,format!("0x{u:02X}"))}
            2=>{let w=u16::from_le_bytes([raw[0],raw[1]]);(u64::from(w),i64::from(w as i16),format!("0x{w:04X}"))}
            4=>{let d=u32::from_le_bytes([raw[0],raw[1],raw[2],raw[3]]);(u64::from(d),i64::from(d as i32),format!("0x{d:08X}"))}
            n=>{
                let u=raw.iter().enumerate().fold(0u64,|acc,(i,&b)|acc|(u64::from(b)<<(i*8)));
                (u,u as i64,format!("0x{u:0width$X}",width=n*2))
            }
        };
        let span=if access.bytes<=1{format!("0x{:08X}",access.address)}else{format!("0x{:08X}..0x{:08X}",access.address,access.address.wrapping_add(u32::from(access.bytes)-1))};
        let unit=if access.bytes==1{"1 byte"}else{&format!("{} bytes",access.bytes)};
        let stage_label=match stage{"written"=>"written","read"=>"read","next_write"=>"target write","next_read"=>"target read",_=>stage};
        let dec_str=if sval<0{format!("{uval} (signed {sval})")}else{format!("{uval}")};
        Some(format!("Memory Cell [{span}] ({unit}, {stage_label}): Hex={hex_val}  Dec={dec_str}  Bytes=[{hex_bytes}]  Chars ({}): |{ascii_bar}| ({char_lits})",raw.len()))
    }
    fn current_pil_trace(&self,stepped:usize)->Option<(String,usize,Option<usize>)>{
        if let Some(image)=&self.image{
            let d=image.decode(&self.ram,self.cpu.eip).ok()?;
            match &d.operation{
                super::guest::Operation::Pil{program,dst,src,flow,address}=>{
                    let machine=image.segment_state();
                    let (offset,mem)=if let Some(mut a)=*address{
                        a.segments=machine.segments;
                        let (o,c)=a.resolve(&self.cpu.regs).unwrap_or((0,super::address::MemoryContext{segment:super::address::SegmentRegister::Ds,cache:machine.segments[3]}));
                        (o,Some(c))
                    }else{(0,None)};
                    let (trace,early)=program.trace_disassemble(stepped,&self.cpu,*dst,*src,*flow,&self.ram,offset,mem,Some(&machine),address.map(|a|a.effective));
                    Some((trace,program.tokens.len(),early))
                }
                _=>None,
            }
        }else{
            let instruction=self.instructions.get(&self.cpu.eip)?;
            match &instruction.action{
                Action::Cpu{program,dst,src,context}=>{
                    let (trace,early)=program.trace_disassemble(stepped,&self.cpu,*dst,*src,*context,&self.ram,0,None,None,None);
                    Some((trace,program.tokens.len(),early))
                }
                Action::CpuMemory{program,dst,src,context,address}=>{
                    let (trace,early)=program.trace_disassemble(stepped,&self.cpu,*dst,*src,*context,&self.ram,*address,None,None,None);
                    Some((trace,program.tokens.len(),early))
                }
                _=>None,
            }
        }
    }
    pub fn step_pil(&mut self)->(String,Stop){
        let done=self.pil_cursor.filter(|(ip,_)|*ip==self.cpu.eip).map(|(_,n)|n).unwrap_or(0);
        if let Some((_,total,early))=self.current_pil_trace(done){
            let next=done+1;
            if next>=total || early==Some(done){
                let from=self.cpu.eip;
                let stop=self.step();
                let outcome=if stop==Stop::Advanced{"retired instruction"}else{"execution stopped"};
                return (format!("PIL token {done}/{}: {outcome}; {}; EIP {from:08x} -> {:08x}",total.saturating_sub(1),Self::stop_text(&stop),self.cpu.eip),stop);
            }
            self.pil_cursor=Some((self.cpu.eip,next));
            return (format!("PIL step {next}/{total} at EIP={:08x}",self.cpu.eip),Stop::Advanced);
        }
        let from=self.cpu.eip;
        let stop=self.step();
        (format!("{}; EIP {from:08x} -> {:08x}",Self::stop_text(&stop),self.cpu.eip),stop)
    }
    pub fn current_pil(&self) -> String {
        let mem_line=self.format_memory_effect().map(|m|format!("\n{m}")).unwrap_or_default();
        let stepped=self.pil_cursor.filter(|(ip,_)|*ip==self.cpu.eip).map(|(_,n)|n).unwrap_or(0);
        let trace_body=self.current_pil_trace(stepped).map(|(t,_,_)|t);
        if let Some(image) = &self.image {
            match image.decode(&self.ram, self.cpu.eip) {
                Ok(d) => match &d.operation {
                    super::guest::Operation::Pil { program, dst, src, flow: _, address } => {
                        let dst_desc = super::guest::operand_name(*dst, *address);
                        let src_desc = super::guest::operand_name(*src, None);
                        let body = trace_body.unwrap_or_else(|| program.disassemble());
                        format!("Instruction: {}\nOperands: dst={}, src={}{mem_line}\nPIL ({}/{} stepped):\n{}", d.label, dst_desc, src_desc, stepped, program.tokens.len(), body)
                    }
                    super::guest::Operation::Input { next } => format!("Instruction: {}\nOperation: MOCK_IN -> AL (next EIP={:08x}){mem_line}", d.label, next),
                    super::guest::Operation::Output { next } => format!("Instruction: {}\nOperation: MOCK_OUT <- AL (next EIP={:08x}){mem_line}", d.label, next),
                    super::guest::Operation::Halt => format!("Instruction: {}\nOperation: HLT (monitor stop){mem_line}", d.label),
                },
                Err(e) => format!("Decode error at EIP={:08x}: {e:?}{mem_line}", self.cpu.eip),
            }
        } else if let Some(instruction) = self.instructions.get(&self.cpu.eip) {
            match &instruction.action {
                Action::Cpu { program, dst, src, .. } => {
                    let dst_desc = super::guest::register_name(*dst);
                    let src_desc = super::guest::register_name(*src);
                    let body = trace_body.unwrap_or_else(|| program.disassemble());
                    format!("Instruction: {}\nOperands: dst={}, src={}{mem_line}\nPIL ({}/{} stepped):\n{}", instruction.label, dst_desc, src_desc, stepped, program.tokens.len(), body)
                }
                Action::CpuMemory { program, dst, src, address, .. } => {
                    let dst_desc = super::guest::register_name(*dst);
                    let src_desc = super::guest::register_name(*src);
                    let body = trace_body.unwrap_or_else(|| program.disassemble());
                    format!("Instruction: {}\nOperands: dst={}, src={}, addr=0x{:x}{mem_line}\nPIL ({}/{} stepped):\n{}", instruction.label, dst_desc, src_desc, address, stepped, program.tokens.len(), body)
                }
                _ => format!("Instruction: {}{mem_line}", instruction.label),
            }
        } else {
            format!("No instruction at EIP={:08x}{mem_line}", self.cpu.eip)
        }
    }
    fn dump_memory(&self,address:u32,count:u64,width:super::memory::MemWidth,decimal:bool)->Result<String,String>{
        if count==0||count>256{return Err("dump count must be 1..256 values".into());}
        let bytes=usize::from(width.bytes());let length=(count as usize)*bytes;
        if u64::from(address)+length as u64>0x1_0000_0000{return Err("RAM range out of bounds".into());}
        let start=usize::try_from(address).map_err(|_|"address out of range")?;
        let end=start.checked_add(length).ok_or("address out of range")?;
        let data=self.ram.bytes().get(start..end).ok_or("RAM range out of bounds")?;
        let digits=if decimal{match bytes{1=>3,2=>5,_=>10}}else{bytes*2};
        let columns=(16/bytes)*(digits+1)-1;let mut lines=Vec::new();
        for (row,chunk) in data.chunks(16).enumerate(){
            let values=chunk.chunks_exact(bytes).map(|item|{
                let value=item.iter().enumerate().fold(0u32,|value,(i,b)|value|(u32::from(*b)<<(i*8)));
                if decimal{format!("{value:>digits$}")}else{format!("{value:0digits$x}")}
            }).collect::<Vec<_>>().join(" ");
            let ascii:String=chunk.iter().map(|&b|if (0x20..=0x7e).contains(&b){char::from(b)}else{'.'}).collect();
            let at=u64::from(address)+(row*16) as u64;
            lines.push(format!("{at:08x}: {values:<columns$} |{ascii:<16}|"));
        }
        Ok(lines.join("\n"))
    }
    fn command(&mut self,line:&str)->Result<(String,bool),String> {self.command_mode(line,true)}
    fn command_mode(&mut self,line:&str,show_guest_output:bool)->Result<(String,bool),String> {
        let tx_before=self.tx_events.len();
        let words:Vec<_>=line.split_whitespace().collect();
        let response=match words.as_slice() {
            []=>String::new(),
            ["help"]=>concat!(
                "help | demo counter|echo|memory|selftest|hello|factorials|loop|mandelbrot|extensions | load <binary> | check | list | step | run [1..10000] | continue [budget]\n",
                "list [count 1..256] | list <address> <count> (bytes, operands, branch targets; * marks EIP)\n",
                "dosprobe <com|exe|img|zip path> [steps] (isolated DOS import/run/report; preserves current debugger)\n",
                "cache [stats|on|off|clear] (binary monitor)\n",
                "time [steps 1..1000000] | bench [iterations 1..100000] [samples 1..9] | stress [passes 1..1000]\n",
                "regs | flags | eip | set <register|eip|eflags> <decimal|0xhex>\n",
                "break <address> | delete <address> | breaks | reset | quit\n",
                "serial status | serial rx <hex bytes...> | serial tx | serial clear\n",
                "mem8|mem16|mem32 <address> [values 1..256] [hex|dec] (little-endian + printable ASCII)\n",
                "mem <address> [count] | poke <address> <hex bytes...> | protect <address> <count>\n",
                "Image profile: IN/OUT 0xe9 connect to mock serial; HLT is a monitor stop.\n",
                "Counter, echo and memory use harness operations. Check selftest, factorials, loop, mandelbrot or extensions.").into(),
            ["quit"]|["exit"]=>return Ok(("bye".into(),true)),
            ["demo",name]=>{self.load_demo(name)?;format!("loaded {} at EIP={:08x}",self.demo,self.cpu.eip)}
            ["load",path]=>{
                let bytes=read_guest_image_file(path).map_err(|e|e.to_string())?;
                self.load_image(&bytes,path)?;format!("loaded {} guest bytes at EIP=00000100",bytes.len())
            }
            ["dosprobe",path]=>super::dos::report(&[path.to_string()],10000)?,
            ["dosprobe",path,budget]=>super::dos::report(&[path.to_string()],usize::try_from(number(budget)?).map_err(|_|"budget out of range")?)?,
            ["check"]=>self.check_current()?,
            ["pil"]=>self.current_pil(),
            ["list"]=>self.listing(self.cpu.eip,16)?,
            ["list",count]=>self.listing(self.cpu.eip,number(count)?)?,
            ["list",start,count]=>{
                let start=u32::try_from(number(start)?).map_err(|_|"listing address must fit 32 bits")?;
                self.listing(start,number(count)?)?
            }
            ["step"]|["s"]=>{
                let from=self.cpu.eip;let stop=self.step();
                format!("{}; EIP {from:08x} -> {:08x}",Self::stop_text(&stop),self.cpu.eip)
            }
            ["pilstep"]|["ps"]=>self.step_pil().0,
            ["run"]|["continue"]|["c"]=>{
                let (n,stop)=self.run(DEFAULT_RUN)?;format!("{n} completed steps; {}",Self::stop_text(&stop))
            }
            ["run",budget]|["continue",budget]|["c",budget]=>{
                let n=number(budget)?;
                if n==0 || n>MAX_RUN as u64 {return Err(format!("run budget must be 1..{MAX_RUN}"));}
                let (n,stop)=self.run(n as usize)?;format!("{n} completed steps; {}",Self::stop_text(&stop))
            }
            ["cache"]|["cache","stats"]=>self.cache.summary(),
            ["cache","on"]=>{self.cache.set_enabled(true);self.cache.summary()},
            ["cache","off"]=>{self.cache.set_enabled(false);self.cache.summary()},
            ["cache","clear"]=>{self.cache.clear();self.cache.summary()},
            ["time"]=>self.timed_run(MAX_RUN)?,
            ["time",budget]=>{let n=number(budget)?;if n==0||n>MAX_TIMED as u64{return Err(format!("timed budget must be 1..{MAX_TIMED}"));}self.timed_run(n as usize)?},
            ["bench"]=>benchmark_mode(1000,3,self.cache.enabled())?,
            ["bench",iterations]=>benchmark_mode(parse_count(iterations)?,3,self.cache.enabled())?,
            ["bench",iterations,samples]=>benchmark_mode(parse_count(iterations)?,parse_count(samples)?,self.cache.enabled())?,
            ["stress"]=>factorial_stress(25)?,
            ["stress",passes]=>factorial_stress(parse_count(passes)?)?,
            [cmd,addr,rest @ ..] if matches!(*cmd,"mem8"|"mem16"|"mem32")=>{
                if rest.len()>2{return Err("usage: mem8|mem16|mem32 ADDRESS [COUNT] [hex|dec]".into());}
                let width=match *cmd{"mem8"=>super::memory::MemWidth::Byte,"mem16"=>super::memory::MemWidth::Word,_=>super::memory::MemWidth::Dword};
                let address=u32::try_from(number(addr)?).map_err(|_|"address must fit 32 bits")?;
                let count=if let Some(count)=rest.first(){number(count)?}else{16/u64::from(width.bytes())};
                let decimal=match rest.get(1).copied().unwrap_or("hex"){"hex"=>false,"dec"=>true,_=>return Err("dump format must be hex or dec".into())};
                self.dump_memory(address,count,width,decimal)?
            }
            ["mem",addr]|["mem",addr,_]=>{
                let address=u32::try_from(number(addr)?).map_err(|_|"address must fit 32 bits")?;
                let count=if words.len()==3 {number(words[2])?}else{16};
                if count==0 || count>256 {return Err("memory count must be 1..256".into());}
                let start=usize::try_from(address).map_err(|_|"address out of range")?;
                let end=start.checked_add(count as usize).ok_or("address out of range")?;
                let data=self.ram.bytes().get(start..end).ok_or("RAM range out of bounds")?;
                format!("{address:08x}: {}",data.iter().map(|b|format!("{b:02x}")).collect::<Vec<_>>().join(" "))
            }
            ["poke",addr,bytes @ ..]=>{
                if bytes.is_empty() || bytes.len()>256 {return Err("supply 1..256 hex bytes".into());}
                let address=u32::try_from(number(addr)?).map_err(|_|"address must fit 32 bits")?;
                let data:Result<Vec<u8>,String>=bytes.iter().map(|text|u8::from_str_radix(text.strip_prefix("0x").unwrap_or(text),16).map_err(|_|"each token must be a hex byte".into())).collect();
                let data=data?;self.ram.write_bytes(address,&data).map_err(|f|format!("RAM write rejected: {f:?}"))?;
                format!("wrote {} bytes at {address:08x}",data.len())
            }
            ["protect",addr,count]=>{
                let address=u32::try_from(number(addr)?).map_err(|_|"address must fit 32 bits")?;
                let count=number(count)?;
                if count==0 || count>self.ram.bytes().len() as u64 {return Err("invalid protected range length".into());}
                self.ram.protect(address,count as usize).map_err(|f|format!("RAM protection rejected: {f:?}"))?;
                format!("protected {count} bytes at {address:08x}; reset restores demo RAM")
            }
            ["regs"]=>{
                let mut lines=REGS.iter().enumerate().map(|(i,name)|format!("{}={:08x}",name.to_ascii_uppercase(),self.cpu.regs[i])).collect::<Vec<_>>();
                lines.push(format!("EIP={:08x}",self.cpu.eip));lines.join(" ")
            }
            ["flags"]=>self.flags(),
            ["eip"]=>format!("EIP={:08x}",self.cpu.eip),
            ["set",name,value]=>{
                let value=u32::try_from(number(value)?).map_err(|_|"value must fit 32 bits")?;
                let name=name.to_ascii_lowercase();
                match name.as_str() {
                    "eip"=>self.cpu.eip=value,
                    "eflags"=>{self.cpu.eflags=value;self.cpu.undefined_flags=0;}
                    _=>{let i=REGS.iter().position(|r|*r==name.as_str()).ok_or("unknown register")?;self.cpu.regs[i]=value;}
                }
                self.stopped_at_break=None;format!("{name}={value:08x}")
            }
            ["break",addr]=>{
                let ip=u32::try_from(number(addr)?).map_err(|_|"address must fit 32 bits")?;
                if let Some(image)=&self.image {
                    if !image.contains_offset(ip){return Err("breakpoint outside image".into());}
                } else if !self.instructions.contains_key(&ip) {return Err("no instruction at breakpoint address".into());}
                self.breakpoints.insert(ip);format!("breakpoint at {ip:08x}")
            }
            ["delete",addr]=>{
                let ip=u32::try_from(number(addr)?).map_err(|_|"address must fit 32 bits")?;
                self.breakpoints.remove(&ip);self.stopped_at_break=None;format!("removed breakpoint at {ip:08x}")
            }
            ["breaks"]=>if self.breakpoints.is_empty() {"no breakpoints".into()}
                else {self.breakpoints.iter().map(|ip|format!("{ip:08x}")).collect::<Vec<_>>().join(" ")},
            ["reset"]=>{self.reset();format!("reset {} to EIP={:08x}; RAM/serial reset; breakpoints retained",self.demo,self.cpu.eip)}
            ["serial","status"]=>{
                let (rx,tx)=self.serial.lengths();let (rc,tc)=self.serial.capacities();
                format!("RX={rx}/{rc} ready={} TX={tx}/{tc}",self.serial.rx_ready())
            }
            ["serial","tx"]=>{
                let bytes=self.serial.take_tx();
                if bytes.is_empty() {"TX: empty".into()} else {format!("TX: {}",bytes.iter().map(|b|format!("{b:02x}")).collect::<Vec<_>>().join(" "))}
            }
            ["serial","clear"]=>{self.serial.clear();"serial queues cleared".into()}
            ["serial","rx",bytes @ ..]=>{
                if bytes.is_empty() {return Err("supply hex bytes, e.g. serial rx 41 00 ff".into());}
                let data:Result<Vec<u8>,String>=bytes.iter().map(|text|{
                    let text=text.strip_prefix("0x").unwrap_or(text);
                    u8::from_str_radix(text,16).map_err(|_|"each RX token must be a hex byte".into())
                }).collect();
                let data=data?;self.serial.inject_rx(&data).map_err(|_|"RX queue capacity exceeded; nothing queued")?;
                format!("queued {} RX bytes",data.len())
            }
            _=>return Err("unknown command or arguments; type help".into()),
        };
        let response=if show_guest_output && self.tx_events.len()>tx_before {
            let bytes=&self.tx_events[tx_before..];
            let hex=bytes.iter().map(|b|format!("{b:02x}")).collect::<Vec<_>>().join(" ");
            let text=bytes.iter().flat_map(|b|std::ascii::escape_default(*b)).map(char::from).collect::<String>();
            format!("{response}\nserial output: {hex} | {text}")
        }else{response};
        Ok((response,false))
    }
}

pub(crate) fn read_guest_image_file(path:&str)->io::Result<Vec<u8>> {
    use std::io::Read;
    const MAX_RAW:usize=super::guest::RAM_BYTES-super::guest::ENTRY as usize;
    const MAX_HEX_TEXT:usize=65536;
    let file=std::fs::File::open(path)?;
    if path.to_ascii_lowercase().ends_with(".hex"){
        let mut raw=Vec::new();
        file.take((MAX_HEX_TEXT+1)as u64).read_to_end(&mut raw)?;
        if raw.len()>MAX_HEX_TEXT{
            return Err(io::Error::new(io::ErrorKind::InvalidData,"hex file exceeds 64 KiB text limit"));
        }
        let text=std::str::from_utf8(&raw).map_err(|e|io::Error::new(io::ErrorKind::InvalidData,e))?;
        let mut bytes=Vec::new();
        for line in text.lines(){
            let clean=line.split('#').next().unwrap_or("").split(';').next().unwrap_or("");
            for token in clean.split_whitespace(){
                let token=token.strip_prefix("0x").or_else(||token.strip_prefix("0X")).unwrap_or(token);
                bytes.push(u8::from_str_radix(token,16).map_err(|e|io::Error::new(io::ErrorKind::InvalidData,e))?);
                if bytes.len()>MAX_RAW{break;}
            }
            if bytes.len()>MAX_RAW{break;}
        }
        Ok(bytes)
    }else{
        let mut bytes=Vec::new();
        file.take((MAX_RAW+1)as u64).read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}
pub fn repl<R:BufRead,W:Write>(input:R,output:W)->io::Result<()> {
    let debugger=Debugger::new("counter").map_err(|e|io::Error::new(io::ErrorKind::Other,e))?;
    repl_with(input,output,debugger)
}
pub fn repl_image<R:BufRead,W:Write>(input:R,output:W,path:&str)->io::Result<()> {
    let mut debugger=Debugger::new("counter").map_err(|e|io::Error::new(io::ErrorKind::Other,e))?;
    let bytes=read_guest_image_file(path)?;
    debugger.load_image(&bytes,path).map_err(|e|io::Error::new(io::ErrorKind::Other,e))?;
    repl_with(input,output,debugger)
}
fn repl_with<R:BufRead,W:Write>(mut input:R,mut output:W,mut debugger:Debugger)->io::Result<()> {
    writeln!(output,"PIL debugger: PIL demos and limited 386 binary monitor. Type help.")?;
    let mut line=String::new();
    loop {
        write!(output,"pil> ")?;output.flush()?;line.clear();
        if input.read_line(&mut line)?==0 {break;}
        match debugger.command(line.trim()) {
            Ok((text,quit))=>{
                if !text.is_empty(){writeln!(output,"{text}")?;}
                if quit{break;}
            }
            Err(e)=>writeln!(output,"error: {e}")?,
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/debugger.rs"]
mod tests;

pub fn selftest()->Result<(),String> {
    let mut d=Debugger::new("selftest")?;d.serial.inject_rx(&[0x41]).map_err(|_|"RX injection failed")?;
    let (steps,stop)=d.run(MAX_RUN)?;
    if !matches!(stop,Stop::Halt(_)){return Err(format!("selftest stopped: {}",Debugger::stop_text(&stop)));}
    d.check_selftest()?;println!("PASS: selftest, {steps} completed steps; DX=600D; RAM signatures; mock TX=41");Ok(())
}

fn parse_count(text:&str)->Result<u32,String> {
    u32::try_from(number(text)?).map_err(|_|"count must fit 32 bits".into())
}
pub fn benchmark(iterations:u32,samples:u32)->Result<String,String> {benchmark_mode(iterations,samples,true)}
pub fn benchmark_mode(iterations:u32,samples:u32,cached:bool)->Result<String,String> {benchmark_details(iterations,samples,cached).map(|x|x.0)}
pub fn benchmark_details(iterations:u32,samples:u32,cached:bool)->Result<(String,f64),String> {
    let bytes=super::guest::loop_bytes(iterations)?;
    if !(1..=9).contains(&samples){return Err("benchmark samples must be 1..9".into());}
    let steps=3*iterations as usize+3;let mut timings=Vec::new();let mut stats=super::cache::Stats::default();
    for sample in 0..=samples {
        let mut d=Debugger::new("counter")?;d.load_image(&bytes,"benchmark")?;d.cache.set_enabled(cached);
        let start=std::time::Instant::now();let (done,stop)=d.run_large(steps+1)?;let elapsed=start.elapsed();
        if done!=steps || !matches!(stop,Stop::Halt(_)){return Err(format!("benchmark execution stopped: {}",Debugger::stop_text(&stop)));}
        d.check_loop(iterations)?;
        if sample!=0 {timings.push(elapsed.as_secs_f64());stats=d.cache.stats();}
    }
    let values=timings.iter().map(|s|format!("{:.3}",s*1000.0)).collect::<Vec<_>>().join(", ");
    timings.sort_by(|a,b|a.partial_cmp(b).unwrap());let mid=timings.len()/2;
    let median=if timings.len()%2==0{(timings[mid-1]+timings[mid])/2.0}else{timings[mid]};
    let rate=if median==0.0{0.0}else{steps as f64/median};
    Ok((format!("PASS: counted-loop benchmark; {iterations} iterations; {steps} completed instructions/sample\nsamples_ms=[{values}]; median={:.3} ms; {rate:.0} guest instructions/s\n{samples} measured samples, 1 excluded warmup; fresh CPU/RAM per sample.\nEngine=PIL-token-interpreter cache={} (cold per sample); last_sample_hits={} misses={} invalidations={}.\nHost wall time includes fetch validation, misses/compilation and interpreter dispatch; excludes image setup, checking and console output. Not 386 cycles. version={} host={}/{} profile={}",median*1000.0,if cached{"on"}else{"off"},stats.hits,stats.misses,stats.invalidations,env!("CARGO_PKG_VERSION"),std::env::consts::OS,std::env::consts::ARCH,if cfg!(debug_assertions){"debug"}else{"release"}),median))
}
pub fn factorial_stress(passes:u32)->Result<String,String> {
    if !(1..=1000).contains(&passes){return Err("stress passes must be 1..1000".into());}
    let start=std::time::Instant::now();let mut completed=0;
    for _ in 0..passes {
        let mut d=Debugger::new("factorials")?;let (steps,stop)=d.run(1000)?;
        if steps!=336 || !matches!(stop,Stop::Halt(_)){return Err("factorial stress execution mismatch".into());}
        d.check_factorials()?;completed+=steps;
    }
    Ok(format!("PASS: factorial stress; {passes} fresh runs; {} results verified; {completed} completed instructions\nelapsed={:.3} ms including setup and result verification; host wall time, not 386 cycles",12*passes,start.elapsed().as_secs_f64()*1000.0))
}
pub fn numeric_test()->Result<(),String> {
    let mut d=Debugger::new("factorials")?;let (steps,stop)=d.run(1000)?;
    if steps!=336 || !matches!(stop,Stop::Halt(_)){return Err("factorial execution mismatch".into());}
    println!("{}",d.check_factorials()?);println!("{}",benchmark(16,2)?);println!("{}",factorial_stress(2)?);Ok(())
}

// Exact architectural/transport trace used to check emitted Rust execution.
pub fn backend_trace(cases:&[super::native::Case])->Result<Vec<u8>,String> {
    let mut out=Vec::new();
    for (id,case) in cases.iter().enumerate() {
        let mut d=Debugger::new("counter")?;d.load_image(&case.bytes,case.name)?;d.image=Some(case.image.clone());
        d.cpu.regs=case.regs;d.cpu.eflags=case.flags;
        if let Some((at,bytes))=&case.patch {d.ram.write_bytes(*at,bytes).map_err(|e|format!("{e:?}"))?;}
        if let Some((at,len))=case.protect {d.ram.protect(at,len).map_err(|e|format!("{e:?}"))?;}
        d.serial.inject_rx(&case.rx).map_err(|e|format!("{e:?}"))?;
        for b in &case.tx {d.serial.try_write(*b).map_err(|e|format!("{e:?}"))?;}
        let mut stopped=false;
        for _ in 0..1000 {
            let stop=d.step();let status=match stop {
                Stop::Advanced=>[0,0,0,0,0],Stop::Halt(ip)=>[1,ip,0,0,0],
                Stop::WaitingRx(ip)=>[2,ip,0,0,0],Stop::TxBlocked(ip)=>[3,ip,0,0,0],
                Stop::Fault(e)=>super::trace::fault(d.cpu.eip,e),
                Stop::Decode{eip,..}=>[7,eip,0,0,0],_=>return Err("unexpected verification stop".into()),
            };
            let (rx,tx)=d.serial.snapshot();
            super::trace::record_machine(&mut out,id as u32,status,&d.cpu.regs,d.cpu.eflags,d.cpu.undefined_flags,d.cpu.eip,d.ram.bytes(),&rx,&tx,&d.image.as_ref().unwrap().segment_state());
            if status[0]!=0 {stopped=true;break;}
        }
        if !stopped{return Err(format!("trace budget exhausted for {}",case.name));}
    }
    Ok(out)
}

pub fn render_mandelbrot(size:u32,iterations:u32)->Result<super::mandelbrot::Render,String>{
    render_mandelbrot_checked(size,iterations,false)
}
pub fn render_mandelbrot_checked(size:u32,iterations:u32,capture:bool)->Result<super::mandelbrot::Render,String>{
    let bytes=super::mandelbrot::guest_bytes(size,iterations)?;let mut d=Debugger::new("counter")?;d.load_image(&bytes,"mandelbrot render")?;
    let expected_pixels=(size*size)as usize;let budget=u64::from(size)*u64::from(size)*(u64::from(iterations)*26+40);
    let mut checkpoints=Vec::new();
    let(mut pixels,mut steps,mut blocked)=(Vec::new(),0u64,0u32);let mut lazy=super::LazyFlags::None;let start=std::time::Instant::now();
    loop{
        if steps>budget{return Err("Mandelbrot instruction budget exhausted".into());}
        let stop=d.step_image_lazy(&mut lazy);
        if stop!=Stop::Advanced{lazy.flush(&mut d.cpu.eflags,&mut d.cpu.undefined_flags);}
        if capture && matches!(stop,Stop::TxBlocked(_)|Stop::Halt(_)){
            let (rx,tx)=d.serial.snapshot();let kind=if matches!(stop,Stop::Halt(_)){1}else{3};
            super::trace::record_machine(&mut checkpoints,0,[kind,d.cpu.eip,0,0,0],&d.cpu.regs,d.cpu.eflags,d.cpu.undefined_flags,d.cpu.eip,d.ram.bytes(),&rx,&tx,&d.image.as_ref().unwrap().segment_state());
        }
        match stop{
            Stop::Advanced=>steps+=1,
            Stop::TxBlocked(_)=>{let chunk=d.serial.take_tx();if chunk.is_empty(){return Err("empty blocked TX queue".into());}pixels.extend(chunk);blocked+=1;},
            Stop::Halt(_)=>{pixels.extend(d.serial.take_tx());break;},
            stop=>return Err(format!("Mandelbrot stopped: {}",Debugger::stop_text(&stop))),
        }
        if d.tx_events.len()>expected_pixels{return Err("too many Mandelbrot pixels".into());}
    }
    let seconds=start.elapsed().as_secs_f64();
    let reference=super::mandelbrot::reference(size,iterations)?;
    if pixels!=reference{return Err(format!("Mandelbrot pixel mismatch: expected {}, received {}",reference.len(),pixels.len()));}
    Ok(super::mandelbrot::Render{pixels,steps,seconds,blocked,cache:d.cache.summary(),checkpoints})
}
