//! X11 window for Dine on Linux and Termux:X11, the counterpart of the Win32
//! window in `dine_window`. It takes the same `Frame` pictures and sends the
//! same `Input` events, so `--dos-console --window` runs the same console loop
//! for a DOS program and for a booted Linux (text screen, VGA graphics modes,
//! keyboard edges with scan codes, mouse). Built with `--features x11`; a
//! pure Rust X11 client (x11rb), no libX11.
//!
//! One thread reads X events and sends `Input` (keys through
//! `dine_x11_keys`); the caller's thread draws, scaled in whole numbers
//! (`dine_window::view`), under a lock shared with the event thread.
use crate::dine_window::{border_rects,initial_scale,view,Frame,Input,MenuGroups};
use crate::dine_x11_keys::{self as keys,Keymap};
use crate::dine_x11_menu::{Action as MenuAction,PopupMenu,Rect};
use std::io::Write as _;
use std::sync::atomic::{AtomicBool,Ordering};
use std::sync::{mpsc,Arc,Mutex};
use std::thread;
use std::time::{Duration,Instant};
use x11rb::connection::{Connection,RequestConnection};
use x11rb::protocol::xkb::{self,ConnectionExt as _};
use x11rb::protocol::xproto::{self as x,ConnectionExt as _};
use x11rb::protocol::Event as XEvent;
use x11rb::rust_connection::{DefaultStream,RustConnection};
use x11rb::wrapper::ConnectionExt as _;
use x11rb::COPY_DEPTH_FROM_PARENT;
use x11rb::CURRENT_TIME;

/// The display's number and screen from `DISPLAY` (":0", ":1.0"), `:0` when
/// it is not set: Termux:X11 is display 0.
fn display_number()->(String,usize){
    let name=std::env::var("DISPLAY").ok().filter(|d|!d.is_empty()).unwrap_or_else(||":0".into());
    let tail=name.rsplit(':').next().unwrap_or("0");
    let mut parts=tail.split('.');
    let number=parts.next().unwrap_or("0").to_string();
    let screen=parts.next().and_then(|s|s.parse().ok()).unwrap_or(0);
    (number,screen)
}

/// Connect to the X server. Termux has no /tmp: its X server's socket is in
/// `$TMPDIR` (or `$PREFIX/tmp`), which a stock X11 client does not look in.
fn connect()->Result<(RustConnection,usize),String>{
    let first=match RustConnection::connect(None){Ok(c)=>return Ok(c),Err(e)=>e.to_string()};
    let (number,screen)=display_number();
    let mut dirs=Vec::new();
    for var in ["TMPDIR","XDG_RUNTIME_DIR"]{if let Ok(d)=std::env::var(var){dirs.push(std::path::PathBuf::from(d));}}
    if let Ok(prefix)=std::env::var("PREFIX"){dirs.push(std::path::Path::new(&prefix).join("tmp"));}
    for dir in dirs{
        let socket=dir.join(".X11-unix").join(format!("X{number}"));
        let Ok(stream)=std::os::unix::net::UnixStream::connect(&socket) else{continue;};
        let Ok((stream,_))=DefaultStream::from_unix_stream(stream) else{continue;};
        if let Ok(c)=RustConnection::connect_to_stream(stream,screen){return Ok((c,screen));}
    }
    Err(format!("cannot connect to the X display ({}): {first}. Start an X server first (Termux: termux-x11 :0, then export DISPLAY=:0).",
        std::env::var("DISPLAY").unwrap_or_else(|_|"DISPLAY not set".into())))
}

/// Whether an X server can be reached, for a clear error before the console
/// starts.
pub(crate) fn check()->Result<(),String>{connect().map(|_|())}

