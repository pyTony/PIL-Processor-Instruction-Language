//! ROM fonts and the read-only C000:0000..F000:FFFF BIOS area.
use super::*;

const FONT_TEXT:&str=include_str!("font8x16.txt");
pub const CELL_HEIGHT:usize=16;

/// Far pointers to the 8x14 and 8x16 fonts in the video BIOS area. As on an
/// EGA/VGA BIOS, INT 43h points at the one that matches a 350- or 480-line
/// graphics mode, and AX=1130h BH=02h/06h returns them.
pub const ROM_FONT14:u32=0xc000_2000;
pub const ROM_FONT16:u32=0xc000_3000;

/// Linear address of a real-mode far pointer.
pub fn linear(pointer:u32)->u32{(pointer>>16)*16+(pointer&0xffff)}

/// The 8x14 font, 14 bytes per character: the 8x16 font without its top and
/// bottom rows, which no glyph that fits a cell uses (line and block
/// characters run on through the cell either way).
pub fn font14()->Result<Vec<u8>,String>{
    Ok(font()?.chunks(CELL_HEIGHT).flat_map(|g|g[1..15].to_vec()).collect())
}

/// The 8x16 font, 16 bytes per character (bit 7 is the left column).
pub fn font()->Result<Vec<u8>,String>{
    let mut lines=FONT_TEXT.lines().filter(|l|!l.starts_with(';'));
    let mut bytes=Vec::with_capacity(256*CELL_HEIGHT);
    for code in 0..256u32{
        let header=lines.next().ok_or("8x16 font file ends early")?;
        if u32::from_str_radix(header.get(..2).unwrap_or(""),16)!=Ok(code){return Err(format!("8x16 font glyph {code:02X}h header is {header:?}"));}
        for _ in 0..CELL_HEIGHT{
            let row=lines.next().ok_or("8x16 font file ends inside a glyph")?;
            if row.len()!=8||!row.bytes().all(|b|b==b'#'||b==b'.'){return Err(format!("8x16 font glyph {code:02X}h row {row:?}"));}
            bytes.push(row.bytes().fold(0u8,|n,b|n<<1|u8::from(b==b'#')));
        }
    }
    if lines.next().is_some(){return Err("8x16 font file has data after glyph FFh".into());}
    Ok(bytes)
}

/// C000:0000..F000:FFFF reads as an empty bus (FFh, so no option-ROM 55AAh
/// signature) and is read-only. The top 16 bytes carry what a PC/AT-class BIOS
/// identifies itself with: the reset jump, a BIOS date at F000:FFF5 and model
/// byte FCh at F000:FFFE.
pub fn rom(ram:&mut Ram)->Result<(),String>{
    let mut top=[0xffu8;16];
    top[..5].copy_from_slice(&[0xea,0x5b,0xe0,0x00,0xf0]);
    top[5..13].copy_from_slice(b"01/10/84");top[13]=0;top[14]=0xfc;top[15]=0;
    ram.write_bytes(0xc0000,&vec![0xff;0x3fff0]).map_err(|e|format!("{e:?}"))?;
    ram.write_bytes(0xffff0,&top).map_err(|e|format!("{e:?}"))?;
    let font8=console::graphics_text::font()?;let upper=console::graphics_text::UPPER_FONT;
    ram.write_bytes(console::graphics_text::FONT,&font8[..1024]).map_err(|e|format!("ROM font: {e:?}"))?;
    ram.write_bytes((upper>>16)*16+(upper&0xffff),&font8[1024..]).map_err(|e|format!("ROM upper font: {e:?}"))?;
    ram.write_bytes(linear(ROM_FONT14),&font14()?).map_err(|e|format!("ROM 8x14 font: {e:?}"))?;
    ram.write_bytes(linear(ROM_FONT16),&font()?).map_err(|e|format!("ROM 8x16 font: {e:?}"))?;
    vectors::install(ram)?;
    // INT 43h starts at the 8x8 graphics font, as after an EGA/VGA BIOS POST.
    ram.store(0x43*4,MemWidth::Dword,0xf000_0000|(console::graphics_text::FONT&0xffff)).map_err(|e|format!("INT 43h vector: {e:?}"))?;
    ram.store(0x1f*4,MemWidth::Dword,upper).map_err(|e|format!("INT 1Fh vector: {e:?}"))?;
    // A write to the BIOS or video ROM is ignored, as on an AT: no fault.
    ram.rom(0xc0000,0x40000).map_err(|e|format!("{e:?}"))
}
