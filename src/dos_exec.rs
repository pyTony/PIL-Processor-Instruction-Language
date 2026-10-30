//! INT 21 AH4B EXEC (load and execute, AL=00h) and returning to the parent
//! when the child terminates, after FreeDOS task.c: the child gets its own
//! environment and program blocks, a PSP with the parent's handles, command
//! tail and FCBs, and the parent resumes after its INT 21 with CF=0.
use super::*;
/// What the parent needs back when its child ends.
#[derive(Clone,Debug)]
pub struct Frame{cpu:Cpu,selectors:[u16;6],image:Image,open_handles:u32,dta:(u16,u16),psp:u16}
/// Nesting limit for programs started by programs.
const MAX_DEPTH:usize=8;
fn fail(p:&mut Loaded,code:u32){p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|code;p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;}
fn word(ram:&Ram,at:u32)->Result<u16,String>{ram.load(at,MemWidth::Word).map(|w|w as u16).map_err(|e|format!("EXEC read: {e:?}"))}
fn bytes(ram:&Ram,at:u32,n:u32)->Result<Vec<u8>,String>{(0..n).map(|i|ram.load(at+i,MemWidth::Byte).map(|b|b as u8).map_err(|e|format!("EXEC read: {e:?}"))).collect()}
/// Far pointer at a linear address, as a linear address.
fn far(ram:&Ram,at:u32)->Result<u32,String>{Ok(u32::from(word(ram,at+2)?)*16+u32::from(word(ram,at)?))}
/// Allocates `paragraphs` through AH48 with the caller's registers kept.
fn allocate(p:&mut Loaded,paragraphs:u16)->Result<Option<u16>,String>{
    let saved=(p.cpu.regs[0],p.cpu.regs[3],p.cpu.eflags,p.cpu.undefined_flags);
    p.cpu.regs[3]=(p.cpu.regs[3]&0xffff0000)|u32::from(paragraphs);memory_service(p,0x48)?;
    let segment=(p.cpu.eflags&1==0).then_some(p.cpu.regs[0]as u16);
    (p.cpu.regs[0],p.cpu.regs[3],p.cpu.eflags,p.cpu.undefined_flags)=saved;
    Ok(segment)
}
/// The program as DOS loads it: module bytes (relocations applied later),
/// relocation entries, paragraphs it needs beyond the module (min, max), the
/// entry CS:IP and SS:SP relative to the load segment, or COM.
struct Program{module:Vec<u8>,relocations:Vec<(u16,u16)>,min:u16,max:u16,entry:Option<(u16,u16,u16,u16)>}
fn parse(bytes:&[u8])->Option<Program>{
    if !(bytes.starts_with(b"MZ")||bytes.starts_with(b"ZM")){
        return (!bytes.is_empty()&&bytes.len()<=0xff00).then(||Program{module:bytes.to_vec(),relocations:Vec::new(),min:0,max:0xffff,entry:None});
    }
    let w=|at:usize|bytes.get(at..at+2).map(|b|u16::from_le_bytes([b[0],b[1]]));
    let(last,pages,count,header,min,max,ss,sp,ip,cs,table)=(w(2)?,w(4)?,w(6)?,w(8)?,w(10)?,w(12)?,w(14)?,w(16)?,w(20)?,w(22)?,w(24)?);
    if pages==0||last>511{return None;}
    let declared=(usize::from(pages)-1)*512+if last==0{512}else{usize::from(last)};
    let header=usize::from(header)*16;
    if header<28||header>=declared||declared>bytes.len()||min>max{return None;}
    let relocations=(0..usize::from(count)).map(|i|Some((w(usize::from(table)+i*4+2)?,w(usize::from(table)+i*4)?))).collect::<Option<Vec<_>>>()?;
    Some(Program{module:bytes[header..declared].to_vec(),relocations,min,max,entry:Some((cs,ip,ss,sp))})
}
pub(super) fn exec(p:&mut Loaded)->Result<(),String>{
    let al=p.cpu.regs[0]as u8;
    if al!=0{return Err(format!("EXEC AL={al:02x}h (load without running or overlay) is not modelled"));}
    if p.exec.len()>=MAX_DEPTH{fail(p,8);return Ok(());}
    let raw=files::asciiz(p,p.cpu.regs[2]&0xffff)?;
    let(file,path)=match files::program(p,&raw)?{Ok(found)=>found,Err(code)=>{fail(p,u32::from(code));return Ok(())}};
    let Some(program)=parse(&file)else{fail(p,11);return Ok(())};
    let block=u32::from(p.image.selectors[SegmentRegister::Es.index()])*16+(p.cpu.regs[3]&0xffff);
    let ram=&p.ram;
    let env_seg=word(ram,block)?;let tail=far(ram,block+2)?;let fcb1=far(ram,block+6)?;let fcb2=far(ram,block+10)?;
    let parent=u32::from(p.psp)*16;
    // Environment: the given block or the parent's, its strings up to the
    // empty one, then the count word 1 and the program's full path.
    let source=u32::from(if env_seg==0{word(ram,parent+0x2c)?}else{env_seg})*16;
    let mut env=Vec::new();
    loop{
        let b=bytes(ram,source+env.len()as u32,1)?[0];env.push(b);
        if env.len()>=2&&env[env.len()-2..]==[0,0]{break;}
        if env.len()>=0x8000{fail(p,10);return Ok(());}
    }
    if env.len()==1{env.push(0);}
    env.extend([1,0]);env.extend(&path);env.push(0);
    let tail_bytes={let n=bytes(ram,tail,1)?[0].min(126);let mut t=bytes(ram,tail,u32::from(n)+1)?;t.push(0x0d);t};
    let(fcb1,fcb2)=(bytes(ram,fcb1,16)?,bytes(ram,fcb2,16)?);
    // Memory: the environment first, then the program block; a COM file takes
    // the largest free block, an EXE what fits between its minimum and maximum.
    let resident=(program.module.len().div_ceil(16)+0x10)as u32;
    let Some(env_block)=allocate(p,(env.len().div_ceil(16))as u16)? else{fail(p,8);return Ok(())};
    let largest=u32::from(p.arena.free());
    let wanted=match program.entry{None=>largest,Some(_)=>(resident+u32::from(program.max)).min(largest)};
    if largest<resident+u32::from(program.min)||(program.entry.is_none()&&largest<resident+0x10){
        p.arena.set_owner(&mut p.ram,env_block,0)?;fail(p,8);return Ok(());
    }
    let Some(child)=allocate(p,wanted as u16)? else{p.arena.set_owner(&mut p.ram,env_block,0)?;fail(p,8);return Ok(())};
    p.arena.set_owner(&mut p.ram,env_block,child)?;p.arena.set_owner(&mut p.ram,child,child)?;
    p.ram.write_bytes(u32::from(env_block)*16,&env).map_err(|e|format!("EXEC environment: {e:?}"))?;
    // The PSP: INT 20h, the block end, the parent's INT 22h/23h/24h targets,
    // parent PSP, a copy of the first 20 handles, environment, 20-entry table.
    let base=u32::from(child)*16;let end=child+wanted as u16;
    let mut psp=[0u8;256];psp[..2].copy_from_slice(&[0xcd,0x20]);psp[2..4].copy_from_slice(&end.to_le_bytes());
    let return_ip=(p.cpu.eip+2)&0xffff;let return_cs=p.selectors[SegmentRegister::Cs.index()];
    psp[0x0a..0x0c].copy_from_slice(&(return_ip as u16).to_le_bytes());psp[0x0c..0x0e].copy_from_slice(&return_cs.to_le_bytes());
    for(at,vector)in [(0x0e,0x23u32),(0x12,0x24)]{psp[at..at+4].copy_from_slice(&(p.ram.load(vector*4,MemWidth::Dword).map_err(|e|format!("EXEC vector read: {e:?}"))?).to_le_bytes());}
    psp[0x16..0x18].copy_from_slice(&p.psp.to_le_bytes());
    let mut handles=0u32;
    for h in 0..20u16{
        let entry=if p.open_handles&(1<<h)!=0{p.ram.load(services::jft(p,h)?,MemWidth::Byte).map_err(|e|format!("EXEC JFT read: {e:?}"))?as u8}else{0xff};
        psp[0x18+usize::from(h)]=entry;if entry!=0xff{handles|=1<<h;}
    }
    psp[0x2c..0x2e].copy_from_slice(&env_block.to_le_bytes());
    psp[0x32..0x34].copy_from_slice(&20u16.to_le_bytes());psp[0x34..0x36].copy_from_slice(&0x18u16.to_le_bytes());psp[0x36..0x38].copy_from_slice(&child.to_le_bytes());
    psp[0x40..0x42].copy_from_slice(&bytes(&p.ram,parent+0x40,2)?);
    psp[0x50..0x53].copy_from_slice(&[0xcd,0x21,0xcb]);
    psp[0x5c..0x6c].copy_from_slice(&fcb1);psp[0x6c..0x7c].copy_from_slice(&fcb2);
    psp[0x80..0x80+tail_bytes.len()].copy_from_slice(&tail_bytes);
    p.ram.write_bytes(base,&psp).map_err(|e|format!("EXEC PSP: {e:?}"))?;
    let load=child+0x10;let mut module=program.module;
    for(segment,offset)in &program.relocations{
        let at=usize::from(*segment)*16+usize::from(*offset);
        let Some(value)=module.get(at..at+2).map(|b|u16::from_le_bytes([b[0],b[1]]))else{p.arena.release(&mut p.ram,child,None)?;fail(p,11);return Ok(())};
        module[at..at+2].copy_from_slice(&value.wrapping_add(load).to_le_bytes());
    }
    p.ram.write_bytes(u32::from(load)*16,&module).map_err(|e|format!("EXEC module: {e:?}"))?;
    // Save the parent, then enter the child with DS=ES=PSP.
    p.exec.push(Frame{cpu:p.cpu.clone(),selectors:p.selectors,image:p.image.clone(),open_handles:p.open_handles,dta:p.dta,psp:p.psp});
    let(cs,ip,ss,sp)=match program.entry{
        Some((cs,ip,ss,sp))=>(load.wrapping_add(cs),ip,load.wrapping_add(ss),sp),
        None=>{
            let sp=if wanted>=0x1000{0xfffe}else{(wanted*16-2)as u16};
            p.ram.store(base+u32::from(sp),MemWidth::Word,0).map_err(|e|format!("EXEC stack: {e:?}"))?;
            (child,0x100,child,sp)
        },
    };
    let mut selectors=[child;6];selectors[SegmentRegister::Cs.index()]=cs;selectors[SegmentRegister::Ss.index()]=ss;
    p.selectors=selectors;p.image.selectors=selectors;p.image.segments=selectors.map(SegmentCache::real_mode);
    p.image.fetch_base=base;p.image.start=0;p.image.end=u32::from(end)*16-base;
    // The service loop steps past the INT 21 it returns from; start 2 bytes early.
    p.cpu.eip=u32::from(ip.wrapping_sub(2));p.cpu.regs[4]=u32::from(sp);p.cpu.regs[0]=0;p.cpu.regs[3]=0;p.cpu.regs[1]=0;
    p.cpu.eflags=0x202;
    p.open_handles=handles;p.dta=(child,0x80);p.psp=child;
    Ok(())
}
/// The child ended with `code` (`kind` 0 normal, 3 resident): free what it
/// owned, close its files and resume the parent after its INT 21.
pub(super) fn terminate(p:&mut Loaded,code:u8,kind:u8)->Result<(),String>{
    let frame=p.exec.pop().ok_or("terminate without a parent")?;
    let keep=p.resident.take().map(|_|p.psp);
    p.arena.release(&mut p.ram,p.psp,keep)?;
    files::close_child_files(p,frame.open_handles);
    p.return_code=u16::from(kind)<<8|u16::from(code);
    p.cpu=frame.cpu;p.cpu.eip=(p.cpu.eip+2)&0xffff;p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;
    p.selectors=frame.selectors;p.image=frame.image;p.image.retired();
    p.open_handles=frame.open_handles;p.dta=frame.dta;p.psp=frame.psp;
    Ok(())
}
