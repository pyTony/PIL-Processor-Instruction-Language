//! Original bounded raw-DEFLATE reader for ZIP method8 (RFC1951).
struct Bits<'a>{data:&'a[u8],at:usize}
impl Bits<'_>{
    fn peek(&self,n:usize)->u32{let mut v=0;for i in 0..n{let bit=self.at+i;if bit<self.data.len()*8{v|=u32::from((self.data[bit/8]>>(bit%8))&1)<<i;}}v}
    fn read(&mut self,n:usize)->Result<u32,String>{if n>16||self.at+n>self.data.len()*8{return Err("truncated DEFLATE bits".into());}let v=self.peek(n);self.at+=n;Ok(v)}
}
struct Huff{max:usize,table:Vec<(u16,u8)>}
impl Huff{
    fn new(lengths:&[u8])->Result<Self,String>{
        let max=usize::from(*lengths.iter().max().unwrap_or(&0));if max>15{return Err("DEFLATE code length exceeds15".into());}
        if max==0{return Ok(Self{max,table:Vec::new()});}
        let mut counts=[0u32;16];for &n in lengths{if n!=0{counts[n as usize]+=1;}}
        let mut next=[0u32;16];let mut code=0;
        for n in 1..=max{code=(code+counts[n-1])<<1;if code+counts[n]>1u32<<n{return Err("oversubscribed DEFLATE tree".into());}next[n]=code;}
        if code+counts[max]!=(1u32<<max)&&!(max==1&&counts[1]==1){return Err("incomplete DEFLATE tree".into());}
        let mut table=vec![(0,0);1<<max];
        for(symbol,&len)in lengths.iter().enumerate(){if len==0{continue;}
            let n=len as usize;let c=next[n];next[n]+=1;let reverse=c.reverse_bits()>>(32-n);
            for suffix in 0..(1usize<<(max-n)){let at=reverse as usize|(suffix<<n);if table[at].1!=0{return Err("overlapping DEFLATE code".into());}table[at]=(symbol as u16,len);}
        }Ok(Self{max,table})
    }
    fn read(&self,b:&mut Bits)->Result<usize,String>{
        if self.max==0{return Err("empty DEFLATE distance tree used".into());}
        let(symbol,n)=self.table[b.peek(self.max)as usize];if n==0{return Err("invalid DEFLATE code".into());}b.read(n as usize)?;Ok(symbol as usize)
    }
}
const LB:[usize;29]=[3,4,5,6,7,8,9,10,11,13,15,17,19,23,27,31,35,43,51,59,67,83,99,115,131,163,195,227,258];
const LE:[usize;29]=[0,0,0,0,0,0,0,0,1,1,1,1,2,2,2,2,3,3,3,3,4,4,4,4,5,5,5,5,0];
const DB:[usize;30]=[1,2,3,4,5,7,9,13,17,25,33,49,65,97,129,193,257,385,513,769,1025,1537,2049,3073,4097,6145,8193,12289,16385,24577];
const DE:[usize;30]=[0,0,0,0,1,1,2,2,3,3,4,4,5,5,6,6,7,7,8,8,9,9,10,10,11,11,12,12,13,13];
fn trees(b:&mut Bits,kind:u32)->Result<(Huff,Huff),String>{
    if kind==1{let mut ll=vec![8;288];ll[144..256].fill(9);ll[256..280].fill(7);return Ok((Huff::new(&ll)?,Huff::new(&[5;32])?));}
    let nl=b.read(5)?as usize+257;let nd=b.read(5)?as usize+1;let nc=b.read(4)?as usize+4;
    if nl>286{return Err("reserved DEFLATE literal count".into());}
    let order=[16,17,18,0,8,7,9,6,10,5,11,4,12,3,13,2,14,1,15];let mut cl=[0u8;19];
    for &n in &order[..nc]{cl[n]=b.read(3)?as u8;}let alphabet=Huff::new(&cl)?;let mut lengths=Vec::new();
    while lengths.len()<nl+nd{
        match alphabet.read(b)?{
            n@0..=15=>lengths.push(n as u8),
            n@16..=18=>{let(count,v)=match n{16=>(b.read(2)?as usize+3,*lengths.last().ok_or("DEFLATE repeat has no predecessor")?),17=>(b.read(3)?as usize+3,0),_=>(b.read(7)?as usize+11,0)};
                if lengths.len()+count>nl+nd{return Err("DEFLATE code repeat exceeds alphabet".into());}lengths.extend(std::iter::repeat(v).take(count));},
            _=>return Err("invalid DEFLATE length symbol".into()),
        }
    }
    if lengths[256]==0{return Err("DEFLATE tree has no end-of-block".into());}
    Ok((Huff::new(&lengths[..nl])?,Huff::new(&lengths[nl..])?))
}
pub fn decode(data:&[u8],expected:usize)->Result<Vec<u8>,String>{
    if expected>super::containers::MAX_ENTRY{return Err("DEFLATE output exceeds entry limit".into());}
    let mut b=Bits{data,at:0};let mut out=Vec::with_capacity(expected);
    loop{
        let last=b.read(1)?;let kind=b.read(2)?;
        match kind{
            0=>{b.at=(b.at+7)&!7;let n=b.read(16)?as usize;let inverse=b.read(16)?as usize;if n^inverse!=0xffff{return Err("DEFLATE stored length mismatch".into());}
                if out.len()+n>expected{return Err("DEFLATE exceeds declared output size".into());}for _ in 0..n{out.push(b.read(8)?as u8);}},
            1|2=>{let(ll,dd)=trees(&mut b,kind)?;
                loop{match ll.read(&mut b)?{
                    n@0..=255=>{if out.len()==expected{return Err("DEFLATE exceeds declared output size".into());}out.push(n as u8);},
                    256=>break,
                    n@257..=285=>{let n=n-257;let count=LB[n]+b.read(LE[n])?as usize;let d=dd.read(&mut b)?;
                        if d>=30{return Err("reserved DEFLATE distance".into());}let distance=DB[d]+b.read(DE[d])?as usize;
                        if distance>out.len()||out.len()+count>expected{return Err("invalid DEFLATE back-reference/output size".into());}
                        for _ in 0..count{out.push(out[out.len()-distance]);}},
                    _=>return Err("reserved DEFLATE length".into()),
                }}
            },_=>return Err("reserved DEFLATE block type".into()),
        }
        if last!=0{break;}
    }
    if out.len()!=expected||(b.at+7)/8!=data.len(){return Err("DEFLATE size/trailing-byte mismatch".into());}Ok(out)
}