/// How the server wants a 32-bit pixel: the colour masks and byte order.
#[derive(Clone,Copy,Debug)]
struct Format{red:u32,green:u32,blue:u32,msb_first:bool}
impl Format{
    fn channel(mask:u32,value:u32)->u32{
        let shift=mask.trailing_zeros().min(31);
        let bits=(mask>>shift).count_ones().min(8);
        ((value&0xff)>>(8-bits))<<shift
    }
    /// A 0x00RRGGBB host pixel as the server's pixel value.
    fn pack(&self,pixel:u32)->u32{
        Self::channel(self.red,pixel>>16)|Self::channel(self.green,pixel>>8)|Self::channel(self.blue,pixel)
    }
}

struct Shared{
    conn:RustConnection,
    win:x::Window,
    gc:x::Gcontext,
    blank:x::Cursor,
    format:Format,
    max_bytes:usize,
    wake:x::Atom,
    net_wm_name:x::Atom,
    utf8:x::Atom,
    frame:Mutex<Option<Frame>>,
    /// The window's client size.
    client:Mutex<(i32,i32)>,
    last_view:Mutex<Option<(i32,i32,i32,i32)>>,
    last_pointer:Mutex<(i32,i32)>,
    menu:Mutex<Option<PopupMenu>>,
    font:Vec<u8>,
    draw:Mutex<()>,
    closed:AtomicBool,
    stop:AtomicBool,
    hidden:AtomicBool,
    keymap:Mutex<Keymap>,
}

