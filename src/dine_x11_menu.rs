//! Pure-Rust host popup menu layout, interaction and 8x16 bitmap renderer
//! for the X11 window (`dine_x11`). Free of X11 dependencies so layout,
//! keyboard/mouse navigation and glyph drawing run in unit tests on any host.
use crate::bios::rom_font::CELL_HEIGHT;
use crate::dine_window::MenuGroups;

const CHAR_W:i32=8;
const CHAR_H:i32=CELL_HEIGHT as i32;
const ROW_H:i32=22;
const PAD_X:i32=10;
const BORDER:i32=1;

const BG:u32=0x001e_222a;
const BORDER_COLOR:u32=0x005c_6370;
const TEXT:u32=0x00e6_edf3;
const HILITE_BG:u32=0x001f_6feb;
const HILITE_TEXT:u32=0x00ff_ffff;

/// Screen rectangle in client pixels.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) struct Rect{
    pub(crate) x:i32,
    pub(crate) y:i32,
    pub(crate) w:i32,
    pub(crate) h:i32,
}

impl Rect{
    pub(crate) fn contains(self,x:i32,y:i32)->bool{
        x>=self.x&&x<self.x.saturating_add(self.w)&&y>=self.y&&y<self.y.saturating_add(self.h)
    }
}

/// Outcome of delivering a host event to an open popup menu.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum Action{
    Stay,
    Redraw,
    Choose(u16),
    Cancel,
}

/// Two-level host menu: top-level groups plus the active group's submenu.
#[derive(Clone,Debug)]
pub(crate) struct PopupMenu{
    groups:MenuGroups,
    anchor:(i32,i32),
    client:(i32,i32),
    group:usize,
    item:Option<usize>,
}

fn clamp_origin(origin:i32,size:i32,limit:i32)->i32{
    origin.min((limit-size).max(0)).max(0)
}

fn panel_width(chars:usize)->i32{
    (chars.max(4) as i32)*CHAR_W+PAD_X*2+BORDER*2
}

impl PopupMenu{
    pub(crate) fn open(groups:MenuGroups,anchor:(i32,i32),client:(i32,i32))->Option<Self>{
        if groups.is_empty(){return None;}
        Some(Self{groups,anchor,client,group:0,item:None})
    }

    #[cfg_attr(not(all(unix,feature="x11")),allow(dead_code))]
    pub(crate) fn resize(&mut self,client:(i32,i32)){
        self.client=client;
    }

    pub(crate) fn root_rect(&self)->Rect{
        let chars=self.groups.iter().map(|(title,_)|title.chars().count()+3).max().unwrap_or(8);
        let w=panel_width(chars).min(self.client.0.max(1));
        let h=(self.groups.len() as i32*ROW_H+BORDER*2).min(self.client.1.max(1));
        Rect{x:clamp_origin(self.anchor.0,w,self.client.0),y:clamp_origin(self.anchor.1,h,self.client.1),w,h}
    }

    pub(crate) fn sub_rect(&self)->Option<Rect>{
        let items=&self.groups.get(self.group)?.1;
        if items.is_empty(){return None;}
        let root=self.root_rect();
        let chars=items.iter().map(|(_,label)|label.chars().count()).max().unwrap_or(6);
        let w=panel_width(chars).min(self.client.0.max(1));
        let h=(items.len() as i32*ROW_H+BORDER*2).min(self.client.1.max(1));
        let right=root.x+root.w-1;
        let x=if right+w<=self.client.0{right}else{(root.x-w+1).max(0)};
        let y=clamp_origin(root.y+self.group as i32*ROW_H,h,self.client.1);
        Some(Rect{x,y,w,h})
    }

    fn row_at(rect:Rect,count:usize,y:i32)->Option<usize>{
        if y<rect.y+BORDER{return None;}
        let row=((y-rect.y-BORDER)/ROW_H) as usize;
        (row<count).then_some(row)
    }

    pub(crate) fn pointer_move(&mut self,x:i32,y:i32)->Action{
        let root=self.root_rect();
        if let Some(sub)=self.sub_rect(){
            if sub.contains(x,y){
                if let Some(row)=Self::row_at(sub,self.groups[self.group].1.len(),y){
                    if self.item!=Some(row){self.item=Some(row);return Action::Redraw;}
                    return Action::Stay;
                }
            }
        }
        if root.contains(x,y){
            if let Some(row)=Self::row_at(root,self.groups.len(),y){
                if self.group!=row||self.item.is_some(){
                    self.group=row;
                    self.item=None;
                    return Action::Redraw;
                }
                return Action::Stay;
            }
        }
        if self.item.take().is_some(){return Action::Redraw;}
        Action::Stay
    }

