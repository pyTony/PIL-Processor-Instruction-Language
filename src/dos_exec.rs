//! INT 21 AH4B EXEC (load and execute, AL=00h; load overlay, AL=03h) and returning to the parent
//! when the child terminates, after FreeDOS task.c: the child gets its own
//! environment and program blocks, a PSP with the parent's handles, command
//! tail and FCBs, and the parent resumes after its INT 21 with CF=0.
use super::*;
/// What the parent needs back when its child ends.
#[derive(Clone,Debug)]
pub struct Frame{cpu:Cpu,selectors:[u16;6],pub(super) image:Image,open_handles:u32,dta:(u16,u16),psp:u16}
/// AH=55h installs a PSP without suspending its caller. In particular the
/// CPU at the AH55 call is NOT the continuation after the child exits.
/// DOS uses the child PSP's INT22 vector for that return.
#[derive(Clone,Debug)]
pub struct InMemoryChild {
    child:u16,parent:u16,exec_depth:usize,
    open_handles:u32,dta:(u16,u16),
    system:[(u32,u32);SYSTEM_RANGES],
}
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
    // Whole pages up to the end of the file, as the first program loads.
    let loaded=(usize::from(pages)*512).min(bytes.len());
    Some(Program{module:bytes[header..loaded].to_vec(),relocations,min,max,entry:Some((cs,ip,ss,sp))})
}
/// Load an overlay into memory supplied by the caller. DOS does not allocate
/// a PSP or transfer control for AL=03h. Relocations add the independent
/// factor from ES:BX+2, which need not equal the load segment.
fn overlay(p:&mut Loaded,file:&[u8])->Result<(),String>{
    let mz=file.starts_with(b"MZ")||file.starts_with(b"ZM");
    let mut program=if mz{match parse(file){Some(program)=>program,None=>{fail(p,11);return Ok(());}}}
        else{Program{module:file.to_vec(),relocations:Vec::new(),min:0,max:0,entry:None}};
    if mz{
        let header=usize::from(u16::from_le_bytes([file[8],file[9]]))*16;
        let table=usize::from(u16::from_le_bytes([file[24],file[25]]));
        if !program.relocations.is_empty()&&(table<28||table+program.relocations.len()*4>header){fail(p,11);return Ok(());}
    }
    let block=u32::from(p.image.selectors[SegmentRegister::Es.index()])*16+(p.cpu.regs[3]&0xffff);
    let load=word(&p.ram,block)?;let factor=word(&p.ram,block+2)?;
    for(segment,offset)in &program.relocations{
        let at=usize::from(*segment)*16+usize::from(*offset);
        let Some(value)=program.module.get(at..at+2).map(|b|u16::from_le_bytes([b[0],b[1]]))else{fail(p,11);return Ok(())};
        program.module[at..at+2].copy_from_slice(&value.wrapping_add(factor).to_le_bytes());
    }
    let start=u32::from(load)*16;let end=start.checked_add(program.module.len()as u32).ok_or("EXEC overlay address overflow")?;
    let in_window=start>=p.image.fetch_base+p.image.start&&end<=p.image.fetch_base+p.image.end;
    let in_system=p.image.system.iter().any(|&(a,b)|start>=a&&end<=b&&a<b);
    let slot=if !program.module.is_empty()&&!in_window&&!in_system{
        let overlap=p.image.system.iter().position(|&(a,b)|a<b&&start<=b&&a<=end);
        Some(overlap.or_else(||p.image.system.iter().position(|&(a,b)|a==0&&b==0)).ok_or("EXEC overlay code range capacity exhausted")?)
    }else{None};
    // write_bytes validates the entire range and protection before changing RAM.
    p.ram.write_bytes(start,&program.module).map_err(|e|format!("EXEC overlay write: {e:?}"))?;
    if let Some(i)=slot{let(a,b)=p.image.system[i];p.image.system[i]=if a<b{(a.min(start),b.max(end))}else{(start,end)};}
    p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;Ok(())
}
pub(super) fn exec(p:&mut Loaded)->Result<(),String>{
    let al=p.cpu.regs[0]as u8;
    if !matches!(al,0|3){return Err(format!("EXEC AL={al:02x}h (load without running) is not modelled"));}
    if al==3{
        let raw=files::asciiz(p,p.cpu.regs[2]&0xffff)?;
        let(file,_)=match files::program(p,&raw)?{Ok(found)=>found,Err(code)=>{fail(p,u32::from(code));return Ok(())}};
        return overlay(p,&file);
    }
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
    let parent_window=(p.image.fetch_base+p.image.start,p.image.fetch_base+p.image.end);
    p.exec.push(Frame{cpu:p.cpu.clone(),selectors:p.selectors,image:p.image.clone(),open_handles:p.open_handles,dta:p.dta,psp:p.psp});
    let mut system=p.image.system;
    if let Some(slot)=system.iter_mut().find(|(s,e)|*s==0&&*e==0){*slot=parent_window;}
    p.image.system=system;
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
/// Whether termination has a real waiting parent (AH4B or AH55).
pub(super) fn has_parent(p:&Loaded)->bool{
    !p.exec.is_empty() || p.in_memory.last().is_some_and(|f|f.child==p.psp&&f.exec_depth==p.exec.len())
}
/// Dispatch termination by the active PSP and EXEC nesting depth. An AH55
/// child must never pop the older AH4B frame that launched its parent.
pub(super) fn terminate(p:&mut Loaded,code:u8,kind:u8)->Result<(),String>{
    let in_memory=p.in_memory.last().is_some_and(|f|f.child==p.psp&&f.exec_depth==p.exec.len());
    let base=u32::from(p.psp)*16;
    // Read the exit target before changing the MCB, IVT, or execution context.
    let exit=if in_memory{
        let frame=p.in_memory.last().unwrap();
        let parent=word(&p.ram,base+0x16)?;
        if parent!=frame.parent{return Err(format!("AH55 child PSP {:04x} has unexpected parent {:04x}, expected {:04x}",p.psp,parent,frame.parent));}
        let target=p.ram.load(base+0x0a,MemWidth::Dword).map_err(|e|format!("AH55 child INT22 read: {e:?}"))?;
        if target==0{return Err("AH55 child has no INT22 termination continuation".into());}
        Some(target)
    }else{None};
    // Return the INT22/23/24 vectors saved in the terminating PSP.
    for(at,vector)in [(0x0a,0x22u32),(0x0e,0x23),(0x12,0x24)]{
        let saved=p.ram.load(base+at,MemWidth::Dword).map_err(|e|format!("terminate PSP read: {e:?}"))?;
        p.ram.store(vector*4,MemWidth::Dword,saved).map_err(|e|format!("terminate vector write: {e:?}"))?;
    }
    brk::drop_program(p);
    let keep=p.resident.take().map(|_|p.psp);
    p.arena.release(&mut p.ram,p.psp,keep)?;
    p.return_code=u16::from(kind)<<8|u16::from(code);
    if let Some(target)=exit{
        let frame=p.in_memory.pop().unwrap();
        // Preserve the child's live register/stack state; its PSP INT22 target
        // is the explicit continuation established by the parent at run time.
        // The DOS service loop has already retired the terminating INT.
        let cs=(target>>16)as u16;
        let ip=target as u16;
        let index=SegmentRegister::Cs.index();
        p.selectors[index]=cs;p.image.selectors[index]=cs;
        p.image.segments[index]=SegmentCache::real_mode(cs);
        p.cpu.eip=u32::from(ip);
        p.image.system=frame.system;
        p.open_handles=frame.open_handles;p.dta=frame.dta;p.psp=frame.parent;
        return files::drop_unreferenced(p);
    }
    let frame=p.exec.pop().ok_or("terminate without a parent")?;
    p.cpu=frame.cpu;p.cpu.eip=(p.cpu.eip+2)&0xffff;p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;
    let (msw,controls)=(p.image.msw,p.image.controls);
    p.selectors=frame.selectors;p.image=frame.image;p.image.msw=msw;p.image.controls=controls;p.image.retired();
    p.open_handles=frame.open_handles;p.dta=frame.dta;p.psp=frame.psp;
    // Forget AH55 PSPs abandoned when the parent itself terminates.
    p.in_memory.retain(|f|f.exec_depth<p.exec.len()+1);
    files::drop_unreferenced(p)
}
/// The running PSP, AH55 parents, and waiting AH4B EXEC parents retain SFT references.
pub(super) fn chain(p:&Loaded)->Vec<(u16,u32)>{
    std::iter::once((p.psp,p.open_handles))
        .chain(p.in_memory.iter().rev().map(|f|(f.parent,f.open_handles)))
        .chain(p.exec.iter().rev().map(|f|(f.psp,f.open_handles))).collect()
}
/// INT 21h AH=55h: Create Child PSP at DX with memory size SI (DOS 3.0+).
/// The child PSP inherits parent's handles, environment, FCBs, and command tail,
/// and records the parent PSP segment. If the child block is allocated in the
/// MCB arena, ownership is transferred to the child. The active PSP switches to DX.
pub(super) fn create_child_psp(p:&mut Loaded)->Result<(),String>{
    if p.exec.len()+p.in_memory.len()>=MAX_DEPTH{return Err("DOS AH55 child nesting limit reached".into());}
    let child=(p.cpu.regs[2]&0xffff)as u16;
    let memory_end=(p.cpu.regs[6]&0xffff)as u16;
    let base=u32::from(child)*16;
    if base<0x500||base+256>crate::dos::LIMIT as u32{
        return Err(format!("DOS create child PSP at {child:04x}h outside conventional RAM is not modelled"));
    }
    if child==p.psp{
        return Err("DOS create child PSP at current PSP is not modelled".into());
    }
    let Some((header,owner,size))=p.arena.find_segment(child) else{
        return Err(format!("DOS create child PSP at DX={child:04x}h SI={memory_end:04x}h parent={:04x}h outside tracked MCB arena is not modelled",p.psp));
    };
    if child==header{
        return Err(format!("DOS create child PSP at DX={child:04x}h overlaps MCB header at {header:04x}h"));
    }
    if owner==0{
        return Err(format!("DOS create child PSP at DX={child:04x}h in unallocated memory (MCB header={header:04x}h size={size:04x}h) is not modelled"));
    }
    if owner!=p.psp{
        return Err(format!("DOS create child PSP at DX={child:04x}h in memory owned by {owner:04x}h, not caller {:04x}h (MCB header={header:04x}h size={size:04x}h)",p.psp));
    }
    let alloc_end=header+size+1;
    if child+0x10>alloc_end{
        return Err(format!("DOS create child PSP at DX={child:04x}h needs 10h paragraphs but allocated block ends at {alloc_end:04x}h (MCB header={header:04x}h size={size:04x}h owner={owner:04x}h)"));
    }
    if memory_end>alloc_end{
        return Err(format!("DOS create child PSP SI={memory_end:04x}h exceeds allocated block boundary {alloc_end:04x}h (MCB header={header:04x}h size={size:04x}h)"));
    }
    if memory_end<child+0x10{
        return Err(format!("DOS create child PSP SI={memory_end:04x}h is smaller than child PSP boundary {:04x}h",child+0x10));
    }
    let is_exact_base=child==header+1;
    if is_exact_base{
        let mcb_owner_addr=u32::from(header)*16+1;
        p.ram.check_write(mcb_owner_addr,MemWidth::Word).map_err(|e|format!("child MCB owner write check: {e:?}"))?;
    }
    // Temporarily allow instruction fetch from the child region while the
    // caller continues to execute in its own window. Do not replace the current
    // image or capture the CPU here: the child has not started yet.
    let saved_system=p.image.system;
    let mut next_system=saved_system;
    let window=(base,u32::from(memory_end)*16);
    let covered=(p.image.fetch_base+p.image.start<=window.0 && p.image.fetch_base+p.image.end>=window.1)
        ||next_system.iter().any(|&(a,b)|a<=window.0&&b>=window.1&&a<b);
    if !covered{
        let slot=next_system.iter().position(|&(a,b)|a==0&&b==0)
            .ok_or("DOS AH55 child code fetch range capacity exhausted")?;
        next_system[slot]=window;
    }
    eprintln!("[DOS AH=55h] Create child PSP: DX={child:04x}h SI={memory_end:04x}h parent={:04x}h MCB(header={header:04x}h size={size:04x}h owner={owner:04x}h exact_base={is_exact_base})",p.psp);
    let parent=u32::from(p.psp)*16;
    let mut psp=[0u8;256];
    psp[..2].copy_from_slice(&[0xcd,0x20]);
    psp[2..4].copy_from_slice(&memory_end.to_le_bytes());
    for(at,vector)in [(0x0au32,0x22u32),(0x0e,0x23),(0x12,0x24)]{
        let saved=p.ram.load(vector*4,MemWidth::Dword).map_err(|e|format!("child PSP vector read: {e:?}"))?;
        psp[at as usize..at as usize+4].copy_from_slice(&saved.to_le_bytes());
    }
    psp[0x16..0x18].copy_from_slice(&p.psp.to_le_bytes());
    let mut handles=0u32;
    for h in 0..20u16{
        let entry=if p.open_handles&(1<<h)!=0{
            p.ram.load(services::jft(p,h)?,MemWidth::Byte).map_err(|e|format!("child PSP JFT read: {e:?}"))?as u8
        }else{
            0xff
        };
        psp[0x18+usize::from(h)]=entry;
        if entry!=0xff{handles|=1<<h;}
    }
    let env_seg=p.ram.load(parent+0x2c,MemWidth::Word).map_err(|e|format!("child PSP env read: {e:?}"))?;
    psp[0x2c..0x2e].copy_from_slice(&(env_seg as u16).to_le_bytes());
    psp[0x32..0x34].copy_from_slice(&20u16.to_le_bytes());
    psp[0x34..0x36].copy_from_slice(&0x18u16.to_le_bytes());
    psp[0x36..0x38].copy_from_slice(&child.to_le_bytes());
    let version=p.ram.load(parent+0x40,MemWidth::Word).map_err(|e|format!("child PSP version read: {e:?}"))?;
    psp[0x40..0x42].copy_from_slice(&(version as u16).to_le_bytes());
    psp[0x50..0x53].copy_from_slice(&[0xcd,0x21,0xcb]);
    let fcb1=bytes(&p.ram,parent+0x5c,16)?;
    let fcb2=bytes(&p.ram,parent+0x6c,16)?;
    let tail=bytes(&p.ram,parent+0x80,128)?;
    psp[0x5c..0x6c].copy_from_slice(&fcb1);
    psp[0x6c..0x7c].copy_from_slice(&fcb2);
    psp[0x80..0x100].copy_from_slice(&tail);
    p.ram.write_bytes(base,&psp).map_err(|e|format!("child PSP write: {e:?}"))?;
    if is_exact_base{
        p.arena.set_owner(&mut p.ram,child,child)?;
    }
    p.in_memory.push(InMemoryChild{child,parent:p.psp,exec_depth:p.exec.len(),open_handles:p.open_handles,dta:p.dta,system:saved_system});
    p.image.system=next_system;
    p.psp=child;
    p.open_handles=handles;
    p.dta=(child,0x80);
    p.cpu.regs[0]=(p.cpu.regs[0]&!0xff)|0xf0;
    Ok(())
}