impl Shared{
    fn load_keymap(&self){
        let setup=self.conn.setup();
        let (min,max)=(setup.min_keycode,setup.max_keycode);
        let Ok(cookie)=self.conn.get_keyboard_mapping(min,max-min+1) else{return;};
        let Ok(reply)=cookie.reply() else{return;};
        if let Ok(mut map)=self.keymap.lock(){
            *map=Keymap{min,per:usize::from(reply.keysyms_per_keycode),syms:reply.keysyms};
        }
    }
    /// The picture's place in the window, from the current frame.
    fn picture(&self)->(i32,i32,i32,i32){
        let (cw,ch)=self.client.lock().map_or((0,0),|c|*c);
        let size=self.frame.lock().ok().and_then(|f|f.as_ref().map(|f|(f.width,f.height)));
        match size{Some((fw,fh))=>view(cw,ch,fw,fh),None=>(0,0,cw,ch)}
    }
    fn apply_cursor(&self){
        let menu_open=self.menu.lock().ok().is_some_and(|m|m.is_some());
        let cursor=if !menu_open&&self.hidden.load(Ordering::Acquire){self.blank}else{x11rb::NONE};
        let _=self.conn.change_window_attributes(self.win,&x::ChangeWindowAttributesAux::new().cursor(cursor));
        let _=self.conn.flush();
    }
    fn put_rgb(&self,rect:Rect,pixels:&[u32])->bool{
        if rect.w<=0||rect.h<=0||pixels.len()<(rect.w*rect.h) as usize{return true;}
        let stride=rect.w as usize*4;
        let rows=((self.max_bytes.saturating_sub(64))/stride.max(1)).clamp(1,rect.h as usize);
        let mut y=0;
        while y<rect.h{
            let band=rows.min((rect.h-y) as usize);
            let mut data=Vec::with_capacity(band*stride);
            for row in 0..band{
                let start=(y as usize+row)*rect.w as usize;
                for &pixel in &pixels[start..start+rect.w as usize]{
                    let value=self.format.pack(pixel);
                    data.extend_from_slice(&if self.format.msb_first{value.to_be_bytes()}else{value.to_le_bytes()});
                }
            }
            if self.conn.put_image(x::ImageFormat::Z_PIXMAP,self.win,self.gc,rect.w as u16,band as u16,rect.x as i16,(rect.y+y) as i16,0,24,&data).is_err(){
                self.closed.store(true,Ordering::Release);
                return false;
            }
            y+=band as i32;
        }
        true
    }
    /// Draw the current frame: nearest-neighbour, whole-number scaling, clearing
    /// borders whenever the picture rectangle shrinks or moves, plus any open popup menu.
    fn draw(&self){self.draw_inner(false);}
    fn draw_inner(&self,clear_borders:bool){
        let Ok(_busy)=self.draw.lock() else{return;};
        let (cw,ch)=self.client.lock().map_or((0,0),|c|*c);
        let menu=self.menu.lock().ok().and_then(|m|m.clone());
        let Ok(frame)=self.frame.lock() else{return;};
        if let Some(frame)=frame.as_ref(){
            if frame.width>0&&frame.height>0&&frame.pixels.len()>=(frame.width*frame.height) as usize{
                let (vx,vy,vw,vh)=view(cw,ch,frame.width,frame.height);
                let mut view_changed=false;
                if let Ok(mut prev)=self.last_view.lock(){
                    view_changed=*prev!=Some((vx,vy,vw,vh));
                    *prev=Some((vx,vy,vw,vh));
                }
                if clear_borders||view_changed{
                    for (bx,by,bw,bh) in border_rects(cw,ch,vx,vy,vw,vh){
                        let _=self.conn.clear_area(false,self.win,bx as i16,by as i16,bw as u16,bh as u16);
                    }
                }
                if vw>0&&vh>0{
                    let packed:Vec<u32>=frame.pixels.iter().map(|&p|self.format.pack(p)).collect();
                    let columns:Vec<usize>=(0..vw).map(|x|(i64::from(x)*i64::from(frame.width)/i64::from(vw)) as usize).collect();
                    let stride=vw as usize*4;
                    let rows=((self.max_bytes.saturating_sub(64))/stride.max(1)).clamp(1,vh as usize);
                    let mut y=0;
                    while y<vh{
                        let band=rows.min((vh-y) as usize);
                        let mut data=Vec::with_capacity(band*stride);
                        for row in 0..band{
                            let sy=(i64::from(y)+row as i64)*i64::from(frame.height)/i64::from(vh);
                            let source=&packed[sy as usize*frame.width as usize..][..frame.width as usize];
                            for &c in &columns{
                                let value=source[c];
                                data.extend_from_slice(&if self.format.msb_first{value.to_be_bytes()}else{value.to_le_bytes()});
                            }
                        }
                        let sent=self.conn.put_image(x::ImageFormat::Z_PIXMAP,self.win,self.gc,vw as u16,band as u16,vx as i16,(vy+y) as i16,0,24,&data);
                        if sent.is_err(){self.closed.store(true,Ordering::Release);return;}
                        y+=band as i32;
                    }
                }
            }
        }
        drop(frame);
        if let Some(menu)=menu{
            for (rect,pixels) in menu.render_panels(&self.font){
                if !self.put_rgb(rect,&pixels){return;}
            }
        }
        let _=self.conn.flush();
    }
    fn set_title(&self,text:&str){
        let _=self.conn.change_property8(x::PropMode::REPLACE,self.win,x::AtomEnum::WM_NAME,x::AtomEnum::STRING,text.as_bytes());
        let _=self.conn.change_property8(x::PropMode::REPLACE,self.win,self.net_wm_name,self.utf8,text.as_bytes());
        let _=self.conn.flush();
    }
    fn focus(&self){
        let _=self.conn.set_input_focus(x::InputFocus::PARENT,self.win,CURRENT_TIME);
        let _=self.conn.flush();
    }
}

fn atom(conn:&RustConnection,name:&str)->Result<x::Atom,String>{
    conn.intern_atom(false,name.as_bytes()).map_err(|e|e.to_string())?.reply().map(|r|r.atom).map_err(|e|e.to_string())
}

