//! Native host file/folder dialogs; no guest memory or guest input involved.
use std::{ffi::c_void,path::PathBuf,ptr};
type Handle=*mut c_void;
#[repr(C)]
struct OpenFile{
    size:u32,owner:Handle,instance:Handle,filter:*const u16,custom:*mut u16,
    max_custom:u32,filter_index:u32,file:*mut u16,max_file:u32,
    file_title:*mut u16,max_title:u32,initial:*const u16,title:*const u16,
    flags:u32,file_offset:u16,extension_offset:u16,default_extension:*const u16,
    data:isize,hook:usize,template:*const u16,reserved:*mut c_void,reserved_num:u32,flags_ex:u32,
}
#[link(name="comdlg32")]
extern "system"{fn GetOpenFileNameW(info:*mut OpenFile)->i32;fn CommDlgExtendedError()->u32;}
#[repr(C)]
struct Browse{owner:Handle,root:*const c_void,display:*mut u16,title:*const u16,flags:u32,callback:usize,data:isize,image:i32}
#[link(name="shell32")]
extern "system"{fn SHBrowseForFolderW(info:*const Browse)->*mut c_void;fn SHGetPathFromIDListW(id:*const c_void,path:*mut u16)->i32;}
#[link(name="ole32")]
extern "system"{fn CoInitializeEx(reserved:*mut c_void,mode:u32)->i32;fn CoUninitialize();fn CoTaskMemFree(memory:*mut c_void);}

fn path(buffer:&[u16])->PathBuf{
    use std::os::windows::ffi::OsStringExt;
    std::ffi::OsString::from_wide(&buffer[..buffer.iter().position(|&c|c==0).unwrap_or(buffer.len())]).into()
}
pub(super) fn file(manifest:bool)->Result<Option<PathBuf>,String>{
    let filter=if manifest{"Disk-set manifests\0*.lst;*.txt\0All files\0*.*\0\0"}else{"DOS media\0*.img;*.ima;*.lst\0All files\0*.*\0\0"};
    let filter:Vec<u16>=filter.encode_utf16().collect();
    let title:Vec<u16>=if manifest{"Select ordered disk-set manifest\0"}else{"Mount read-only disk image\0"}.encode_utf16().collect();
    let mut buffer=vec![0u16;32768];
    let mut info:OpenFile=unsafe{std::mem::zeroed()};
    info.size=std::mem::size_of::<OpenFile>() as u32;info.filter=filter.as_ptr();
    info.file=buffer.as_mut_ptr();info.max_file=buffer.len() as u32;info.title=title.as_ptr();
    // Resolve at runtime: never embed the developer's checkout directory.
    let initial=std::path::Path::new("images").canonicalize().ok();
    let initial:Option<Vec<u16>>=initial.as_ref().map(|p|{
        use std::os::windows::ffi::OsStrExt;
        p.as_os_str().encode_wide().chain(Some(0)).collect()
    });
    if let Some(initial)=initial.as_ref(){info.initial=initial.as_ptr();}
    // Existing files only; do not change the process working directory.
    info.flags=0x1000|0x800|0x8|0x80000;
    if unsafe{GetOpenFileNameW(&mut info)}!=0{return Ok(Some(path(&buffer)));}
    let error=unsafe{CommDlgExtendedError()};
    if error==0{Ok(None)}else{Err(format!("file dialog failed: {error:08X}"))}
}
pub(super) fn folder()->Result<Option<PathBuf>,String>{
    let result=unsafe{CoInitializeEx(ptr::null_mut(),2)};
    if result<0{return Err(format!("folder dialog initialization failed: {result:08X}"));}
    let mut display=vec![0u16;260];let mut buffer=vec![0u16;260];
    let title:Vec<u16>="Mount writable DOS folder\0".encode_utf16().collect();
    let info=Browse{owner:ptr::null_mut(),root:ptr::null(),display:display.as_mut_ptr(),title:title.as_ptr(),flags:0x1|0x40,callback:0,data:0,image:0};
    let id=unsafe{SHBrowseForFolderW(&info)};
    let chosen=if id.is_null(){Ok(None)}else{
        let ok=unsafe{SHGetPathFromIDListW(id,buffer.as_mut_ptr())};
        unsafe{CoTaskMemFree(id)};
        if ok!=0{Ok(Some(path(&buffer)))}else{Err("selected folder has no filesystem path".into())}
    };
    unsafe{CoUninitialize()};chosen
}
