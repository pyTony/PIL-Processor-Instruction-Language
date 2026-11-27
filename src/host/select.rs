//! Copying text off the Dine console window, as a terminal does: a left
//! drag selects character cells from one point to another in reading order,
//! the window shows them inverted, and the text goes to the host clipboard.
//! The selection is the host's alone; the program sees no mouse event for
//! it. It works in the graphics modes too, where the characters are read
//! back by matching each cell against the BIOS font (dos_graphics_text).
//! Nothing here reads guest memory except through `grid` and `screen_text`.
#![cfg_attr(not(any(windows,test)),allow(dead_code))]
use super::*;

/// A selection between two character cells (row, column), both included.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) struct Selection{pub(crate) anchor:(u32,u32),pub(crate) head:(u32,u32)}
impl Selection{
    /// First and last cell in reading order.
    fn span(&self)->((u32,u32),(u32,u32)){if self.anchor<=self.head{(self.anchor,self.head)}else{(self.head,self.anchor)}}
    pub(crate) fn contains(&self,row:u32,column:u32)->bool{
        let(first,last)=self.span();
        (row,column)>=first&&(row,column)<=last
    }
}

/// The cell under a pointer at (x, y) in a picture `width` x `height`
/// showing `columns` x `rows` cells, clamped to the screen.
pub(crate) fn cell_at(x:i32,y:i32,width:i32,height:i32,columns:u32,rows:u32)->(u32,u32){
    let along=|v:i32,size:i32,cells:u32|if size<=0||cells==0{0}else{
        ((i64::from(v.clamp(0,size-1))*i64::from(cells)/i64::from(size)) as u32).min(cells-1)};
    (along(y,height,rows),along(x,width,columns))
}

/// The text grid shown now: (columns, rows, one CP437 byte a cell).
pub(crate) fn screen_text(session:&Session)->Result<(u32,u32,Vec<u8>),String>{
    let p=session.machine().ok_or("session state lost")?;
    if graphics_mode(p){return super::super::console::graphics_text::screen_text(p);}
    let(cells,columns,_)=session.screen()?;
    let text:Vec<u8>=cells.chunks(2).map(|c|c[0]).collect();
    Ok((columns,(text.len()/columns.max(1) as usize) as u32,text))
}

/// The cells of `text` (`columns` wide) inside `selection`, or the whole
/// screen without one, as host text: each line without trailing blanks,
/// lines joined with CR LF, no trailing empty lines. The selection is
/// clamped to the screen.
pub(crate) fn copy_text(text:&[u8],columns:u32,selection:Option<Selection>)->String{
    let columns=columns.max(1);let rows=(text.len()/columns as usize) as u32;
    if rows==0{return String::new();}
    let clamp=|(r,c):(u32,u32)|(r.min(rows-1),c.min(columns-1));
    let(first,last)=selection.map_or(((0,0),(rows-1,columns-1)),|s|{let(a,b)=s.span();(clamp(a),clamp(b))});
    let mut lines=Vec::new();
    for row in first.0..=last.0{
        let from=if row==first.0{first.1}else{0};
        let to=if row==last.0{last.1}else{columns-1};
        let line:String=(from..=to).map(|c|glyph(text[(row*columns+c) as usize])).collect();
        lines.push(line.trim_end().to_string());
    }
    while lines.last().is_some_and(|l|l.is_empty()){lines.pop();}
    lines.join("\r\n")
}

/// The keys that type `text` into the program: line ends as Enter, tabs as
/// Tab; characters with no CP437 key are left out.
pub(crate) fn paste_keys(text:&str)->Vec<u16>{
    let text=text.replace("\r\n","\n").replace('\r',"\n");
    text.chars().filter_map(|c|match c{
        '\n'=>bios_word(Key::Enter,Mods::default()),
        '\t'=>bios_word(Key::Tab,Mods::default()),
        c if c<' '=>None,
        c=>bios_word(Key::Char(c),Mods::default()),
    }).collect()
}

/// Shows `selection` on a picture of `columns` x `rows` cells by inverting
/// the colours of the selected cells.
pub(crate) fn highlight(frame:&mut (usize,usize,Vec<u32>),columns:u32,rows:u32,selection:Selection){
    let(width,height)=(frame.0,frame.1);
    if columns==0||rows==0||width==0||height==0{return;}
    for y in 0..height{
        let row=(y*rows as usize/height) as u32;
        for x in 0..width{
            let column=(x*columns as usize/width) as u32;
            if selection.contains(row,column){frame.2[y*width+x]^=0x00ff_ffff;}
        }
    }
}

/// The selection gesture: what a pointer event did to it.
#[derive(Default)]
pub(crate) struct Selecting{
    pub(crate) selection:Option<Selection>,
    dragging:bool,
    buttons:u16,
}
/// What a pointer event asks of the front end besides the selection.
#[derive(Debug,PartialEq,Eq)]
pub(crate) enum Gesture{None,Copy,Paste}
impl Selecting{
    /// A pointer event on a `columns` x `rows` grid. A left press starts a
    /// selection, moving with the button held extends it, and a press that
    /// is released on the same cell clears it. A right press copies the
    /// selection when there is one and pastes otherwise.
    pub(crate) fn pointer(&mut self,cell:(u32,u32),buttons:u16)->Gesture{
        let pressed=buttons&!self.buttons;self.buttons=buttons;
        if pressed&2!=0{return if self.selection.is_some(){Gesture::Copy}else{Gesture::Paste};}
        if pressed&1!=0{self.selection=Some(Selection{anchor:cell,head:cell});self.dragging=true;return Gesture::None;}
        if self.dragging{
            if let Some(s)=self.selection.as_mut(){s.head=cell;}
            if buttons&1==0{
                self.dragging=false;
                if self.selection.is_some_and(|s|s.anchor==s.head){self.selection=None;}
            }
        }
        Gesture::None
    }
    /// Buttons already pressed for something else (the host menu): a later
    /// event with them still down is not a new press.
    pub(crate) fn hold(&mut self,buttons:u16){self.buttons|=buttons;}
    /// Whether a drag is under way, so its events stay with the host.
    pub(crate) fn dragging(&self)->bool{self.dragging}
    pub(crate) fn clear(&mut self){self.selection=None;self.dragging=false;}
}