/// Create and map the window, sized to a whole number of times the first
/// picture.
fn open(title:&str,frame_w:i32,frame_h:i32)->Result<Arc<Shared>,String>{
    let (conn,screen_number)=connect()?;
    let setup=conn.setup();
    let screen=setup.roots.get(screen_number).ok_or("the X server has no such screen")?;
    let depth=screen.root_depth;
    let visual=screen.allowed_depths.iter().filter(|d|d.depth==depth).flat_map(|d|d.visuals.iter())
        .find(|v|v.visual_id==screen.root_visual).ok_or("the X server's root visual is unknown")?;
    let bits=setup.pixmap_formats.iter().find(|f|f.depth==depth).map_or(0,|f|f.bits_per_pixel);
    if bits!=32||visual.red_mask==0{return Err(format!("the X server's pixel format ({depth} bits deep, {bits} per pixel) is not supported; a 24-bit TrueColor screen is needed"));}
    let format=Format{red:visual.red_mask,green:visual.green_mask,blue:visual.blue_mask,msb_first:setup.image_byte_order==x::ImageOrder::MSB_FIRST};
    let setting=std::env::var("DINE_WINDOW_SCALE").ok();
    let scale=initial_scale(setting.as_deref(),i32::from(screen.width_in_pixels)-16,i32::from(screen.height_in_pixels)-48,frame_w,frame_h);
    let (width,height)=((frame_w*scale).clamp(1,16000),(frame_h*scale).clamp(1,16000));
    let win=conn.generate_id().map_err(|e|e.to_string())?;
    let events=x::EventMask::KEY_PRESS|x::EventMask::KEY_RELEASE|x::EventMask::BUTTON_PRESS|x::EventMask::BUTTON_RELEASE
        |x::EventMask::POINTER_MOTION|x::EventMask::EXPOSURE|x::EventMask::STRUCTURE_NOTIFY|x::EventMask::FOCUS_CHANGE|x::EventMask::KEYMAP_STATE;
    conn.create_window(COPY_DEPTH_FROM_PARENT,win,screen.root,0,0,width as u16,height as u16,0,x::WindowClass::INPUT_OUTPUT,screen.root_visual,
        &x::CreateWindowAux::new().background_pixel(screen.black_pixel).event_mask(events)).map_err(|e|e.to_string())?;
    let gc=conn.generate_id().map_err(|e|e.to_string())?;
    conn.create_gc(gc,win,&x::CreateGCAux::new()).map_err(|e|e.to_string())?;
    // A pointer with no shape, for programs that draw their own.
    let pixmap=conn.generate_id().map_err(|e|e.to_string())?;
    let blank=conn.generate_id().map_err(|e|e.to_string())?;
    conn.create_pixmap(1,pixmap,win,1,1).map_err(|e|e.to_string())?;
    conn.create_cursor(blank,pixmap,pixmap,0,0,0,0,0,0,0,0).map_err(|e|e.to_string())?;
    conn.free_pixmap(pixmap).map_err(|e|e.to_string())?;
    let protocols=atom(&conn,"WM_PROTOCOLS")?;
    let delete=atom(&conn,"WM_DELETE_WINDOW")?;
    let net_wm_name=atom(&conn,"_NET_WM_NAME")?;
    let utf8=atom(&conn,"UTF8_STRING")?;
    let wake=atom(&conn,"PIL_WAKE")?;
    conn.change_property32(x::PropMode::REPLACE,win,protocols,x::AtomEnum::ATOM,&[delete]).map_err(|e|e.to_string())?;
    conn.change_property8(x::PropMode::REPLACE,win,x::AtomEnum::WM_CLASS,x::AtomEnum::STRING,b"pil\0Dine\0").map_err(|e|e.to_string())?;
    // A held key then arrives as repeated presses with no releases between
    // (a PC's typematic), not as release/press pairs the console would take
    // for a key bouncing. Without the XKB extension the pairs are joined
    // below by their time stamp, which is less certain.
    if let Ok(cookie)=conn.xkb_use_extension(1,0){
        if cookie.reply().is_ok_and(|r|r.supported){
            let flag=xkb::PerClientFlag::DETECTABLE_AUTO_REPEAT;
            let none=xkb::BoolCtrl::from(0u32);
            if let Ok(cookie)=conn.xkb_per_client_flags(xkb::ID::USE_CORE_KBD.into(),flag,flag,none,none,none){let _=cookie.reply();}
        }
    }
    let max_bytes=conn.maximum_request_bytes().min(1<<20);
    let font=crate::bios::rom_font::font().unwrap_or_else(|_|vec![0;256*crate::bios::rom_font::CELL_HEIGHT]);
    let shared=Arc::new(Shared{conn,win,gc,blank,format,max_bytes,wake,net_wm_name,utf8,
        frame:Mutex::new(None),client:Mutex::new((width,height)),last_view:Mutex::new(None),
        last_pointer:Mutex::new((24,24)),menu:Mutex::new(None),font,draw:Mutex::new(()),
        closed:AtomicBool::new(false),stop:AtomicBool::new(false),hidden:AtomicBool::new(false),keymap:Mutex::new(Keymap::default())});
    shared.set_title(title);
    shared.load_keymap();
    shared.conn.map_window(win).map_err(|e|e.to_string())?;
    shared.conn.flush().map_err(|e|e.to_string())?;
    Ok(shared)
}

