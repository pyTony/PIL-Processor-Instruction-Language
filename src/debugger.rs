//! Host REPL for predecoded PIL programs, not arbitrary x86 binaries.
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, BufRead, Write};
use super::{compile, description, execute_flow, instantiate, number, Cpu, Program, View};
use super::flow::{ExecError, FlowContext};
use super::serial::{MockSerial, SerialTransport};

const MAX_RUN: usize = 10_000;
const DEFAULT_RUN: usize = 100;
const REGS: [&str;8] = ["eax","ecx","edx","ebx","esp","ebp","esi","edi"];

#[derive(Clone, Debug)]
enum Action {
    Cpu { program: Program, dst: View, src: View, context: FlowContext },
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
}

struct Debugger {
    cpu: Cpu,
    initial: Cpu,
    instructions: BTreeMap<u32, Instruction>,
    breakpoints: BTreeSet<u32>,
    stopped_at_break: Option<u32>,
    serial: MockSerial,
    demo: String,
}

impl Debugger {
    fn new(demo: &str) -> Result<Self,String> {
        let mut d=Self {
            cpu: Cpu::default(), initial: Cpu::default(),
            instructions: BTreeMap::new(), breakpoints: BTreeSet::new(),
            stopped_at_break: None, serial: MockSerial::new(256,256), demo: String::new(),
        };
        d.load_demo(demo)?; Ok(d)
    }
    fn load_demo(&mut self, name: &str) -> Result<(),String> {
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
            _ => return Err("demo must be counter or echo".into()),
        }
        self.instructions=code;self.cpu=cpu.clone();self.initial=cpu;
        self.breakpoints.clear();self.stopped_at_break=None;self.serial.clear();self.demo=name.into();
        Ok(())
    }
    fn reset(&mut self) {
        self.cpu=self.initial.clone();self.serial.clear();self.stopped_at_break=None;
        // Keep breakpoints while resetting the currently loaded demo.
    }
    fn step(&mut self) -> Stop {
        self.stopped_at_break=None;
        let instruction=match self.instructions.get(&self.cpu.eip) {
            Some(i)=>i, None=>return Stop::MissingInstruction(self.cpu.eip),
        };
        match &instruction.action {
            Action::Cpu{program,dst,src,context} => match execute_flow(program,&mut self.cpu,*dst,*src,*context) {
                Ok(())=>Stop::Advanced,Err(e)=>Stop::Fault(e),
            },
            Action::MockRx{next} => {
                let byte=match self.serial.try_read() {Some(b)=>b,None=>return Stop::WaitingRx(self.cpu.eip)};
                self.cpu.regs[0]=(self.cpu.regs[0]&!255)|u32::from(byte);
                self.cpu.eip=*next;Stop::Advanced
            }
            Action::MockTx{next} => {
                if self.serial.try_write(self.cpu.regs[0] as u8).is_err() {
                    return Stop::TxBlocked(self.cpu.eip);
                }
                self.cpu.eip=*next;Stop::Advanced
            }
            Action::HarnessHalt=>Stop::Halt(self.cpu.eip),
        }
    }
    fn run(&mut self, budget:usize)->Result<(usize,Stop),String> {
        if budget==0 || budget>MAX_RUN {return Err(format!("run budget must be 1..{MAX_RUN}"));}
        let resume=self.stopped_at_break.take().filter(|eip|*eip==self.cpu.eip);
        for done in 0..budget {
            if self.breakpoints.contains(&self.cpu.eip) && !(done==0 && resume==Some(self.cpu.eip)) {
                self.stopped_at_break=Some(self.cpu.eip);
                return Ok((done,Stop::Breakpoint(self.cpu.eip)));
            }
            let stop=self.step();
            if stop!=Stop::Advanced {return Ok((done,stop));}
        }
        Ok((budget,Stop::Budget(self.cpu.eip)))
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
            Stop::Breakpoint(ip)=>format!("breakpoint before EIP={ip:08x}"),
            Stop::Budget(ip)=>format!("budget exhausted at EIP={ip:08x}"),
            Stop::WaitingRx(ip)=>format!("RX empty; waiting at EIP={ip:08x}"),
            Stop::TxBlocked(ip)=>format!("TX full; waiting at EIP={ip:08x}"),
            Stop::Halt(ip)=>format!("harness halted at EIP={ip:08x}"),
            Stop::MissingInstruction(ip)=>format!("no predecoded instruction at EIP={ip:08x}"),
            Stop::Fault(ExecError::GeneralProtection{eip,error_code})=>format!("#GP({error_code}) at EIP={eip:08x}; instruction not committed"),
            Stop::Fault(e)=>format!("execution error: {e:?}"),
        }
    }
    fn command(&mut self,line:&str)->Result<(String,bool),String> {
        let words:Vec<_>=line.split_whitespace().collect();
        let response=match words.as_slice() {
            []=>String::new(),
            ["help"]=>concat!(
                "help | demo counter|echo | list | step | run [1..10000] | continue [budget]\n",
                "regs | flags | eip | set <register|eip|eflags> <decimal|0xhex>\n",
                "break <address> | delete <address> | breaks | reset | quit\n",
                "serial status | serial rx <hex bytes...> | serial tx | serial clear\n",
                "MOCK_RX/TX and HARNESS_HALT are harness operations, not x86 I/O instructions.").into(),
            ["quit"]|["exit"]=>return Ok(("bye".into(),true)),
            ["demo",name]=>{self.load_demo(name)?;format!("loaded {} at EIP={:08x}",self.demo,self.cpu.eip)}
            ["list"]=>{
                self.instructions.iter().map(|(ip,i)|format!("{} {ip:08x} {}",
                    if *ip==self.cpu.eip {"*"}else{" "},i.label)).collect::<Vec<_>>().join("\n")
            }
            ["step"]|["s"]=>{
                let from=self.cpu.eip;let stop=self.step();
                format!("{}; EIP {from:08x} -> {:08x}",Self::stop_text(&stop),self.cpu.eip)
            }
            ["run"]|["continue"]|["c"]=>{
                let (n,stop)=self.run(DEFAULT_RUN)?;format!("{n} completed steps; {}",Self::stop_text(&stop))
            }
            ["run",budget]|["continue",budget]|["c",budget]=>{
                let n=number(budget)?;
                if n==0 || n>MAX_RUN as u64 {return Err(format!("run budget must be 1..{MAX_RUN}"));}
                let (n,stop)=self.run(n as usize)?;format!("{n} completed steps; {}",Self::stop_text(&stop))
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
                if !self.instructions.contains_key(&ip) {return Err("no instruction at breakpoint address".into());}
                self.breakpoints.insert(ip);format!("breakpoint at {ip:08x}")
            }
            ["delete",addr]=>{
                let ip=u32::try_from(number(addr)?).map_err(|_|"address must fit 32 bits")?;
                self.breakpoints.remove(&ip);self.stopped_at_break=None;format!("removed breakpoint at {ip:08x}")
            }
            ["breaks"]=>if self.breakpoints.is_empty() {"no breakpoints".into()}
                else {self.breakpoints.iter().map(|ip|format!("{ip:08x}")).collect::<Vec<_>>().join(" ")},
            ["reset"]=>{self.reset();format!("reset {} to EIP={:08x}; serial queues cleared; breakpoints retained",self.demo,self.cpu.eip)}
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
        Ok((response,false))
    }
}

pub fn repl<R:BufRead,W:Write>(mut input:R,mut output:W)->io::Result<()> {
    let mut debugger=Debugger::new("counter").map_err(|e|io::Error::new(io::ErrorKind::Other,e))?;
    writeln!(output,"PIL debugger: predecoded demos; no x86 binary loader. Type help.")?;
    let mut line=String::new();
    loop {
        write!(output,"pil> ")?;output.flush()?;line.clear();
        if input.read_line(&mut line)?==0 {break;}
        match debugger.command(line.trim()) {
            Ok((text,quit))=>{if !text.is_empty(){writeln!(output,"{text}")?;}if quit{break;}}
            Err(e)=>writeln!(output,"error: {e}")?,
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/debugger.rs"]
mod tests;
