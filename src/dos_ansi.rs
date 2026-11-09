//! An ANSI.SYS-style CON driver, loaded by the session's `ansi on` as
//! DEVICE=ANSI.SYS would load it. Its state lives in guest RAM in Dine's DOS
//! data segment, so it changes together with the screen it draws, all or none.
use super::*;
/// DOSDATA:0390h: installed, parser state (0 text, 1 after ESC, 2 in ESC [,
/// 3 in a quoted string), attribute, saved row and
/// column, parameter count, private-mode flag, then 16 parameter words.
const BASE:u32=(DOSDATA as u32)*16+0x390;
const MAXP:u32=16;
const STATE_SIZE:u32=8+2*MAXP;
pub(in super::super) fn install(p:&mut Loaded)->Result<(),String>{
    let mut bytes=vec![0u8;STATE_SIZE as usize];bytes[0]=1;bytes[2]=7;
    p.ram.write_bytes(BASE,&bytes).map_err(|e|format!("ANSI driver state: {e:?}"))
}
pub(in super::super) fn installed(p:&Loaded)->Result<bool,String>{Ok(byte(p,BASE)?==1)}
/// ANSI colour order (black, red, green, yellow, blue, magenta, cyan,
/// white) in CGA attribute bits.
const COLOUR:[u32;8]=[0,4,2,6,1,5,3,7];
pub(super) fn plan(p:&Loaded,page:u32,bytes:&[u8])->Result<Writes,String>{
    if page>=8{return Err("BIOS video page must be 0..7".into());}
    let mut state:Vec<u32>=(0..8).map(|i|byte(p,BASE+i)).collect::<Result<_,_>>()?;
    // Guest RAM is writable: validate the count before indexing the fixed parameter array.
    if state[5]>MAXP{return Err("invalid ANSI driver parameter count".into());}
    let mut params:Vec<u32>=(0..MAXP).map(|i|read(p,BASE+8+2*i)).collect::<Result<_,_>>()?;
    let(mut row,mut col)=cursor(p,page)?;let mut cells=page_cells(p,page)?;
    let(mut dirty,mut whole)=(Vec::new(),false);
    let blank=|attr:u32|0x20|(attr<<8);
    for &b in bytes{
        let attr=state[2];
        match state[1]{
            1 if b==b'['=>{state[1]=2;state[5]=0;state[6]=0;params.fill(0);continue;},
            // ESC without '[' is no sequence: both bytes are shown.
            1=>{state[1]=0;put(&mut cells,&mut dirty,&mut whole,&mut row,&mut col,27,attr);},
            2=>{
                let n=state[5];
                match b{
                    b'0'..=b'9'=>{if n==0{state[5]=1;}let i=(state[5]-1)as usize;params[i]=(params[i]*10+u32::from(b-b'0')).min(0xffff);},
                    b';'=>{if n==0{state[5]=1;}if state[5]<MAXP{state[5]+=1;}},
                    b'='|b'?' if n==0=>state[6]=1,
                    0x40..=0x7e=>{
                        state[1]=0;
                        let got=&params[..n as usize];
                        let first=got.first().copied().unwrap_or(0);let count=first.max(1);
                        match b{
                            b'H'|b'f'=>{row=(first.max(1)-1).min(ROWS-1);col=(got.get(1).copied().unwrap_or(0).max(1)-1).min(COLS-1);},
                            b'A'=>row=row.saturating_sub(count),
                            b'B'=>row=(row+count).min(ROWS-1),
                            b'C'=>col=(col+count).min(COLS-1),
                            b'D'=>col=col.saturating_sub(count),
                            b's'=>{state[3]=row;state[4]=col;},
                            b'u'=>{row=state[3].min(ROWS-1);col=state[4].min(COLS-1);},
                            b'J'=>{
                                let at=row*COLS+col;
                                let range=match first{0=>at..CELLS,1=>0..at+1,_=>0..CELLS};
                                for cell in range{cells[cell as usize]=blank(attr);}
                                whole=true;if first==2{row=0;col=0;}
                            },
                            b'K'=>for c in col..COLS{let cell=(row*COLS+c)as usize;cells[cell]=blank(attr);dirty.push(cell);},
                            b'm'=>{
                                let mut a=attr;
                                for &v in if got.is_empty(){&[0][..]}else{got}{
                                    a=match v{
                                        0=>7,1=>a|8,5=>a|0x80,
                                        7=>(a&0x88)|((a&7)<<4)|((a>>4)&7),
                                        8=>(a&0xf0)|((a>>4)&7),
                                        30..=37=>(a&!7)|COLOUR[(v-30)as usize],
                                        40..=47=>(a&!0x70)|(COLOUR[(v-40)as usize]<<4),
                                        // Underline needs a monochrome adapter; others are not ANSI.SYS's.
                                        _=>a,
                                    };
                                }
                                state[2]=a;
                            },
                            b'n'|b'h'|b'l'|b'p'=>return Err(format!("ANSI.SYS sequence ESC[...{} (status report, video mode or key reassignment) is not modelled",b as char)),
                            // Other final bytes are not ANSI.SYS commands: the sequence is dropped.
                            _=>{},
                        }
                    },
                    // A quoted string belongs to a key reassignment (…p).
                    b'"'=>state[1]=3,
                    // Other bytes end the sequence unshown.
                    _=>state[1]=0,
                }
                continue;
            },
            3=>{if b==b'"'{state[1]=2;}continue;},
            _ if b==27=>{state[1]=1;continue;},
            _=>{},
        }
        put(&mut cells,&mut dirty,&mut whole,&mut row,&mut col,b,attr);
    }
    let base=VIDEO+page*PAGE;
    let mut writes=if whole{window_writes(page,&cells,(0,0,ROWS-1,COLS-1))}
        else{dirty.into_iter().map(|cell|(base+cell as u32*2,MemWidth::Word,cells[cell])).collect()};
    writes.push((0x450+page*2,MemWidth::Word,(row<<8)|col));
    writes.extend(state.iter().enumerate().skip(1).map(|(i,&v)|(BASE+i as u32,MemWidth::Byte,v)));
    writes.extend(params.iter().enumerate().map(|(i,&v)|(BASE+8+2*i as u32,MemWidth::Word,v)));
    Ok(writes)
}
/// One shown byte: characters take the driver's attribute; BEL, BS, LF,
/// CR and TAB act as in teletype output, and the bottom line scrolls in
/// the driver's attribute.
fn put(cells:&mut [u32],dirty:&mut Vec<usize>,whole:&mut bool,row:&mut u32,col:&mut u32,b:u8,attr:u32){
    let n=if b==9{8-*col%8}else{1};
    for _ in 0..n{
        match b{
            7=>{},
            8=>*col=col.saturating_sub(1),
            10=>*row+=1,
            13=>*col=0,
            _=>{
                let cell=(*row*COLS+*col)as usize;let ch=if b==9{32}else{u32::from(b)};
                cells[cell]=ch|(attr<<8);dirty.push(cell);*col+=1;if *col==COLS{*col=0;*row+=1;}
            },
        }
        if *row==ROWS{scroll(cells,(0,0,ROWS-1,COLS-1),1,attr,true);*row=ROWS-1;*whole=true;}
    }
}
