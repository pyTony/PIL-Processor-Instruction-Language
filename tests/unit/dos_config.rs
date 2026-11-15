use super::*;
#[test]
fn disk_set_config_paths_and_drive_conflicts(){
    let c=parse("diskset_a=media/install.lst\ndiskset_B=other.lst\nD=programs",Path::new("base")).unwrap();
    assert_eq!(c.disk_sets,vec![(b'A',PathBuf::from("base/media/install.lst")),(b'B',PathBuf::from("base/other.lst"))]);
    for text in ["diskset_c=set.lst","diskset_a=","diskset_a=x\ndiskset_A=y","A=disk.img\ndiskset_a=x","diskset_b=x\nB=disk.img"]{
        assert!(parse(text,Path::new("base")).is_err(),"{text}");
    }
}
#[test]
fn parses_drives_country_and_rejects_ambiguous_entries(){
    let c=parse("# drives\n#@program=images/FD14LITE.img::COMMAND.COM\n#@args=/K D:\\AUTOEXEC.BAT\nC=images/FD14LITE.img\nd: = drive D\ncountry=358\nheadless=true\nrust_graphics=true\nmouse=true\nfpu=80387\nshell=true\nengine=target/release/pil.exe\n",Path::new("base")).unwrap();
    assert_eq!(c.country,Some(358));
    assert_eq!(c.mouse,Some(true));
    assert_eq!(c.fpu,Some(true));
    assert_eq!(c.program,Some(format!("{}::COMMAND.COM",Path::new("base").join("images/FD14LITE.img").display())));
    assert_eq!(c.args.as_deref(),Some("/K D:\\AUTOEXEC.BAT"));
    assert_eq!(c.drives,vec![(b'C',PathBuf::from("base/images/FD14LITE.img")),(b'D',PathBuf::from("base/drive D"))]);
    for text in ["C=one\nc=two", "Z=ram", "country=999", "country=1\ncountry=358", "C=", "other=x", "headless=maybe", "fpu=8087", "#@program=a\n#@program=b", "#@args=a\n#@args=b", "own_programs=Z", "own_programs=DD", "own_programs=D\nown_programs=none"]{
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
#[test]
fn starts_update_the_own_programs_drive_from_programs_dos(){
    use std::time::{SystemTime,UNIX_EPOCH};
    let root=std::env::temp_dir().join(format!("pil-own-config-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    let source=root.join("programs").join("dos");
    std::fs::create_dir_all(&source).unwrap();std::fs::create_dir(root.join("ddrive")).unwrap();std::fs::create_dir(root.join("edrive")).unwrap();
    std::fs::write(source.join("DEMO.COM"),b"demo").unwrap();
    assert_eq!(parse("own_programs=e:",&root).unwrap().own_programs,Some(Some(b'E')));
    assert_eq!(parse("own-programs=None",&root).unwrap().own_programs,Some(None));
    let path=root.join("pildos.conf");
    // Off: nothing is copied.
    std::fs::write(&path,"D=ddrive\nown_programs=none\n").unwrap();
    assert_eq!(refresh_own_programs(path.to_str()).unwrap(),None);
    assert!(!root.join("ddrive/DEMO.COM").exists());
    // Another drive, then the default D: as a session start does it.
    std::fs::write(&path,"D=ddrive\nE=edrive\nown_programs=E\n").unwrap();
    assert_eq!(refresh_own_programs(path.to_str()).unwrap().as_deref(),Some("E: from programs/dos: added DEMO.COM."));
    assert!(!root.join("ddrive/DEMO.COM").exists());
    std::fs::write(&path,"D=ddrive\n").unwrap();
    let mut p=dos::load(&dos::hello("com").unwrap(),"HELLO.COM").unwrap();
    apply(&mut p,path.to_str(),None).unwrap();
    assert_eq!(std::fs::read(root.join("ddrive/DEMO.COM")).unwrap(),b"demo");
    // A legacy --mount for D: replaces the configured folder: left alone.
    std::fs::remove_file(root.join("ddrive/DEMO.COM")).unwrap();
    let mut q=dos::load(&dos::hello("com").unwrap(),"HELLO.COM").unwrap();
    apply(&mut q,path.to_str(),Some(b'D')).unwrap();
    assert!(!root.join("ddrive/DEMO.COM").exists());
    std::fs::remove_dir_all(root).unwrap();
}
#[cfg(windows)]
#[test]
fn wsl_paths_convert_for_windows_targets(){
    assert_eq!(host_path("/mnt/c/fixtures/drive D"),PathBuf::from(r"C:\fixtures\drive D"));
}
#[test]
fn console_settings_and_offsets_follow_the_frontend_rules(){
    let c=parse("program=A.COM\nfpu=80387\nmouse=1\nansi=false\nutc-offset=5.5\nspeaker_volume=100\n",Path::new("base")).unwrap();
    assert_eq!((c.fpu,c.mouse,c.ansi,c.utc_offset),(Some(true),Some(true),Some(false),Some(19800)));
    assert_eq!(c.speaker_volume,Some(100));
    assert_eq!(parse("speaker-volume=0\nspeaker_volume=7\n",Path::new("base")).unwrap().speaker_volume,Some(7));
    assert_eq!(parse("mouse=1\n",Path::new("base")).unwrap().speaker_volume,None);
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
#[test]
fn host_pointer_setting_parses_and_decides_when_to_hide(){
    assert_eq!(parse("mouse=1\n",Path::new("base")).unwrap().host_pointer,None);
    for (text,want) in [("auto",HostPointer::Auto),("Show",HostPointer::Show),(" HIDE ",HostPointer::Hide)]{
        assert_eq!(parse(&format!("host_pointer={text}\n"),Path::new("base")).unwrap().host_pointer,Some(want),"{text}");
    }
    assert_eq!(parse("host-pointer=hide\n",Path::new("base")).unwrap().host_pointer,Some(HostPointer::Hide));
    for text in ["host_pointer=","host_pointer=off","host_pointer=auto\nhost_pointer=show"]{
        assert!(parse(text,Path::new("base")).is_err(),"{text}");
    }
    assert_eq!(HostPointer::default(),HostPointer::Auto);
    // Auto follows the INT 33h cursor; show and hide ignore it.
    assert_eq!([false,true].map(|v|HostPointer::Auto.hides(v)),[false,true]);
    assert_eq!([false,true].map(|v|HostPointer::Show.hides(v)),[false,false]);
    assert_eq!([false,true].map(|v|HostPointer::Hide.hides(v)),[true,true]);
}
