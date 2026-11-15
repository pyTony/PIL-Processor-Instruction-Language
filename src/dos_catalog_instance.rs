//! One host catalog per Windows login session; the OS releases the guard on exit.
#[cfg(windows)]
use std::{ffi::c_void,ptr};
pub(super) struct Instance{
    #[cfg(windows)]
    handle:*mut c_void,
}
#[cfg(windows)]
#[link(name="kernel32")]
extern "system"{
    fn CreateMutexW(attributes:*const c_void,owner:i32,name:*const u16)->*mut c_void;
    fn GetLastError()->u32;
    fn CloseHandle(handle:*mut c_void)->i32;
}
#[cfg(windows)]
#[link(name="user32")]
extern "system"{
    fn FindWindowW(class:*const u16,title:*const u16)->*mut c_void;
    fn IsIconic(window:*mut c_void)->i32;
    fn ShowWindow(window:*mut c_void,command:i32)->i32;
    fn SetForegroundWindow(window:*mut c_void)->i32;
}
impl Instance{
    pub(super) fn acquire()->Result<Option<Self>,String>{
        #[cfg(windows)]
        {
            let instance=Self::named("Local\\PIL.RetroBrowser")?;
            if instance.is_none(){
                let title:Vec<u16>="PIL Retro Browser\0".encode_utf16().collect();
                let window=unsafe{FindWindowW(ptr::null(),title.as_ptr())};
                if !window.is_null(){unsafe{if IsIconic(window)!=0{ShowWindow(window,9);}SetForegroundWindow(window);}}
            }
            Ok(instance)
        }
        #[cfg(not(windows))]
        {Ok(Some(Self{}))}
    }
    #[cfg(windows)]
    fn named(name:&str)->Result<Option<Self>,String>{
        let name:Vec<u16>=name.encode_utf16().chain(Some(0)).collect();
        let handle=unsafe{CreateMutexW(ptr::null(),0,name.as_ptr())};
        let error=unsafe{GetLastError()};
        if handle.is_null(){return Err(format!("browser instance guard failed: {error}"));}
        if error==183{unsafe{CloseHandle(handle)};return Ok(None);}
        Ok(Some(Self{handle}))
    }
}
impl Drop for Instance{
    fn drop(&mut self){
        #[cfg(windows)]
        unsafe{CloseHandle(self.handle);}
    }
}
#[cfg(all(test,windows))]
mod tests{
    use super::*;
    #[test]
    fn catalog_single_instance_rejects_duplicates_and_releases_on_close(){
        let name=format!("Local\\PIL.CatalogTest.{}",std::process::id());
        let first=Instance::named(&name).unwrap().unwrap();
        assert!(Instance::named(&name).unwrap().is_none());
        drop(first);
        assert!(Instance::named(&name).unwrap().is_some());
    }
}
