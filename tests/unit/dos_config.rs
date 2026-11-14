use super::*;
#[test]
fn parses_drives_country_and_rejects_ambiguous_entries(){
    let c=parse("# drives\n#@program=images/FD14LITE.img::COMMAND.COM\n#@args=/K D:\\AUTOEXEC.BAT\nC=images/FD14LITE.img\nd: = drive D\ncountry=358\nheadless=true\nrust_graphics=true\nmouse=true\nfpu=80387\nshell=true\nengine=target/release/pil.exe\n",Path::new("base")).unwrap();
    assert_eq!(c.country,Some(358));
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
    assert_eq!(host_path("/mnt/c/Users/admin/drive D"),PathBuf::from(r"C:\Users\admin\drive D"));
}
#[test]
fn console_settings_and_offsets_follow_the_frontend_rules(){
    let c=parse("program=A.COM\nfpu=80387\nmouse=1\nansi=false\nutc-offset=5.5\nspeaker_volume=100\n",Path::new("base")).unwrap();
    assert_eq!((c.fpu,c.mouse,c.ansi,c.utc_offset),(Some(true),Some(true),Some(false),Some(19800)));
    for (text,seconds) in [("2",7200),("-5",-18000),("+02:00",7200),("-3:30",-12600),("14",50400),("-12",-43200),("0",0)]{
        assert_eq!(parse_offset(text),Ok(seconds),"{text}");
    }
    for text in ["","7200","15","-12:30","2:60","x","1e3","+"]{assert!(parse_offset(text).is_err(),"{text}");}
    for text in ["mouse=1\nmouse=0","utc_offset=99","speaker_volume=101","speaker-volume=x","fpu=none\nfpu=80387"]{
        assert!(parse(text,Path::new("base")).is_err(),"{text}");
    }
}
#[test]
fn the_editor_keeps_lines_spelling_and_line_ends(){
    use edit::Document;
    let text="\u{feff}; profile\r\n  #@Program = a.com\r\nD:=drive\r\n\r\nutc-offset=2";
    let mut d=Document::parse(text);
    assert_eq!(d.text(),text);
    assert_eq!(d.get("program").as_deref(),Some("a.com"));
    assert_eq!(d.get("d").as_deref(),Some("drive"));
    assert_eq!(d.get("utc_offset").as_deref(),Some("2"));
    d.set("program",Some("b.exe"));d.set("utc_offset",None);d.set("e",Some("e dir"));d.set("country",Some("358"));
    assert_eq!(d.text(),"\u{feff}; profile\r\n  #@Program = b.exe\r\nD:=drive\r\n\r\nE=e dir\r\ncountry=358\r\n");
    assert!(d.check(Path::new("base")).is_ok());
    assert_eq!(Document::parse("").text(),"");
    let mut empty=Document::parse("");empty.set("program",Some("x.com"));
    assert_eq!(empty.text(),"program=x.com\n");
}
