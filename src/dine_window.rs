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
    use std::ffi::c_void;
    use std::ptr;
    use std::sync::{atomic::{AtomicBool,AtomicIsize,Ordering},mpsc,Mutex,OnceLock};
    use std::thread;

    type Handle=*mut c_void;
    const WM_CLOSE:u32=0x10;
    const WM_DESTROY:u32=0x02;
    const WM_PAINT:u32=0x0f;
    const WS_OVERLAPPEDWINDOW:u32=0x00cf0000;
    const WS_EX_NOACTIVATE:u32=0x08000000;
    const SW_SHOWNOACTIVATE:i32=4;
    const CW_USEDEFAULT:i32=i32::MIN;
    const DIB_RGB_COLORS:u32=0;
    const SRCCOPY:u32=0x00cc0020;
    const COLOR_WINDOW:isize=6;

    static FRAME:OnceLock<Mutex<Option<Frame>>>=OnceLock::new();
    static WINDOW:AtomicIsize=AtomicIsize::new(0);
    static CLOSED:AtomicBool=AtomicBool::new(false);

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
        fn InvalidateRect(window:Handle,area:*const Rect,erase:i32)->i32;
        fn PostMessageW(window:Handle,message:u32,wparam:usize,lparam:isize)->i32;
        fn BeginPaint(window:Handle,paint:*mut Paint)->Handle;
        fn EndPaint(window:Handle,paint:*const Paint)->i32;
        fn GetClientRect(window:Handle,area:*mut Rect)->i32;
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
                                StretchDIBits(dc,0,0,rect.right-rect.left,
                                    rect.bottom-rect.top,0,0,frame.width,frame.height,
                                    frame.pixels.as_ptr().cast(),&info,DIB_RGB_COLORS,SRCCOPY);
                            }
                        }
                    }
                    EndPaint(hwnd,&paint);
                }
                0
            },
            WM_CLOSE=>{DestroyWindow(hwnd);0},
            WM_DESTROY=>{
                CLOSED.store(true,Ordering::Release);
                WINDOW.store(0,Ordering::Release);
                PostQuitMessage(0);
                0
            },
            _=>DefWindowProcW(hwnd,msg,w,l),
        }
    }

    fn run(ready:mpsc::Sender<isize>){
        unsafe{
            let class:Vec<u16>="DineVgaFrame\0".encode_utf16().collect();
            let title:Vec<u16>="Dine VGA (keyboard in terminal)\0".encode_utf16().collect();
            let instance=GetModuleHandleW(ptr::null());
            let descriptor=WndClass{
                style:0,proc:Some(wndproc),class_extra:0,window_extra:0,
                instance,icon:ptr::null_mut(),cursor:ptr::null_mut(),
                background:(COLOR_WINDOW+1) as Handle,
                menu_name:ptr::null(),class_name:class.as_ptr(),
            };
            if RegisterClassW(&descriptor)==0{
                eprintln!("Dine video: cannot register VGA window class");
                let _=ready.send(0);
                return;
            }
            let hwnd=CreateWindowExW(WS_EX_NOACTIVATE,class.as_ptr(),title.as_ptr(),
                WS_OVERLAPPEDWINDOW,CW_USEDEFAULT,CW_USEDEFAULT,664,520,
                ptr::null_mut(),ptr::null_mut(),instance,ptr::null_mut());
            if hwnd.is_null(){
                eprintln!("Dine video: cannot create VGA window");
                let _=ready.send(0);
                return;
            }
            CLOSED.store(false,Ordering::Release);
            WINDOW.store(hwnd as isize,Ordering::Release);
            let _=ready.send(hwnd as isize);
            ShowWindow(hwnd,SW_SHOWNOACTIVATE);
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
    }
    impl Window{
        pub(crate) fn new()->Self{Self{worker:None,ready:None}}
        pub(crate) fn closed(&self)->bool{self.worker.is_some()&&CLOSED.load(Ordering::Acquire)}
        pub(crate) fn present(&mut self,frame:Frame){
            let state=FRAME.get_or_init(||Mutex::new(None));
            if let Ok(mut guard)=state.lock(){*guard=Some(frame);}
            if self.worker.is_none(){
                let(tx,rx)=mpsc::channel();
                match thread::Builder::new().name("dine-vga".into()).spawn(move||run(tx)){
                    Ok(worker)=>{self.worker=Some(worker);self.ready=Some(rx);},
                    Err(e)=>eprintln!("Dine video: cannot start display thread: {e}"),
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