/// X events to console input, until the window is closed or dropped.
fn events(shared:&Arc<Shared>,input:&Mutex<Option<mpsc::Sender<Input>>>){
    let send=|i:Input|{if let Ok(g)=input.lock(){if let Some(tx)=g.as_ref(){let _=tx.send(i);}}};
    let close_menu=|id:u16|{
        if let Ok(mut m)=shared.menu.lock(){*m=None;}
        shared.apply_cursor();
        shared.draw_inner(true);
        send(Input::Menu{id});
    };
    let mut buttons:u16=0;
    let mut focused=false;
    let mut queued:Option<XEvent>=None;
    let protocols=atom(&shared.conn,"WM_PROTOCOLS").unwrap_or(0);
    loop{
        let event=match queued.take(){
            Some(e)=>e,
            None=>match shared.conn.wait_for_event(){Ok(e)=>e,Err(_)=>{shared.closed.store(true,Ordering::Release);send(Input::FocusLost);return;}},
        };
        if shared.stop.load(Ordering::Acquire){return;}
        match event{
            XEvent::Expose(e)=>{
                if !focused{focused=true;shared.focus();}
                if e.count==0{shared.draw_inner(true);}
            },
            XEvent::ConfigureNotify(e)=>{
                let size=(i32::from(e.width),i32::from(e.height));
                let changed=shared.client.lock().map(|mut c|{let old=*c;*c=size;old!=size}).unwrap_or(false);
                if changed{
                    if let Ok(mut prev)=shared.last_view.lock(){*prev=None;}
                    if let Ok(mut m)=shared.menu.lock(){if let Some(menu)=m.as_mut(){menu.resize(size);}}
                    let _=shared.conn.clear_area(false,shared.win,0,0,0,0);
                    shared.draw_inner(true);
                }
            },
            XEvent::MappingNotify(_)=>shared.load_keymap(),
            XEvent::KeyPress(k)=>{
                let info=keys::translate(&shared.keymap.lock().map(|m|m.clone()).unwrap_or_default(),k.detail,k.state.into());
                let action=shared.menu.lock().ok().and_then(|mut m|m.as_mut().map(|menu|menu.key(info.vk)));
                if let Some(action)=action{
                    match action{
                        MenuAction::Stay=>{},
                        MenuAction::Redraw=>shared.draw_inner(false),
                        MenuAction::Choose(id)=>close_menu(id),
                        MenuAction::Cancel=>close_menu(0),
                    }
                    continue;
                }
                if info.scan==0{continue;}
                send(Input::Physical{vk:info.vk,scan:info.scan,extended:info.extended,down:true,repeat:1,shift:info.shift,ctrl:info.ctrl,alt:info.alt});
                if info.dead{send(Input::DeadKey);}
                else if let Some(ch)=info.text{send(Input::Text{scan:info.scan,extended:info.extended,ch,shift:info.shift,ctrl:info.ctrl,alt:info.alt});}
            },
            XEvent::KeyRelease(k)=>{
                if shared.menu.lock().ok().is_some_and(|m|m.is_some()){continue;}
                // The server repeats a held key as a release followed by a
                // press with the same time: that is one held key, not two.
                match shared.conn.poll_for_event(){
                    Ok(Some(XEvent::KeyPress(p)))if p.detail==k.detail&&p.time==k.time=>{queued=Some(XEvent::KeyPress(p));continue;},
                    Ok(Some(other))=>queued=Some(other),
                    _=>{},
                }
                let info=keys::translate(&shared.keymap.lock().map(|m|m.clone()).unwrap_or_default(),k.detail,k.state.into());
                if info.scan==0{continue;}
                send(Input::Physical{vk:info.vk,scan:info.scan,extended:info.extended,down:false,repeat:1,shift:info.shift,ctrl:info.ctrl,alt:info.alt});
            },
            XEvent::FocusOut(_)=>{
                if shared.menu.lock().ok().is_some_and(|m|m.is_some()){close_menu(0);}
                send(Input::FocusLost);
            },
            XEvent::ButtonPress(b)|XEvent::ButtonRelease(b)=>{
                if let Ok(mut p)=shared.last_pointer.lock(){*p=(i32::from(b.event_x),i32::from(b.event_y));}
                let down=matches!(event_kind(&b),Kind::Press);
                let action=shared.menu.lock().ok().and_then(|mut m|m.as_mut().map(|menu|if down{menu.pointer_press(i32::from(b.event_x),i32::from(b.event_y),b.detail)}else{MenuAction::Stay}));
                if let Some(action)=action{
                    match action{
                        MenuAction::Stay=>{},
                        MenuAction::Redraw=>shared.draw_inner(false),
                        MenuAction::Choose(id)=>close_menu(id),
                        MenuAction::Cancel=>close_menu(0),
                    }
                    continue;
                }
                let bit=match b.detail{1=>1,3=>2,2=>4,_=>0};
                if bit!=0{if down{buttons|=bit;shared.focus();}else{buttons&=!bit;}}
                else{continue;}
                let (px,py,pw,ph)=shared.picture();
                send(Input::Mouse{x:i32::from(b.event_x)-px,y:i32::from(b.event_y)-py,width:pw,height:ph,buttons,shift:u16::from(b.state)&keys::SHIFT!=0});
            },
            XEvent::MotionNotify(m)=>{
                if let Ok(mut p)=shared.last_pointer.lock(){*p=(i32::from(m.event_x),i32::from(m.event_y));}
                let action=shared.menu.lock().ok().and_then(|mut g|g.as_mut().map(|menu|menu.pointer_move(i32::from(m.event_x),i32::from(m.event_y))));
                if let Some(action)=action{
                    match action{
                        MenuAction::Redraw=>shared.draw_inner(false),
                        MenuAction::Stay|MenuAction::Choose(_)|MenuAction::Cancel=>{},
                    }
                    continue;
                }
                let (px,py,pw,ph)=shared.picture();
                send(Input::Mouse{x:i32::from(m.event_x)-px,y:i32::from(m.event_y)-py,width:pw,height:ph,buttons,shift:u16::from(m.state)&keys::SHIFT!=0});
            },
            XEvent::ClientMessage(c)=>{
                if c.type_==protocols&&c.format==32{
                    if shared.menu.lock().ok().is_some_and(|m|m.is_some()){close_menu(0);}
                    send(Input::FocusLost);
                    shared.closed.store(true,Ordering::Release);
                    return;
                }
            },
            XEvent::DestroyNotify(_)=>{
                if shared.menu.lock().ok().is_some_and(|m|m.is_some()){close_menu(0);}
                shared.closed.store(true,Ordering::Release);send(Input::FocusLost);return;
            },
            _=>{},
        }
    }
}

