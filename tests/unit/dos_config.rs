use super::*;
#[test]
fn parses_drives_country_and_rejects_ambiguous_entries(){
    let c=parse("# drives\n#@program=images/FD14LITE.img::COMMAND.COM\n#@args=/K D:\\AUTOEXEC.BAT\nC=images/FD14LITE.img\nd: = drive D\ncountry=358\nheadless=true\nrust_graphics=true\nmouse=true\nfpu=80387\nshell=true\nengine=target/release/pil.exe\n",Path::new("base")).unwrap();
    assert_eq!(c.country,Some(358));
    assert!(c.text.mouse);
    assert_eq!(c.text.fpu.as_deref(),Some("80387"));
    assert_eq!(c.program,Some(format!("{}::COMMAND.COM",Path::new("base").join("images/FD14LITE.img").display())));
    assert_eq!(c.args.as_deref(),Some("/K D:\\AUTOEXEC.BAT"));
    assert_eq!(c.drives,vec![(b'C',PathBuf::from("base/images/FD14LITE.img")),(b'D',PathBuf::from("base/drive D"))]);
    for text in ["C=one\nc=two", "Z=ram", "country=999", "country=1\ncountry=358", "C=", "other=x", "headless=maybe", "fpu=8087", "#@program=a\n#@program=b", "#@args=a\n#@args=b"]{
        assert!(parse(text,Path::new("base")).is_err(),"{text}");
    }
}
#[test]
fn config_mounts_selects_c_and_updates_the_startup_environment(){
    use std::time::{SystemTime,UNIX_EPOCH};
    let root=std::env::temp_dir().join(format!("pil-config-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir(&root).unwrap();std::fs::create_dir(root.join("cdrive")).unwrap();std::fs::create_dir(root.join("ddrive")).unwrap();
    let path=root.join("pildos.conf");std::fs::write(&path,"C=cdrive\nD=ddrive\ncountry=358\n").unwrap();
    let mut p=dos::load(&dos::hello("com").unwrap(),"HELLO.COM").unwrap();
    assert_eq!(p.drive.letter(),b'A');
    apply(&mut p,path.to_str(),None).unwrap();
    assert_eq!(p.drive.letter(),b'C');assert_eq!(p.country,358);
    let env=p.ram.load(u32::from(p.psp)*16+0x2c,crate::MemWidth::Word).unwrap()*16;
    let bytes=p.ram.bytes();let start=env as usize;
    assert!(bytes[start..].starts_with(b"COMSPEC=C:\\COMMAND.COM\0"));
    assert!(bytes[start..start+100].windows(12).any(|v|v==b"C:\\HELLO.COM"));
    let mut z=dos::load_with_drives(&dos::hello("com").unwrap(),"HELLO.COM",vec![dos::Source::Memory(dos::MemDir::default())],None).unwrap();
    apply(&mut z,path.to_str(),None).unwrap();
    let ze=z.ram.load(u32::from(z.psp)*16+0x2c,crate::MemWidth::Word).unwrap()as usize*16;
    assert!(z.ram.bytes()[ze..ze+100].windows(12).any(|v|v==b"C:\\HELLO.COM"));
    assert!(z.ram.bytes()[ze..ze+100].windows(9).any(|v|v==b"TEMP=Z:\\\0"));
    let mut q=dos::load(&dos::hello("com").unwrap(),"HELLO.COM").unwrap();
    apply(&mut q,path.to_str(),Some(b'C')).unwrap();assert_eq!(q.drive.letter(),b'A');
    std::fs::remove_dir_all(root).unwrap();
}
#[cfg(windows)]
#[test]
fn wsl_paths_convert_for_windows_targets(){
    assert_eq!(host_path("/mnt/c/fixtures/drive D"),PathBuf::from(r"C:\fixtures\drive D"));
}
