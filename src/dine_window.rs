//! Reusable Win32/GDI RGB framebuffer window for Dine (pil and the Dine
//! library). No interpreter or DOS dependency.
//! `Frame` contains host RGB pixels, never guest state or pointers.

#[cfg(windows)]
pub(crate) struct Frame {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) pixels: Vec<u32>,
}

/// Input from an interactive window, before any DOS mapping: a typed
/// character, a key that types none (Windows virtual-key code), or the mouse
/// in client coordinates.
#[cfg(windows)]
#[derive(Clone,Copy,Debug)]
pub(crate) enum Input{
    Char{ch:char,shift:bool,ctrl:bool,alt:bool},
    Key{vk:u16,shift:bool,ctrl:bool,alt:bool},
    Mouse{x:i32,y:i32,width:i32,height:i32,buttons:u16},
}

#[cfg(windows)]
pub(crate) use self::windows::Window;

/// Where a `frame_w` x `frame_h` picture goes in a client area, centred
/// (x, y, width, height): the largest whole-number enlargement that fits, so
/// every guest pixel is the same number of host pixels. When that would use
/// less than 85% of the size that fits with the picture's proportions (a
/// 640x480 mode in a window made for 2x text), it is drawn at that size
/// instead, proportions kept.
#[cfg_attr(not(windows),allow(dead_code))]
pub(crate) fn view(client_w:i32,client_h:i32,frame_w:i32,frame_h:i32)->(i32,i32,i32,i32){
    if client_w<=0||client_h<=0||frame_w<=0||frame_h<=0{return (0,0,client_w.max(0),client_h.max(0));}
    let (cw,ch,fw,fh)=(i64::from(client_w),i64::from(client_h),i64::from(frame_w),i64::from(frame_h));
    let (fit_w,fit_h)=if cw*fh<=ch*fw{(cw,(cw*fh/fw).max(1))}else{((ch*fw/fh).max(1),ch)};
    let scale=(cw/fw).min(ch/fh);
    let (w,h)=if scale>=1&&fw*scale*100>=fit_w*85{(fw*scale,fh*scale)}else{(fit_w,fit_h)};
    (((cw-w)/2) as i32,((ch-h)/2) as i32,w as i32,h as i32)
}

/// The whole-number enlargement for a new window: `DINE_WINDOW_SCALE` (1-8)
/// when set, otherwise the largest that fits the free desktop area
/// (`room_w` x `room_h`, without the window frame), at least 1.
#[cfg_attr(not(windows),allow(dead_code))]
pub(crate) fn initial_scale(setting:Option<&str>,room_w:i32,room_h:i32,frame_w:i32,frame_h:i32)->i32{
    if let Some(n)=setting.and_then(|s|s.trim().parse::<i32>().ok()).filter(|n|(1..=8).contains(n)){return n;}
    if frame_w<=0||frame_h<=0{return 1;}
    (room_w/frame_w).min(room_h/frame_h).clamp(1,8)
}

