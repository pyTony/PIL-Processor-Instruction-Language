use super::*;
use crate::dos::{load,files,session::Session};

struct Fixture{root:PathBuf,images:[Vec<u8>;2]}
impl Fixture{
    fn new()->Self{
        let nonce=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let root=std::env::temp_dir().join(format!("pil-media-{}-{nonce}",std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let images=[image(b"one"),image(b"two")];
        for (n,bytes) in images.iter().enumerate(){std::fs::write(root.join(format!("disk{}.img",n+1)),bytes).unwrap();}
        std::fs::write(root.join("set.lst"),"# ordered media\r\ndisk1.img\r\ndisk2.img\r\n").unwrap();
        Self{root,images}
    }
}
impl Drop for Fixture{fn drop(&mut self){std::fs::remove_dir_all(&self.root).unwrap();}}
fn image(marker:&[u8])->Vec<u8>{
    let mut b=vec![0;720*512];b[..3].copy_from_slice(&[0xeb,0x3c,0x90]);
    b[11..13].copy_from_slice(&512u16.to_le_bytes());b[13]=1;b[14..16].copy_from_slice(&1u16.to_le_bytes());b[16]=2;
    b[17..19].copy_from_slice(&112u16.to_le_bytes());b[19..21].copy_from_slice(&720u16.to_le_bytes());b[21]=0xf9;b[22..24].copy_from_slice(&3u16.to_le_bytes());
    for fat in 0..2{let at=512+fat*3*512;b[at..at+6].copy_from_slice(&[0xf9,0xff,0xff,0xff,0x0f,0]);}
    let at=7*512;b[at..at+11].copy_from_slice(b"MARKER  TXT");b[at+26..at+28].copy_from_slice(&2u16.to_le_bytes());b[at+28..at+32].copy_from_slice(&(marker.len() as u32).to_le_bytes());
    b[14*512..14*512+marker.len()].copy_from_slice(marker);b
}
fn marker(session:&Session)->Vec<u8>{files::program(session.machine().unwrap(),b"A:\\MARKER.TXT").unwrap().unwrap().0.to_vec()}
#[test]
fn disk_set_swaps_without_restarting_and_keeps_images_read_only(){
    let f=Fixture::new();let mut s=Session::new(load(&[0xf4],"sample.com").unwrap());
    assert!(s.disk_set(b'A',&f.root.join("set.lst")).unwrap().contains("disk 1/2"));
    assert_eq!(marker(&s),b"one");let cpu=s.machine().unwrap().cpu.clone();let ram=s.machine().unwrap().ram.clone();
    assert!(s.disk(b'A',Some("next")).unwrap().contains("disk 2/2"));assert_eq!(marker(&s),b"two");
    assert_eq!(s.machine().unwrap().cpu,cpu);assert_eq!(s.machine().unwrap().ram,ram);
    assert!(s.disk(b'A',Some("next")).is_err());assert_eq!(marker(&s),b"two");
    s.disk(b'A',Some("prev")).unwrap();assert_eq!(marker(&s),b"one");
    s.disk(b'A',Some("2")).unwrap();assert_eq!(marker(&s),b"two");
    for (n,bytes) in f.images.iter().enumerate(){assert_eq!(&std::fs::read(f.root.join(format!("disk{}.img",n+1))).unwrap(),bytes);}
}
#[test]
fn disk_set_refuses_busy_or_bad_media_without_changing_the_inserted_disk(){
    let f=Fixture::new();let mut s=Session::new(load(&[0xf4],"sample.com").unwrap());s.disk_set(b'A',&f.root.join("set.lst")).unwrap();
    let p=s.machine_mut().unwrap();let address=u32::from(p.image.selectors[crate::address::SegmentRegister::Ds.index()])*16+0x300;
    p.ram.write_bytes(address,b"A:\\MARKER.TXT\0").unwrap();p.cpu.regs[0]=0x3d00;p.cpu.regs[2]=0x300;
    files::service(p,0x3d).unwrap();assert_eq!(p.cpu.eflags&1,0);let handle=p.cpu.regs[0]&65535;
    assert!(s.disk(b'A',Some("next")).unwrap_err().contains("open files"));assert_eq!(marker(&s),b"one");
    let p=s.machine_mut().unwrap();p.cpu.regs[3]=handle;files::service(p,0x3e).unwrap();
    std::fs::write(f.root.join("disk2.img"),b"not a floppy").unwrap();
    assert!(s.disk(b'A',Some("next")).is_err());assert!(s.disk(b'A',None).unwrap().contains("disk 1/2"));assert_eq!(marker(&s),b"one");
    assert!(s.disk_set(b'C',&f.root.join("set.lst")).is_err());
    s.unmount_media(b'A').unwrap();assert!(s.disk(b'A',None).is_err());
}
#[test]
fn disk_set_manifest_validation_and_session_protocol(){
    let f=Fixture::new();
    for text in ["", "disk1.img\ndisk1.img", "missing.img", "disk1.exe", &"disk1.img\n".repeat(100)]{
        std::fs::write(f.root.join("bad.lst"),text).unwrap();assert!(DiskSet::read(&f.root.join("bad.lst")).is_err());
    }
    std::fs::write(f.root.join("bad.lst"),vec![b' ';65537]).unwrap();assert!(DiskSet::read(&f.root.join("bad.lst")).is_err());
    let mut s=Session::new(load(&[0xf4],"sample.com").unwrap());
    for command in [format!("diskset A {}",f.root.join("set.lst").display()),"disk A next".into(),"disk A 1".into(),"disk A".into()]{
        let (reply,quit)=s.command(1,&command).unwrap();assert!(reply.contains("\"ok\":true"),"{reply}");assert!(!quit);
    }
    for command in ["disk A 0","disk A 3","disk A prev","disk B next"]{
        let (reply,_)=s.command(2,command).unwrap();assert!(reply.contains("\"ok\":false"),"{reply}");
    }
    let (reply,_)=s.command(3,&format!("mount A {}",f.root.join("disk2.img").display())).unwrap();assert!(reply.contains("\"ok\":true"));
    assert!(s.disk(b'A',None).is_err());assert_eq!(marker(&s),b"two");
}

#[test]
fn disk_set_native_browsing_detects_only_matching_numbered_siblings(){
    let f=Fixture::new();
    std::fs::write(f.root.join("unrelated2.img"),b"unrelated").unwrap();
    std::fs::write(f.root.join("disk03.img"),b"different digit width").unwrap();
    std::fs::write(f.root.join("disk10.img"),&f.images[0]).unwrap();
    let set=DiskSet::numbered(&f.root.join("disk2.img")).unwrap().unwrap();
    assert_eq!(set.images.iter().map(|p|p.file_name().unwrap().to_str().unwrap()).collect::<Vec<_>>(),vec!["disk1.img","disk2.img"]);
    assert!(DiskSet::numbered(&f.root.join("unrelated2.img")).unwrap().is_none());
    let mut s=Session::new(load(&[0xf4],"sample.com").unwrap());
    assert!(s.floppy_media(b'A',&f.root.join("disk2.img")).unwrap().contains("disk 1/2"));assert_eq!(marker(&s),b"one");
    s.disk(b'A',Some("next")).unwrap();assert_eq!(marker(&s),b"two");
}