/// X reports a button press and release with the same event structure; the
/// kind is the event it arrived as.
enum Kind{Press,Release}
fn event_kind(b:&x::ButtonPressEvent)->Kind{
    if b.response_type&0x7f==x::BUTTON_RELEASE_EVENT{Kind::Release}else{Kind::Press}
}

/// A window with the same surface as the Win32 one.
pub(crate) struct Window{
    title:Mutex<String>,
    input:Arc<Mutex<Option<mpsc::Sender<Input>>>>,
    shared:Option<Arc<Shared>>,
    worker:Option<thread::JoinHandle<()>>,
    failed:bool,
}

impl Window{
    /// A window that takes keyboard and mouse input and sends it to `input`;
    /// it opens at the first picture.
    pub(crate) fn interactive(title:&str,input:mpsc::Sender<Input>)->Self{
        Self{title:Mutex::new(title.into()),input:Arc::new(Mutex::new(Some(input))),shared:None,worker:None,failed:false}
    }
    pub(crate) fn no_frame(&mut self){}
    pub(crate) fn closed(&self)->bool{self.shared.as_ref().is_some_and(|s|s.closed.load(Ordering::Acquire))}
    /// Hide the host pointer over the window, or show it again.
    pub(crate) fn hide_pointer(&self,hidden:bool){
        let Some(s)=self.shared.as_ref() else{return;};
        if s.hidden.swap(hidden,Ordering::AcqRel)==hidden{return;}
        s.apply_cursor();
    }
    /// Display a host popup over the window; IDs belong to the caller.
    pub(crate) fn popup(&self,groups:MenuGroups){
        if let Ok(g)=self.input.lock(){if let Some(tx)=g.as_ref(){let _=tx.send(Input::FocusLost);}}
        let Some(s)=self.shared.as_ref() else{
            if let Ok(g)=self.input.lock(){if let Some(tx)=g.as_ref(){let _=tx.send(Input::Menu{id:0});}}
            return;
        };
        let client=s.client.lock().map_or((640,400),|c|*c);
        let anchor=s.last_pointer.lock().map_or((24,24),|p|*p);
        let Some(menu)=PopupMenu::open(groups,anchor,client) else{
            if let Ok(g)=self.input.lock(){if let Some(tx)=g.as_ref(){let _=tx.send(Input::Menu{id:0});}}
            return;
        };
        if let Ok(mut slot)=s.menu.lock(){*slot=Some(menu);}
        s.apply_cursor();
        s.draw_inner(true);
    }
    pub(crate) fn title(&self,text:&str){
        if let Ok(mut slot)=self.title.lock(){*slot=text.into();}
        if let Some(s)=self.shared.as_ref(){s.set_title(text);}
    }
    pub(crate) fn present(&mut self,frame:Frame){
        if self.failed{return;}
        if self.shared.is_none(){
            let title=self.title.lock().map_or_else(|_|"PIL console".into(),|t|t.clone());
            match open(&title,frame.width,frame.height){
                Ok(shared)=>{
                    let (s,input)=(shared.clone(),self.input.clone());
                    match thread::Builder::new().name("dine-x11".into()).spawn(move||events(&s,&input)){
                        Ok(worker)=>self.worker=Some(worker),
                        Err(e)=>{eprintln!("Dine window: cannot start the X event thread: {e}");self.failed=true;return;},
                    }
                    self.shared=Some(shared);
                },
                Err(e)=>{eprintln!("Dine window: {e}");self.failed=true;return;},
            }
        }
        let Some(s)=self.shared.as_ref() else{return;};
        if let Ok(mut slot)=s.frame.lock(){*slot=Some(frame);}
        s.draw();
    }
}