    pub(crate) fn pointer_press(&mut self,x:i32,y:i32,button:u8)->Action{
        if button!=1{
            let on_root=self.root_rect().contains(x,y);
            let on_sub=self.sub_rect().is_some_and(|r|r.contains(x,y));
            return if on_root||on_sub{Action::Stay}else{Action::Cancel};
        }
        if let Some(sub)=self.sub_rect(){
            if sub.contains(x,y){
                if let Some(row)=Self::row_at(sub,self.groups[self.group].1.len(),y){
                    return Action::Choose(self.groups[self.group].1[row].0);
                }
                return Action::Stay;
            }
        }
        let root=self.root_rect();
        if root.contains(x,y){
            if let Some(row)=Self::row_at(root,self.groups.len(),y){
                self.group=row;
                self.item=(!self.groups[row].1.is_empty()).then_some(0);
                return Action::Redraw;
            }
            return Action::Stay;
        }
        Action::Cancel
    }

    pub(crate) fn key(&mut self,vk:u16)->Action{
        match vk{
            0x1b=>Action::Cancel, // Escape
            0x26=>{ // Up
                if let Some(i)=self.item{
                    let n=self.groups[self.group].1.len();
                    if n>0{self.item=Some((i+n-1)%n);}
                }else{
                    let n=self.groups.len();
                    self.group=(self.group+n-1)%n;
                }
                Action::Redraw
            },
            0x28=>{ // Down
                if let Some(i)=self.item{
                    let n=self.groups[self.group].1.len();
                    if n>0{self.item=Some((i+1)%n);}
                }else{
                    let n=self.groups.len();
                    self.group=(self.group+1)%n;
                }
                Action::Redraw
            },
            0x27=>{ // Right
                if self.item.is_none()&&!self.groups[self.group].1.is_empty(){
                    self.item=Some(0);
                    return Action::Redraw;
                }
                Action::Stay
            },
            0x25=>{ // Left
                if self.item.take().is_some(){return Action::Redraw;}
                Action::Stay
            },
            0x0d|0x20=>{ // Enter or Space
                if let Some(i)=self.item{
                    if let Some(&(id,_))=self.groups[self.group].1.get(i){return Action::Choose(id);}
                }else if !self.groups[self.group].1.is_empty(){
                    self.item=Some(0);
                    return Action::Redraw;
                }
                Action::Stay
            },
            _=>Action::Stay,
        }
    }

    /// Render the root panel and active submenu into `0x00RRGGBB` pixel buffers.
    pub(crate) fn render_panels(&self,font:&[u8])->Vec<(Rect,Vec<u32>)>{
        let mut out=Vec::with_capacity(2);
        let root=self.root_rect();
        let rows:Vec<String>=self.groups.iter().map(|(title,_)|title.clone()).collect();
        out.push((root,draw_panel(root,&rows,Some(self.group),true,font)));
        if let Some(sub)=self.sub_rect(){
            let items:Vec<String>=self.groups[self.group].1.iter().map(|(_,l)|l.clone()).collect();
            out.push((sub,draw_panel(sub,&items,self.item,false,font)));
        }
        out
    }
}

fn draw_panel(rect:Rect,rows:&[String],selected:Option<usize>,submenu_arrows:bool,font:&[u8])->Vec<u32>{
    let (w,h)=(rect.w.max(1) as usize,rect.h.max(1) as usize);
    let mut pixels=vec![BG;w*h];
    for x in 0..w{
        pixels[x]=BORDER_COLOR;
        pixels[(h-1)*w+x]=BORDER_COLOR;
    }
    for y in 0..h{
        pixels[y*w]=BORDER_COLOR;
        pixels[y*w+w-1]=BORDER_COLOR;
    }
    for (row,label) in rows.iter().enumerate(){
        let y0=BORDER+row as i32*ROW_H;
        let y1=(y0+ROW_H).min(rect.h-BORDER);
        if y0>=y1{break;}
        let active=selected==Some(row);
        let (bg,fg)=if active{(HILITE_BG,HILITE_TEXT)}else{(BG,TEXT)};
        for y in y0..y1{
            let row_slice=&mut pixels[y as usize*w+BORDER as usize..y as usize*w+w-BORDER as usize];
            row_slice.fill(bg);
        }
        let text_y=y0+(ROW_H-CHAR_H)/2;
        let max_chars=((rect.w-BORDER*2-PAD_X*2)/CHAR_W).max(0) as usize;
        for (i,ch) in label.chars().take(max_chars).enumerate(){
            draw_char(&mut pixels,w,h,BORDER+PAD_X+i as i32*CHAR_W,text_y,ch,fg,font);
        }
        if submenu_arrows{
            let arrow_x=rect.w-BORDER-PAD_X-CHAR_W;
            if arrow_x>=BORDER{draw_char(&mut pixels,w,h,arrow_x,text_y,'>',fg,font);}
        }
    }
    pixels
}

