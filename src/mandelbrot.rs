//! Original Q12 guest image and host reference/image output; no VGA emulation.
use std::{collections::BTreeMap,fs,path::PathBuf};
pub fn validate(size:u32,iterations:u32)->Result<(),String>{
    if !(2..=256).contains(&size)||!size.is_power_of_two(){return Err("Mandelbrot size must be a power of two in 2..256".into());}
    if !(1..=64).contains(&iterations){return Err("Mandelbrot iterations must be 1..64".into());}Ok(())
}
struct Assembler{bytes:Vec<u8>,labels:BTreeMap<&'static str,usize>,fixups:Vec<(usize,&'static str)>}
impl Assembler{
    fn emit(&mut self,data:&[u8]){self.bytes.extend_from_slice(data);}
    fn label(&mut self,name:&'static str){self.labels.insert(name,self.bytes.len());}
    fn imm(&mut self,opcode:&[u8],value:i32){self.emit(opcode);self.emit(&value.to_le_bytes());}
    fn jump(&mut self,cc:Option<u8>,name:&'static str){if let Some(cc)=cc{self.emit(&[0x0f,0x80+cc]);}else{self.emit(&[0xe9]);}self.fixups.push((self.bytes.len(),name));self.emit(&[0,0]);}
    fn finish(mut self)->Result<Vec<u8>,String>{for(at,name)in self.fixups{let target=*self.labels.get(name).ok_or("unknown assembly label")?;let delta=i16::try_from(target as i64-(at+2) as i64).map_err(|_|"branch out of range")?;self.bytes[at..at+2].copy_from_slice(&delta.to_le_bytes());}Ok(self.bytes)}
}
pub fn guest_bytes(size:u32,iterations:u32)->Result<Vec<u8>,String>{
    validate(size,iterations)?;let step=12288/size as i32;
    let mut a=Assembler{bytes:Vec::new(),labels:BTreeMap::new(),fixups:Vec::new()};
    a.imm(&[0x66,0xbf],0); // EDI=row
    a.imm(&[0x66,0xbb],-6144);a.emit(&[0x66,0x89,0x1e,0x04,0x05]); // DS:504 cy
    a.label("row");a.imm(&[0x66,0xbe],0); // ESI=column
    a.imm(&[0x66,0xbb],-8192);a.emit(&[0x66,0x89,0x1e,0,0x05]); // DS:500 cx
    a.label("pixel");a.imm(&[0x66,0xb8],0);a.imm(&[0x66,0xba],0);a.imm(&[0x66,0xbd],0);
    a.label("iterate");
    a.emit(&[0x66,0x89,0xc3,0x66,0x0f,0xaf,0xd8]); // EBX=x*x
    a.emit(&[0x66,0x89,0xd1,0x66,0x0f,0xaf,0xca]); // ECX=y*y
    a.emit(&[0x66,0x01,0xcb]);a.imm(&[0x66,0x81,0xfb],4<<24);a.jump(Some(7),"escape");
    a.emit(&[0x66,0x29,0xcb,0x66,0x29,0xcb,0x66,0xc1,0xfb,12]); // (xx-yy)>>12
    a.emit(&[0x66,0x0f,0xaf,0xc2,0x66,0xd1,0xe0,0x66,0xc1,0xf8,12]); // 2xy>>12
    a.emit(&[0x66,0x03,0x06,4,5,0x66,0x89,0xc2,0x66,0x89,0xd8,0x66,0x03,0x06,0,5]);
    a.emit(&[0x66,0x45]);a.imm(&[0x66,0x81,0xfd],iterations as i32);a.jump(Some(2),"iterate");
    a.imm(&[0x66,0xb8],0);a.jump(None,"emit");
    a.label("escape");a.emit(&[0x66,0x89,0xe8,0x66,0xc1,0xe0,2]);
    a.label("emit");a.emit(&[0xe6,0xe9]);
    a.imm(&[0x66,0x81,0x06,0,5],step);a.emit(&[0x66,0x46]);a.imm(&[0x66,0x81,0xfe],size as i32);a.jump(Some(2),"pixel");
    a.imm(&[0x66,0x81,0x06,4,5],step);a.emit(&[0x66,0x47]);a.imm(&[0x66,0x81,0xff],size as i32);a.jump(Some(2),"row");
    a.emit(&[0xf4]);a.finish()
}
// Mathematical reference: no decoder, CPU tokens or assembly execution.
pub fn reference(size:u32,iterations:u32)->Result<Vec<u8>,String>{
    validate(size,iterations)?;let mut pixels=Vec::new();let step=12288/i64::from(size);
    for row in 0..size{for col in 0..size{
        let(cx,cy)=(-8192+i64::from(col)*step,-6144+i64::from(row)*step);
        let(mut x,mut y,mut color)=(0i64,0i64,0u8);
        for n in 0..iterations{
            let(xx,yy)=(x*x,y*y);if xx+yy>4<<24{color=(n*4)as u8;break;}
            let next_x=((xx-yy)>>12)+cx;let next_y=((2*x*y)>>12)+cy;x=next_x;y=next_y;
        }pixels.push(color);
    }}Ok(pixels)
}
pub struct Render{pub pixels:Vec<u8>,pub steps:u64,pub seconds:f64,pub blocked:u32,pub cache:String}
pub fn bitmap(size:u32,pixels:&[u8])->Result<Vec<u8>,String>{
    if !(1..=256).contains(&size)||pixels.len()!=size as usize*size as usize{return Err("bitmap dimensions mismatch".into());}
    let stride=(size as usize*3+3)&!3;let length=54+stride*size as usize;let mut out=vec![0u8;length];out[..2].copy_from_slice(b"BM");
    for (at,value) in [(2,length as u32),(10,54),(14,40),(18,size),(22,size),(34,(stride*size as usize)as u32)]{out[at..at+4].copy_from_slice(&value.to_le_bytes());}
    out[26..28].copy_from_slice(&1u16.to_le_bytes());out[28..30].copy_from_slice(&24u16.to_le_bytes());
    for row in 0..size as usize{for col in 0..size as usize{let value=pixels[row*size as usize+col];let at=54+(size as usize-1-row)*stride+col*3;out[at..at+3].fill(value);}}Ok(out)
}
pub fn render_files(size:u32,iterations:u32)->Result<String,String>{
    validate(size,iterations)?;let result=super::debugger::render_mandelbrot(size,iterations)?;
    let root=PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").unwrap_or_else(||"target".into()));
    let directory=std::env::current_dir().map_err(|e|e.to_string())?.join(root).join("mandelbrot").join(format!("{size}x{size}-i{iterations}"));
    fs::create_dir_all(&directory).map_err(|e|e.to_string())?;
    let mut pgm=format!("P5\n# fixed-point escape iterations, not calibrated luminance\n{size} {size}\n255\n").into_bytes();pgm.extend_from_slice(&result.pixels);
    let display:Vec<u8>=result.pixels.iter().map(|&value|if value==0{0}else{255-value}).collect();
    for (name,bytes) in [("mandelbrot.pgm",pgm),("mandelbrot.bmp",bitmap(size,&display)?),("guest.bin",guest_bytes(size,iterations)?)]{fs::write(directory.join(name),bytes).map_err(|e|e.to_string())?;}
    let report=format!("PASS: {size}x{size} Mandelbrot, {iterations} iterations max; {} serial pixels match independent Q12 reference.\n{} completed guest instructions; elapsed={:.3} ms; {} TX backpressure retries.\n{}\nEngine=PIL-token-interpreter; host time includes serial draining, excludes setup/reference checking/file output; not 386 cycles.\nImage: {}\nRaw image: {}\nGuest binary: {}",result.pixels.len(),result.steps,result.seconds*1000.0,result.blocked,result.cache,directory.join("mandelbrot.bmp").display(),directory.join("mandelbrot.pgm").display(),directory.join("guest.bin").display());
    fs::write(directory.join("report.txt"),&report).map_err(|e|e.to_string())?;Ok(report)
}
