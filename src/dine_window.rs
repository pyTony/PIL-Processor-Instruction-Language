//! Reusable Win32/GDI RGB framebuffer window for Dine (pil and the Dine
//! library). No interpreter or DOS dependency.
//! `Frame` contains host RGB pixels, never guest state or pointers.

#[cfg(windows)]
pub(crate) struct Frame {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) pixels: Vec<u32>,
}

#[cfg(windows)]
pub(crate) use self::windows::Window;

#[cfg(windows)]
mod windows {
    use super::Frame;
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
    const CW_USEDEFAULT:i32=i32::MIN;
    const DIB_RGB_COLORS:u32=0;
    const SRCCOPY:u32=0x00cc0020;
    const COLOR_WINDOW:isize=6;

    static FRAME:OnceLock<Mutex<Option<Frame>>>=OnceLock::new();
    static MOUSE_EVENTS:OnceLock<Mutex<VecDeque<(u16,u16,u16)>>>=OnceLock::new();
    static MOUSE_MAX_X:AtomicU16=AtomicU16::new(639);
    static MOUSE_MAX_Y:AtomicU16=AtomicU16::new(199);
    static MOUSE_VISIBLE:AtomicBool=AtomicBool::new(false);
    static WINDOW:AtomicIsize=AtomicIsize::new(0);
    static CLOSED:AtomicBool=AtomicBool::new(false);
    static PAINT_REPORTED:AtomicBool=AtomicBool::new(false);

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
        fn LoadCursorW(instance:Handle,name:usize)->Handle;
        fn SetCursor(cursor:Handle)->Handle;
    }
    #[link(name="gdi32")]
    extern "system" {
        fn StretchDIBits(dc:Handle,dx:i32,dy:i32,dw:i32,dh:i32,
            sx:i32,sy:i32,sw:i32,sh:i32,bits:*const c_void,
            info:*const BitmapInfo,usage:u32,rop:u32)->i32;
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
                                let drawn=StretchDIBits(dc,0,0,rect.right-rect.left,
                                    rect.bottom-rect.top,0,0,frame.width,frame.height,
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
            WM_MOUSEMOVE|WM_LBUTTONDOWN|WM_LBUTTONUP|WM_RBUTTONDOWN|WM_RBUTTONUP|WM_MBUTTONDOWN|WM_MBUTTONUP=>{
                let mut client=Rect::default();
                if GetClientRect(hwnd,&mut client)!=0 && client.right>0 && client.bottom>0 {
                    let cx=((l as usize&0xffff) as u16 as i16) as i32;
                    let cy=(((l as usize>>16)&0xffff) as u16 as i16) as i32;
                    let max_x=i32::from(MOUSE_MAX_X.load(Ordering::Relaxed));
                    let max_y=i32::from(MOUSE_MAX_Y.load(Ordering::Relaxed));
                    let vx=((cx*(max_x+1))/client.right).clamp(0,max_x) as u16;
                    let vy=((cy*(max_y+1))/client.bottom).clamp(0,max_y) as u16;
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
            let title:Vec<u16>="Dine VGA (keyboard in terminal)\0".encode_utf16().collect();
            let instance=GetModuleHandleW(ptr::null());
            let descriptor=WndClass{
                style:0,proc:Some(wndproc),class_extra:0,window_extra:0,
                instance,icon:ptr::null_mut(),cursor:LoadCursorW(ptr::null_mut(),32512),
                background:(COLOR_WINDOW+1) as Handle,
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
            let hwnd=CreateWindowExW(WS_EX_NOACTIVATE,class.as_ptr(),title.as_ptr(),
                WS_OVERLAPPEDWINDOW,CW_USEDEFAULT,CW_USEDEFAULT,664,520,
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
            let previous_visible=ShowWindow(hwnd,SW_SHOWNOACTIVATE);
            let hwnd_topmost=(-1isize) as Handle;
            let hwnd_notopmost=(-2isize) as Handle;
            let swp_flags=0x0001u32|0x0002u32|0x0010u32; // SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE
            SetWindowPos(hwnd,hwnd_topmost,0,0,0,0,swp_flags);
            SetWindowPos(hwnd,hwnd_notopmost,0,0,0,0,swp_flags);
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