impl Drop for Window{
    fn drop(&mut self){
        let Some(s)=self.shared.take() else{return;};
        s.stop.store(true,Ordering::Release);
        // Wake the event thread, which is waiting for the next X event.
        let wake=x::ClientMessageEvent::new(32,s.win,s.wake,[0u32;5]);
        let _=s.conn.send_event(false,s.win,x::EventMask::NO_EVENT,wake);
        let _=s.conn.flush();
        if let Some(worker)=self.worker.take(){let _=worker.join();}
        let _=s.conn.destroy_window(s.win);
        let _=s.conn.flush();
    }
}

/// Run `program` with `args`, text to its stdin when `text` is given, and
/// return its stdout; none when it is missing, fails or takes over 2 seconds.
fn tool(program:&str,args:&[&str],text:Option<&str>)->Option<String>{
    use std::process::{Command,Stdio};
    let mut child=Command::new(program).args(args).stdin(if text.is_some(){Stdio::piped()}else{Stdio::null()})
        .stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    if let Some(text)=text{
        let mut stdin=child.stdin.take()?;
        let data=text.to_string();
        // A thread, so a tool that does not read cannot stall the console.
        thread::spawn(move||{let _=stdin.write_all(data.as_bytes());});
    }
    let mut out=child.stdout.take()?;
    let reader=thread::spawn(move||{let mut s=String::new();let _=std::io::Read::read_to_string(&mut out,&mut s);s});
    let started=Instant::now();
    loop{
        match child.try_wait(){
            Ok(Some(status))=>return if status.success(){reader.join().ok()}else{None},
            Ok(None) if started.elapsed()<Duration::from_secs(2)=>thread::sleep(Duration::from_millis(10)),
            _=>{let _=child.kill();let _=child.wait();return None;},
        }
    }
}