#[cfg(windows)]
mod windows {
    use super::{Frame,Input,view,initial_scale};
    // This module is also copied into generated standalone DOS executables.
    // Keep diagnostics self-contained: standalone code has no dine_display.
    fn debug_log(message:&str) {
        let Some(path)=std::env::var_os("DINE_VIDEO_LOG") else {return;};
        if path.is_empty(){return;}
        let milliseconds=std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0,|elapsed|elapsed.as_millis());
        let line=format!("{milliseconds} pid={} {message}\n",std::process::id());
        use std::io::Write as _;
        let _=std::fs::OpenOptions::new().create(true).append(true)
            .open(std::path::Path::new(&path))
            .and_then(|mut file|file.write_all(line.as_bytes()));
    }
    use std::collections::VecDeque;
    use std::ffi::c_void;
    use std::ptr;
    use std::sync::{atomic::{AtomicBool,AtomicIsize,AtomicU16,Ordering},mpsc,Mutex,OnceLock};
    use std::thread;

    type Handle=*mut c_void;
    const WM_CLOSE:u32=0x10;
    const WM_DESTROY:u32=0x02;
    const WM_PAINT:u32=0x0f;
    const WM_SETCURSOR:u32=0x20;
    const WM_MOUSEMOVE:u32=0x0200;
    const WM_LBUTTONDOWN:u32=0x0201;
    const WM_LBUTTONUP:u32=0x0202;
    const WM_RBUTTONDOWN:u32=0x0204;
    const WM_RBUTTONUP:u32=0x0205;
    const WM_MBUTTONDOWN:u32=0x0207;
    const WM_MBUTTONUP:u32=0x0208;
    const WS_OVERLAPPEDWINDOW:u32=0x00cf0000;
    const WS_EX_NOACTIVATE:u32=0x08000000;
    const SW_SHOWNOACTIVATE:i32=4;
    const SW_SHOWNORMAL:i32=1;
    const CW_USEDEFAULT:i32=i32::MIN;
    const DIB_RGB_COLORS:u32=0;
    const SRCCOPY:u32=0x00cc0020;
    const BLACK_BRUSH:i32=4;
    const BLACKNESS:u32=0x00000042;
    const CS_VREDRAW:u32=0x0001;
    const CS_HREDRAW:u32=0x0002;
    const SPI_GETWORKAREA:u32=0x0030;

    static FRAME:OnceLock<Mutex<Option<Frame>>>=OnceLock::new();
    static MOUSE_EVENTS:OnceLock<Mutex<VecDeque<(u16,u16,u16)>>>=OnceLock::new();
    static MOUSE_MAX_X:AtomicU16=AtomicU16::new(639);
    static MOUSE_MAX_Y:AtomicU16=AtomicU16::new(199);
    static MOUSE_VISIBLE:AtomicBool=AtomicBool::new(false);
    static WINDOW:AtomicIsize=AtomicIsize::new(0);
    static CLOSED:AtomicBool=AtomicBool::new(false);
    static PAINT_REPORTED:AtomicBool=AtomicBool::new(false);
    /// Set for a window that takes keyboard and mouse input (the DOS console).
    static INPUT:OnceLock<Mutex<Option<mpsc::Sender<Input>>>>=OnceLock::new();
    static TITLE:OnceLock<Mutex<String>>=OnceLock::new();
    const WM_KEYDOWN:u32=0x100;
    const WM_CHAR:u32=0x102;
    const WM_SYSKEYDOWN:u32=0x104;
    const WM_SYSCHAR:u32=0x106;
    const VK_SHIFT:i32=0x10;
    const VK_CONTROL:i32=0x11;
    const VK_MENU:i32=0x12;
    const VK_F4:usize=0x73;

    fn send(input:Input)->bool{
        let Some(state)=INPUT.get() else {return false;};
        let Ok(guard)=state.lock() else {return false;};
        match guard.as_ref(){Some(tx)=>{let _=tx.send(input);true},None=>false}
    }
    fn held(vk:i32)->bool{unsafe{GetKeyState(vk)<0}}
    /// Keys that type no character reach WM_KEYDOWN only: arrows, the editing
    /// block, F1-F12 and Ctrl+Break.
    fn untyped(vk:usize)->bool{matches!(vk,0x03|0x21..=0x28|0x2d|0x2e|0x70..=0x7b)}

    #[repr(C)]
    #[derive(Default,Clone,Copy)]
    struct Point{x:i32,y:i32}
    #[repr(C)]
    #[derive(Default,Clone,Copy)]
    struct Rect{left:i32,top:i32,right:i32,bottom:i32}
    #[repr(C)]
    struct Msg{
        hwnd:Handle,message:u32,wparam:usize,lparam:isize,
        time:u32,point:Point,private:u32,
    }
    #[repr(C)]
    struct WndClass{
        style:u32,
        proc:Option<unsafe extern "system" fn(Handle,u32,usize,isize)->isize>,
        class_extra:i32,window_extra:i32,
        instance:Handle,icon:Handle,cursor:Handle,
        background:Handle,menu_name:*const u16,class_name:*const u16,
    }
    #[repr(C)]
    #[derive(Default)]
    struct Paint{
        dc:Handle,erase:i32,rect:Rect,restore:i32,increment:i32,
        reserved:[u8;32],
    }
    #[repr(C)]
    struct BitmapInfoHeader{
        size:u32,width:i32,height:i32,planes:u16,bits:u16,
        compression:u32,image_size:u32,xppm:i32,yppm:i32,
        used:u32,important:u32,
    }
    #[repr(C)]
    struct BitmapInfo{header:BitmapInfoHeader,colors:[u32;1]}

    #[link(name="kernel32")]
    extern "system" {
        fn GetModuleHandleW(name:*const u16)->Handle;
        fn GetLastError()->u32;
    }
    #[link(name="user32")]
    extern "system" {
        fn RegisterClassW(class:*const WndClass)->u16;
        fn CreateWindowExW(ex:u32,class:*const u16,title:*const u16,style:u32,
            x:i32,y:i32,width:i32,height:i32,parent:Handle,menu:Handle,
            instance:Handle,param:*mut c_void)->Handle;
        fn DefWindowProcW(window:Handle,message:u32,wparam:usize,lparam:isize)->isize;
        fn DestroyWindow(window:Handle)->i32;
        fn PostQuitMessage(code:i32);
        fn GetMessageW(message:*mut Msg,window:Handle,min:u32,max:u32)->i32;
        fn TranslateMessage(message:*const Msg)->i32;
        fn DispatchMessageW(message:*const Msg)->isize;
        fn ShowWindow(window:Handle,how:i32)->i32;
        fn SetWindowPos(window:Handle,after:Handle,x:i32,y:i32,cx:i32,cy:i32,flags:u32)->i32;
        fn InvalidateRect(window:Handle,area:*const Rect,erase:i32)->i32;
        fn PostMessageW(window:Handle,message:u32,wparam:usize,lparam:isize)->i32;
        fn BeginPaint(window:Handle,paint:*mut Paint)->Handle;
        fn EndPaint(window:Handle,paint:*const Paint)->i32;
        fn GetClientRect(window:Handle,area:*mut Rect)->i32;
        fn GetWindowRect(window:Handle,area:*mut Rect)->i32;
        fn IsWindowVisible(window:Handle)->i32;
        fn GetKeyState(key:i32)->i16;
        fn SetFocus(window:Handle)->Handle;
        fn LoadCursorW(instance:Handle,name:usize)->Handle;
        fn SetCursor(cursor:Handle)->Handle;
        fn AdjustWindowRectEx(area:*mut Rect,style:u32,menu:i32,ex:u32)->i32;
        fn SystemParametersInfoW(action:u32,param:u32,data:*mut c_void,ini:u32)->i32;
    }
    #[link(name="gdi32")]
    extern "system" {
        fn StretchDIBits(dc:Handle,dx:i32,dy:i32,dw:i32,dh:i32,
            sx:i32,sy:i32,sw:i32,sh:i32,bits:*const c_void,
            info:*const BitmapInfo,usage:u32,rop:u32)->i32;
        fn PatBlt(dc:Handle,x:i32,y:i32,w:i32,h:i32,rop:u32)->i32;
        fn GetStockObject(object:i32)->Handle;
    }

    /// The picture's place in the client area, from the current frame.
    fn frame_view(client:&Rect)->Option<(i32,i32,i32,i32)>{
        let guard=FRAME.get()?.lock().ok()?;
        let frame=guard.as_ref()?;
        Some(view(client.right-client.left,client.bottom-client.top,frame.width,frame.height))
    }

    unsafe extern "system" fn wndproc(hwnd:Handle,msg:u32,w:usize,l:isize)->isize{
        match msg{
            WM_PAINT=>{
                let mut paint=Paint::default();
                let dc=BeginPaint(hwnd,&mut paint);
                if !dc.is_null(){
                    let mut rect=Rect::default();
                    GetClientRect(hwnd,&mut rect);
                    if let Some(state)=FRAME.get(){
                        if let Ok(guard)=state.lock(){
                            if let Some(frame)=guard.as_ref(){
                                let info=BitmapInfo{
                                    header:BitmapInfoHeader{
                                        size:std::mem::size_of::<BitmapInfoHeader>() as u32,
                                        width:frame.width,height:-frame.height,
                                        planes:1,bits:32,compression:0,image_size:0,
                                        xppm:0,yppm:0,used:0,important:0,
                                    },
                                    colors:[0],
                                };
                                let (cw,ch)=(rect.right-rect.left,rect.bottom-rect.top);
                                let (x,y,w,h)=view(cw,ch,frame.width,frame.height);
                                // Black borders around a picture that does not fill the window.
                                PatBlt(dc,0,0,cw,y,BLACKNESS);
                                PatBlt(dc,0,y+h,cw,ch-y-h,BLACKNESS);
                                PatBlt(dc,0,y,x,h,BLACKNESS);
                                PatBlt(dc,x+w,y,cw-x-w,h,BLACKNESS);
                                let drawn=StretchDIBits(dc,x,y,w,h,0,0,frame.width,frame.height,
                                    frame.pixels.as_ptr().cast(),&info,DIB_RGB_COLORS,SRCCOPY);
                                if !PAINT_REPORTED.swap(true,Ordering::AcqRel) {
                                    debug_log(&format!("WM_PAINT first GDI result={drawn}; client={}x{}; scanout={}x{}",
                                        rect.right-rect.left,rect.bottom-rect.top,frame.width,frame.height));
                                }
                            }
                        }
                    }
                    EndPaint(hwnd,&paint);
                }
                0
            },
            WM_KEYDOWN|WM_SYSKEYDOWN if untyped(w)&&!(msg==WM_SYSKEYDOWN&&w==VK_F4)=>{
                let input=Input::Key{vk:w as u16,shift:held(VK_SHIFT),ctrl:held(VK_CONTROL),alt:held(VK_MENU)};
                if send(input){0}else{DefWindowProcW(hwnd,msg,w,l)}
            },
            WM_CHAR|WM_SYSCHAR=>{
                let Some(ch)=char::from_u32(w as u32) else {return 0;};
                let (ctrl,alt)=(held(VK_CONTROL),msg==WM_SYSCHAR||held(VK_MENU));
                // AltGr arrives as Ctrl+Alt with the character it typed.
                let altgr=ctrl&&alt&&!ch.is_control();
                let input=Input::Char{ch,shift:held(VK_SHIFT),ctrl:ctrl&&!altgr,alt:alt&&!altgr};
                if send(input){0}else{DefWindowProcW(hwnd,msg,w,l)}
            },
            WM_MOUSEMOVE|WM_LBUTTONDOWN|WM_LBUTTONUP|WM_RBUTTONDOWN|WM_RBUTTONUP|WM_MBUTTONDOWN|WM_MBUTTONUP=>{
                let mut client=Rect::default();
                if GetClientRect(hwnd,&mut client)==0{return 0;}
                // The pointer relative to the picture, not to the borders around it.
                let (px,py,pw,ph)=frame_view(&client).unwrap_or((0,0,client.right,client.bottom));
                let x=i32::from(l as u16 as i16)-px;
                let y=i32::from((l>>16) as u16 as i16)-py;
                // An interactive console window maps the pointer itself.
                let buttons=(w&1) as u16|((w>>1)&1) as u16*2|((w>>4)&1) as u16*4;
                if send(Input::Mouse{x,y,width:pw,height:ph,buttons}){return 0;}
                if pw>0 && ph>0 {
                    let max_x=i32::from(MOUSE_MAX_X.load(Ordering::Relaxed));
                    let max_y=i32::from(MOUSE_MAX_Y.load(Ordering::Relaxed));
                    let vx=((x.clamp(0,pw-1)*(max_x+1))/pw).clamp(0,max_x) as u16;
                    let vy=((y.clamp(0,ph-1)*(max_y+1))/ph).clamp(0,max_y) as u16;
                    let buttons=u16::from(w&1!=0)
                        |(u16::from(w&2!=0)<<1)
                        |(u16::from(w&0x10!=0)<<2);
                    let queue=MOUSE_EVENTS.get_or_init(||Mutex::new(VecDeque::new()));
                    if let Ok(mut guard)=queue.lock(){
                        if msg==WM_MOUSEMOVE && guard.back().is_some_and(|e|e.2==buttons) {
                            *guard.back_mut().unwrap()=(vx,vy,buttons);
                        } else {
                            if guard.len()>=64{guard.pop_front();}
                            guard.push_back((vx,vy,buttons));
                        }
                    }
                }
                0
            },
            WM_SETCURSOR=>{
                if (l as usize&0xffff)==1 {
                    let cursor=if MOUSE_VISIBLE.load(Ordering::Acquire){ptr::null_mut()}else{LoadCursorW(ptr::null_mut(),32512)};
                    SetCursor(cursor);
                    1
                } else {
                    DefWindowProcW(hwnd,msg,w,l)
                }
            },
            WM_CLOSE=>{DestroyWindow(hwnd);0},
            WM_DESTROY=>{
                debug_log("WM_DESTROY: VGA window closed");
                CLOSED.store(true,Ordering::Release);
                WINDOW.store(0,Ordering::Release);
                PostQuitMessage(0);
                0
            },
            _=>DefWindowProcW(hwnd,msg,w,l),
        }
    }

    fn run(ready:mpsc::Sender<isize>){
        debug_log("Win32 video worker entered");
        unsafe{
            let class:Vec<u16>="DineVgaFrame\0".encode_utf16().collect();
            let title=TITLE.get().and_then(|t|t.lock().ok().map(|t|t.clone()))
                .unwrap_or_else(||"Dine VGA (keyboard in terminal)".into());
            let title:Vec<u16>=title.encode_utf16().chain([0]).collect();
            let interactive=INPUT.get().is_some_and(|i|i.lock().is_ok_and(|i|i.is_some()));
            let instance=GetModuleHandleW(ptr::null());
            let descriptor=WndClass{
                style:CS_HREDRAW|CS_VREDRAW,proc:Some(wndproc),class_extra:0,window_extra:0,
                instance,icon:ptr::null_mut(),cursor:LoadCursorW(ptr::null_mut(),32512),
                background:GetStockObject(BLACK_BRUSH),
                menu_name:ptr::null(),class_name:class.as_ptr(),
            };
            if RegisterClassW(&descriptor)==0{
                let error=GetLastError();
                debug_log(&format!("RegisterClassW FAILED GetLastError={error}"));
                eprintln!("Dine video: cannot register VGA window class ({error})");
                let _=ready.send(0);
                return;
            }
            debug_log("RegisterClassW succeeded; calling CreateWindowExW (WS_EX_NOACTIVATE)");
            let ex_style=if interactive{0}else{WS_EX_NOACTIVATE};
            // Size the client area to the first picture times a whole number,
            // the largest that fits the desktop (720x400 text: 1440x800 on a
            // 1920x1080 screen).
            let (fw,fh)=FRAME.get().and_then(|f|f.lock().ok().and_then(|f|f.as_ref().map(|f|(f.width,f.height)))).unwrap_or((640,480));
            let mut decor=Rect::default();
            AdjustWindowRectEx(&mut decor,WS_OVERLAPPEDWINDOW,0,ex_style);
            let (decor_w,decor_h)=(decor.right-decor.left,decor.bottom-decor.top);
            let mut work=Rect{left:0,top:0,right:1024,bottom:768};
            SystemParametersInfoW(SPI_GETWORKAREA,0,(&mut work as *mut Rect).cast(),0);
            let setting=std::env::var("DINE_WINDOW_SCALE").ok();
            let scale=initial_scale(setting.as_deref(),work.right-work.left-decor_w,work.bottom-work.top-decor_h,fw,fh);
            let mut outer=Rect{left:0,top:0,right:fw*scale,bottom:fh*scale};
            AdjustWindowRectEx(&mut outer,WS_OVERLAPPEDWINDOW,0,ex_style);
            let (outer_w,outer_h)=(outer.right-outer.left,outer.bottom-outer.top);
            debug_log(&format!("window scale {scale} for {fw}x{fh}: outer {outer_w}x{outer_h}"));
            let hwnd=CreateWindowExW(ex_style,class.as_ptr(),title.as_ptr(),
                WS_OVERLAPPEDWINDOW,CW_USEDEFAULT,CW_USEDEFAULT,outer_w,outer_h,
                ptr::null_mut(),ptr::null_mut(),instance,ptr::null_mut());
            if hwnd.is_null(){
                let error=GetLastError();
                debug_log(&format!("CreateWindowExW FAILED GetLastError={error}"));
                eprintln!("Dine video: cannot create VGA window ({error})");
                let _=ready.send(0);
                return;
            }
            debug_log(&format!("CreateWindowExW succeeded HWND={hwnd:p}"));
            CLOSED.store(false,Ordering::Release);
            WINDOW.store(hwnd as isize,Ordering::Release);
            let _=ready.send(hwnd as isize);
            let previous_visible=ShowWindow(hwnd,if interactive{SW_SHOWNORMAL}else{SW_SHOWNOACTIVATE});
            let hwnd_topmost=(-1isize) as Handle;
            let hwnd_notopmost=(-2isize) as Handle;
            let swp_flags=0x0001u32|0x0002u32|0x0010u32; // SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE
            SetWindowPos(hwnd,hwnd_topmost,0,0,0,0,swp_flags);
            SetWindowPos(hwnd,hwnd_notopmost,0,0,0,0,swp_flags);
            if interactive{SetFocus(hwnd);}
            let now_visible=IsWindowVisible(hwnd);
            let mut bounds=Rect::default();
            let rect_ok=GetWindowRect(hwnd,&mut bounds);
            debug_log(&format!("ShowWindow SW_SHOWNOACTIVATE: previous_visible={previous_visible} now_visible={now_visible} rect_ok={rect_ok} bounds=({},{},{},{})",
                bounds.left,bounds.top,bounds.right,bounds.bottom));
            InvalidateRect(hwnd,ptr::null(),0);
            let mut msg=std::mem::MaybeUninit::<Msg>::uninit();
            loop{
                let received=GetMessageW(msg.as_mut_ptr(),ptr::null_mut(),0,0);
                if received<=0{break;}
                let message=msg.assume_init_ref();
                TranslateMessage(message);
                DispatchMessageW(message);
            }
            WINDOW.store(0,Ordering::Release);
        }
    }

    /// Presentation thread is independent of the DOS interpreter. Guest
    /// input remains in the native terminal; closing the window does not
    /// synthesize a DOS key or terminate the DOS process.
    pub(crate) struct Window{
        worker:Option<thread::JoinHandle<()>>,
        ready:Option<mpsc::Receiver<isize>>,
        last_had_frame:Option<bool>,
    }
    impl Window{
        pub(crate) fn new()->Self{Self{worker:None,ready:None,last_had_frame:None}}
        /// A window that takes keyboard and mouse input and sends it to
        /// `input`. One per process, as the frame is.
        pub(crate) fn interactive(title:&str,input:mpsc::Sender<Input>)->Self{
            if let Ok(mut slot)=INPUT.get_or_init(||Mutex::new(None)).lock(){*slot=Some(input);}
            if let Ok(mut slot)=TITLE.get_or_init(||Mutex::new(String::new())).lock(){*slot=title.into();}
            Self::new()
        }
        pub(crate) fn no_frame(&mut self) {
            if self.last_had_frame!=Some(false) {
                debug_log("scanout absent: text mode or unsupported graphics BDA mode");
                self.last_had_frame=Some(false);
            }
        }
        pub(crate) fn closed(&self)->bool{self.worker.is_some()&&CLOSED.load(Ordering::Acquire)}
        pub(crate) fn set_mouse_state(&self,max_x:u16,max_y:u16,visible:bool){
            MOUSE_MAX_X.store(max_x,Ordering::Relaxed);
            MOUSE_MAX_Y.store(max_y,Ordering::Relaxed);
            MOUSE_VISIBLE.store(visible,Ordering::Release);
        }
        pub(crate) fn drain_mouse_events(&self)->Vec<(u16,u16,u16)>{
            let Some(queue)=MOUSE_EVENTS.get() else {return Vec::new();};
            queue.lock().map_or_else(|_|Vec::new(),|mut guard|guard.drain(..).collect())
        }
        pub(crate) fn present(&mut self,frame:Frame){
            if self.last_had_frame!=Some(true) {
                debug_log(&format!("scanout ready: {}x{} RGB pixels",frame.width,frame.height));
                self.last_had_frame=Some(true);
            }
            let state=FRAME.get_or_init(||Mutex::new(None));
            if let Ok(mut guard)=state.lock(){*guard=Some(frame);}
            if self.worker.is_none(){
                debug_log("spawning Win32 video worker");
                let(tx,rx)=mpsc::channel();
                match thread::Builder::new().name("dine-vga".into()).spawn(move||run(tx)){
                    Ok(worker)=>{self.worker=Some(worker);self.ready=Some(rx);debug_log("Win32 worker spawn succeeded");},
                    Err(e)=>{debug_log(&format!("Win32 worker spawn FAILED: {e}"));eprintln!("Dine video: cannot start display thread: {e}");},
                }
            }
            let hwnd=WINDOW.load(Ordering::Acquire);
            if hwnd!=0{
                unsafe{InvalidateRect(hwnd as Handle,ptr::null(),0);}
            }
        }
    }

    // A host may unload dine.dll immediately after dine_run returns.
    // The window thread MUST be stopped and joined before that happens,
    // otherwise its Win32 callback could execute unmapped DLL code.
    impl Drop for Window {
        fn drop(&mut self){
            debug_log("Win32 video owner dropped; closing worker if present");
            if let Some(worker)=self.worker.take(){
                let hwnd=self.ready.take().and_then(|rx|rx.recv().ok()).unwrap_or(0);
                if hwnd!=0 && WINDOW.load(Ordering::Acquire)==hwnd {
                    unsafe{PostMessageW(hwnd as Handle,WM_CLOSE,0,0);}
                }
                if worker.join().is_err(){
                    eprintln!("Dine video: the VGA window worker panicked");
                }
            }
        }
    }
}
