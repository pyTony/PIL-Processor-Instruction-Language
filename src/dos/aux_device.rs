//! DOS's AUX device on COM1 (handle 3, or COM1/AUX opened by name), over
//! the port in crate::pc::serial.
use super::*;
/// DOS write to the AUX device: all bytes go out.
pub(crate) fn write(p:&mut Loaded,bytes:&[u8])->Result<u32,String>{
    let s=serial::port(p)?;s.tx.extend_from_slice(bytes);
    for &b in bytes{
        s.column=match b{13=>0,8=>s.column.saturating_sub(1),10|7=>s.column,9=>(s.column+8)&!7,_=>s.column+1};
    }
    if matches!(files::system_file(p,1),Ok(Some(3))){
        let page=p.ram.load(0x462,MemWidth::Byte).map_err(|e|format!("BIOS active page read: {e:?}"))?&7;
        let cols=(p.ram.load(0x44a,MemWidth::Word).map_err(|e|format!("BIOS columns read: {e:?}"))?as u8).max(1);
        let max_row=(p.ram.load(0x484,MemWidth::Byte).map_err(|e|format!("BIOS rows read: {e:?}"))?as u8).max(24);
        let pos=p.ram.load(0x450+page*2,MemWidth::Word).map_err(|e|format!("BIOS cursor read: {e:?}"))?as u16;
        let(mut col,mut row)=((pos&0xff)as u8,(pos>>8)as u8);
        for &b in bytes{
            match b{
                13=>col=0,
                10=>if row<max_row{row+=1;},
                8=>col=col.saturating_sub(1),
                7=>{},
                9=>col=((col+8)&!7).min(cols-1),
                _=>{col+=1;if col>=cols{col=0;if row<max_row{row+=1;}}},
            }
        }
        p.ram.store(0x450+page*2,MemWidth::Word,(u32::from(row)<<8)|u32::from(col)).map_err(|e|format!("BIOS cursor write: {e:?}"))?;
    }
    Ok(bytes.len()as u32)
}
/// DOS read from the AUX device: the bytes received so far, up to `wanted`;
/// with none yet the read waits for the frontend, as a device read waits.
pub(crate) fn read(p:&mut Loaded,wanted:usize)->Result<Option<Vec<u8>>,String>{
    let s=serial::port(p)?;
    if s.rx.is_empty(){return Ok(None);}
    let n=wanted.min(s.rx.len());Ok(Some(s.rx.drain(..n).collect()))
}
/// Whether a DOS name is the COM1 device (COM1 or AUX, with any directory
/// or extension), when the session has the port.
pub(crate) fn is_device(p:&Loaded,raw:&[u8])->bool{
    p.com1.is_some()&&(device_name(raw,b"COM1")||device_name(raw,b"AUX"))
}
/// Whether a DOS name is the device `name`, with any directory or extension.
pub(crate) fn device_name(raw:&[u8],name:&[u8])->bool{
    let base=raw.rsplit(|&c|c==b'\\'||c==b'/'||c==b':').next().unwrap_or(raw);
    base.split(|&c|c==b'.').next().unwrap_or(base).eq_ignore_ascii_case(name)
}