/// Put `text` on the host clipboard with the first tool that takes it:
/// Termux:API, Wayland or X11 helpers. Returns whether one did.
pub(crate) fn set_clipboard(text:&str)->bool{
    tool("termux-clipboard-set",&[],Some(text)).is_some()
        ||tool("wl-copy",&[],Some(text)).is_some()
        ||tool("xclip",&["-selection","clipboard"],Some(text)).is_some()
        ||tool("xsel",&["--clipboard","--input"],Some(text)).is_some()
}

/// The host clipboard's text, from the same tools.
pub(crate) fn clipboard()->Option<String>{
    tool("termux-clipboard-get",&[],None)
        .or_else(||tool("wl-paste",&["--no-newline"],None))
        .or_else(||tool("xclip",&["-selection","clipboard","-o"],None))
        .or_else(||tool("xsel",&["--clipboard","--output"],None))
}

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn pixels_are_packed_by_the_servers_masks(){
        let rgb=Format{red:0xff0000,green:0x00ff00,blue:0x0000ff,msb_first:false};
        assert_eq!(rgb.pack(0x00123456),0x00123456);
        let bgr=Format{red:0x0000ff,green:0x00ff00,blue:0xff0000,msb_first:false};
        assert_eq!(bgr.pack(0x00123456),0x00563412);
        // A 5-6-5 screen keeps the top bits of each channel.
        let low=Format{red:0xf800,green:0x07e0,blue:0x001f,msb_first:false};
        assert_eq!(low.pack(0x00ffffff),0xffff);
        assert_eq!(low.pack(0),0);
    }

    #[test]
    fn the_display_comes_from_display_with_display_zero_as_default(){
        // Parsing only; the environment is left alone.
        let parse=|name:&str|{let tail=name.rsplit(':').next().unwrap_or("0");let mut p=tail.split('.');(p.next().unwrap_or("0").to_string(),p.next().and_then(|s|s.parse::<usize>().ok()).unwrap_or(0))};
        assert_eq!(parse(":0"),("0".to_string(),0));
        assert_eq!(parse(":3.1"),("3".to_string(),1));
        assert_eq!(parse("localhost:10.0"),("10".to_string(),0));
    }
}