fn draw_char(pixels:&mut [u32],w:usize,h:usize,x0:i32,y0:i32,ch:char,fg:u32,font:&[u8]){
    let code=if (ch as u32)<256{ch as usize}else{b'?' as usize};
    let offset=code*CELL_HEIGHT;
    if offset+CELL_HEIGHT>font.len(){return;}
    for dy in 0..CHAR_H{
        let py=y0+dy;
        if py<0||py>=h as i32{continue;}
        let bits=font[offset+dy as usize];
        for dx in 0..CHAR_W{
            if bits&(0x80>>dx)==0{continue;}
            let px=x0+dx;
            if px>=0&&px<w as i32{pixels[py as usize*w+px as usize]=fg;}
        }
    }
}

#[cfg(test)]
mod tests{
    use super::*;

    fn sample_groups()->MenuGroups{
        vec![
            ("Clipboard".into(),vec![(1,"Copy".into()),(2,"Paste".into())]),
            ("Diagnostics".into(),vec![(3,"Save snapshot".into()),(5,"Pause".into())]),
        ]
    }

    #[test]
    fn empty_menu_does_not_open(){
        assert!(PopupMenu::open(Vec::new(),(10,10),(640,400)).is_none());
    }

    #[test]
    fn submenu_flips_left_when_anchored_near_right_edge(){
        let menu=PopupMenu::open(sample_groups(),(700,380),(720,400)).unwrap();
        let root=menu.root_rect();
        let sub=menu.sub_rect().unwrap();
        assert!(root.x+root.w<=720);
        assert!(root.y+root.h<=400);
        assert!(sub.x<root.x);
        assert!(sub.y+sub.h<=400);
    }

    #[test]
    fn keyboard_and_pointer_navigation_select_or_cancel(){
        let mut menu=PopupMenu::open(sample_groups(),(20,20),(800,600)).unwrap();
        assert_eq!(menu.key(0x28),Action::Redraw); // Down -> Diagnostics
        assert_eq!(menu.key(0x27),Action::Redraw); // Right -> submenu item 0
        assert_eq!(menu.key(0x28),Action::Redraw); // Down -> Pause (id 5)
        assert_eq!(menu.key(0x0d),Action::Choose(5));
        assert_eq!(menu.key(0x25),Action::Redraw); // Left -> back to root
        assert_eq!(menu.key(0x1b),Action::Cancel);

        // Pointer move into submenu + left click chooses item; click outside cancels.
        let sub=menu.sub_rect().unwrap();
        assert_eq!(menu.pointer_move(sub.x+5,sub.y+BORDER+2),Action::Redraw);
        assert_eq!(menu.pointer_press(sub.x+5,sub.y+BORDER+2,1),Action::Choose(3));
        assert_eq!(menu.pointer_press(790,590,1),Action::Cancel);
    }

    #[test]
    fn rendered_panels_draw_glyphs_using_rom_font(){
        let font=crate::bios::rom_font::font().unwrap();
        let mut menu=PopupMenu::open(sample_groups(),(20,20),(800,600)).unwrap();
        let panels=menu.render_panels(&font);
        assert_eq!(panels.len(),2);
        assert!(panels[0].1.contains(&HILITE_TEXT));
        for (rect,pixels) in &panels{
            assert_eq!(pixels.len(),(rect.w*rect.h) as usize);
            assert!(pixels.contains(&TEXT));
            assert!(pixels.contains(&BORDER_COLOR));
        }
        assert_eq!(menu.key(0x27),Action::Redraw);
        let panels=menu.render_panels(&font);
        assert!(panels[1].1.contains(&HILITE_TEXT));
        menu.resize((320,200));
        assert!(menu.root_rect().x+menu.root_rect().w<=320);
    }
}
