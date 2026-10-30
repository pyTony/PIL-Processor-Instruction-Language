//! Country information for country 001 (United States), code page 437, the
//! only country Dine has: AH38, the AH65 information and upper-case calls, and
//! the far case-map routine their records point to.
use super::*;
/// Offsets in DOSDATA of the case-map routine, its 128-byte table (with the
/// size word before it, as AH65 points at it) and an empty DBCS table.
const CASE_MAP:u16=0x0080;
const UPCASE:u16=0x0090;
const DBCS:u16=0x0110;
const COLLATING:u16=0x0120;
const TERMINATORS:u16=0x0222;
/// File-name terminator table, laid out as FreeDOS has it: size, a flag byte,
/// characters 00h..FFh permitted, 00h..20h excluded, then the 14 characters
/// that end a name.
const TERMINATOR_TABLE:[u8;24]=[0x16,0,1,0x00,0xff,0,0x00,0x20,2,14,b'.',b'"',b'/',b'\\',b'[',b']',b':',b'|',b'<',b'>',b'+',b'=',b';',b','];
/// Upper case of 80h..A5h in code page 437 for country 001; A6h..FFh map to
/// themselves. Accented small letters without a capital in the code page map
/// to the plain capital, as in the MS-DOS and FreeDOS tables.
const UPPER:[u8;38]=[0x80,0x9a,0x45,0x41,0x8e,0x41,0x8f,0x80,0x45,0x45,0x45,0x49,0x49,0x49,0x8e,0x8f,0x90,0x92,0x92,0x4f,0x99,0x4f,0x55,0x55,0x59,0x99,0x9a,0x9b,0x9c,0x9d,0x9e,0x9f,0x41,0x49,0x4f,0x55,0xa5,0xa5];
/// Sort weights of 80h..AFh: letters sort as their plain capital, currency
/// signs as "$", inverted marks and guillemets as "?", "!" and a quote.
const COLLATE:&[u8;48]=b"CUEAAAACEEEIIIAAEAAOOOUUYOU$$$$$AIOUNN\xa6\xa7?\xa9\xaa\xab\xac!\"\"";
fn collate(c:u8)->u8{match c{b'a'..=b'z'=>c-32,0x80..=0xaf=>COLLATE[usize::from(c-0x80)],0xe1=>b'S',c=>c}}
pub(super) fn upper(c:u8)->u8{match c{b'a'..=b'z'=>c-32,0x80..=0xa5=>UPPER[usize::from(c-0x80)],c=>c}}
/// Linear extent of the case-map routine, the only Dine code a program runs.
pub(super) const CODE:(u32,u32)=(DOSDATA as u32*16+CASE_MAP as u32,DOSDATA as u32*16+CASE_MAP as u32+12);
/// The case-map routine (AL >= 80h is looked up in the table) and the table.
pub(super) fn install(ram:&mut Ram)->Result<(),String>{
    let table=UPCASE-0x80;let[lo,hi]=table.to_le_bytes();
    // CMP AL,80h; JB done; PUSH BX; MOV BX,table-80h; CS: XLAT; POP BX; done: RETF
    let code=[0x3c,0x80,0x72,0x07,0x53,0xbb,lo,hi,0x2e,0xd7,0x5b,0xcb];
    let base=u32::from(DOSDATA)*16;
    ram.write_bytes(base+u32::from(CASE_MAP),&code).map_err(|e|format!("DOS case map: {e:?}"))?;
    let mut map=vec![0x80,0];map.extend((0x80..=0xffu8).map(upper));map.extend([0;4]);
    ram.write_bytes(base+u32::from(UPCASE)-2,&map).map_err(|e|format!("DOS case map: {e:?}"))?;
    let mut order=vec![0,1];order.extend((0..=0xffu8).map(collate));
    ram.write_bytes(base+u32::from(COLLATING)-2,&order).map_err(|e|format!("DOS collating table: {e:?}"))?;
    ram.write_bytes(base+u32::from(TERMINATORS),&TERMINATOR_TABLE).map_err(|e|format!("DOS case map: {e:?}"))
}
/// The AH38 record: date format USA, "$", ",", ".", "-", ":", symbol before the
/// amount, 2 decimals, 12-hour clock, the case-map routine and list separator ",".
fn country_info()->[u8;24]{
    let mut info=[0u8;24];
    info[2]=b'$';info[7]=b',';info[9]=b'.';info[11]=b'-';info[13]=b':';info[16]=2;
    info[18..20].copy_from_slice(&CASE_MAP.to_le_bytes());info[20..22].copy_from_slice(&DOSDATA.to_le_bytes());info[22]=b',';
    info
}
fn ok(p:&mut Loaded){p.cpu.eflags&=!1;p.cpu.undefined_flags&=!1;}
fn fail(p:&mut Loaded,code:u32){p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|code;p.cpu.eflags|=1;p.cpu.undefined_flags&=!1;}
fn guest(p:&Loaded,segment:SegmentRegister,offset:u32,write:bool)->Result<u32,String>{
    if offset>0xffff{return Err("DOS country buffer exceeds segment end".into());}
    MemoryContext{segment,cache:p.image.segments[segment.index()]}.translate(offset,MemWidth::Byte,write,p.cpu.eip).map_err(|e|format!("DOS country buffer access: {e:?}"))
}
fn store(p:&mut Loaded,segment:SegmentRegister,offset:u32,bytes:&[u8])->Result<(),String>{
    let addrs=(0..bytes.len()as u32).map(|n|guest(p,segment,offset+n,true)).collect::<Result<Vec<_>,_>>()?;
    for &addr in &addrs{p.ram.check_write(addr,MemWidth::Byte).map_err(|e|format!("DOS country buffer write: {e:?}"))?;}
    for(addr,&b)in addrs.into_iter().zip(bytes){p.ram.store(addr,MemWidth::Byte,u32::from(b)).map_err(|e|format!("DOS country buffer write: {e:?}"))?;}
    Ok(())
}
fn load(p:&Loaded,offset:u32)->Result<u8,String>{
    let addr=guest(p,SegmentRegister::Ds,offset,false)?;
    p.ram.load(addr,MemWidth::Byte).map(|b|b as u8).map_err(|e|format!("DOS country buffer read: {e:?}"))
}
pub(super) fn service(p:&mut Loaded,ah:u8)->Result<(),String>{
    let(al,bx,cx,dx)=(p.cpu.regs[0]as u8,p.cpu.regs[3]&0xffff,p.cpu.regs[1]&0xffff,p.cpu.regs[2]&0xffff);
    if ah==0x38{
        let country=if al==0xff{bx}else{u32::from(al)};
        if !matches!(country,0|1){return Err(format!("DOS country {country} is not modelled (only 001)"));}
        if dx!=0xffff{store(p,SegmentRegister::Ds,dx,&country_info())?;}
        p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|1;p.cpu.regs[3]=(p.cpu.regs[3]&0xffff0000)|1;ok(p);
        return Ok(());
    }
    if matches!(al,1..=7)&&(!matches!(bx,437|0xffff)||!matches!(dx,1|0xffff)){return Err(format!("DOS country {dx} code page {bx} is not modelled (only 001/437)"));}
    match al{
        0x01=>{
            // Info ID 1, size 1Ch, country, code page, then the AH38 record.
            let mut record=vec![1,0x1c,0,1,0,0xb5,0x01];record.extend_from_slice(&country_info());
            if cx<record.len()as u32{fail(p,1);return Ok(());}
            store(p,SegmentRegister::Es,p.cpu.regs[7]&0xffff,&record)?;p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|record.len()as u32;ok(p);
        },
        0x02|0x04..=0x07=>{
            // A pointer to the upper-case table (also used for file names), the
            // file-name terminators, the collating table, or the DBCS
            // lead-byte table, which is empty.
            let table=match al{5=>TERMINATORS,6=>COLLATING-2,7=>DBCS,_=>UPCASE-2};
            let mut record=vec![al];record.extend(table.to_le_bytes());record.extend(DOSDATA.to_le_bytes());
            if cx<5{fail(p,1);return Ok(());}
            store(p,SegmentRegister::Es,p.cpu.regs[7]&0xffff,&record)?;p.cpu.regs[1]=(p.cpu.regs[1]&0xffff0000)|5;ok(p);
        },
        // Not an information ID: invalid function, as in FreeDOS.
        0x00|0x03=>fail(p,1),
        0x20|0xa0=>{p.cpu.regs[2]=(p.cpu.regs[2]&!255)|u32::from(upper(dx as u8));ok(p);},
        0x21|0xa1|0x22|0xa2=>{
            let mut bytes=Vec::new();
            for n in 0..if al&1!=0{cx}else{0x10000}{
                let b=load(p,dx+n)?;if al&2!=0&&b==0{break;}bytes.push(upper(b));
            }
            store(p,SegmentRegister::Ds,dx,&bytes)?;ok(p);
        },
        0x23=>{
            let answer=match dx as u8{b'Y'|b'y'=>1,b'N'|b'n'=>0,_=>2};
            p.cpu.regs[0]=(p.cpu.regs[0]&0xffff0000)|answer;ok(p);
        },
        _=>return Err(format!("DOS country information AX=65{al:02x} is not modelled")),
    }
    Ok(())
}
