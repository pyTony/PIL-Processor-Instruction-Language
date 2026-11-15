use super::*;
use std::time::{SystemTime,UNIX_EPOCH};

fn folder(tag:&str)->PathBuf{
    let root=std::env::temp_dir().join(format!("pil-own-{tag}-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn sha256_matches_the_fips_examples(){
    assert_eq!(hex(&sha256(b"")),"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    assert_eq!(hex(&sha256(b"abc")),"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert_eq!(hex(&sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1");
    // 1,000,000 bytes: several blocks and a length past 32 bits of bits.
    assert_eq!(hex(&sha256(&vec![b'a';1_000_000])),"cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0");
}

#[test]
fn refresh_adds_updates_old_versions_and_keeps_other_files(){
    let root=folder("refresh");
    let source=root.join("programs");let target=root.join("drive");
    std::fs::create_dir_all(&source).unwrap();std::fs::create_dir_all(&target).unwrap();
    std::fs::write(source.join("NEW.COM"),b"new program").unwrap();
    std::fs::write(source.join("OLD.COM"),b"version 2").unwrap();
    std::fs::write(source.join("MINE.COM"),b"ours").unwrap();
    std::fs::write(source.join("SAME.TXT"),b"same").unwrap();
    std::fs::write(source.join("prog.asm"),b"; version 2").unwrap();
    let manifest=format!("# comment\n{}  OLD.COM\n{}  OLD.COM\n{}  PROG.ASM\n",
        hex(&sha256(b"version 1")),hex(&sha256(b"version 2")),hex(&sha256(b"; version 1")));
    std::fs::write(source.join(MANIFEST),manifest).unwrap();
    // On the drive: an earlier version, the user's own file under one of our
    // names, an identical copy, an old version in other letter case, and a
    // file of the user's that is not ours at all.
    std::fs::write(target.join("OLD.COM"),b"version 1").unwrap();
    std::fs::write(target.join("MINE.COM"),b"the user's own").unwrap();
    std::fs::write(target.join("SAME.TXT"),b"same").unwrap();
    std::fs::write(target.join("PROG.ASM"),b"; version 1").unwrap();
    std::fs::write(target.join("OTHER.EXE"),b"third party").unwrap();

    let report=refresh(&source,&target);
    assert_eq!(report,Report{added:vec!["NEW.COM".into()],updated:vec!["OLD.COM".into(),"prog.asm".into()],kept:vec!["MINE.COM".into()],failed:vec![]});
    assert_eq!(std::fs::read(target.join("NEW.COM")).unwrap(),b"new program");
    assert_eq!(std::fs::read(target.join("OLD.COM")).unwrap(),b"version 2");
    assert_eq!(std::fs::read(target.join("PROG.ASM")).unwrap(),b"; version 2");
    assert_eq!(std::fs::read(target.join("MINE.COM")).unwrap(),b"the user's own");
    assert_eq!(std::fs::read(target.join("OTHER.EXE")).unwrap(),b"third party");
    assert!(!target.join(MANIFEST).exists());
    let mut names:Vec<_>=std::fs::read_dir(&target).unwrap().map(|e|e.unwrap().file_name().into_string().unwrap()).collect();
    names.sort();
    assert_eq!(names,["MINE.COM","NEW.COM","OLD.COM","OTHER.EXE","PROG.ASM","SAME.TXT"]);
    assert_eq!(report.summary(b'D').unwrap(),"D: from programs/dos: updated OLD.COM, prog.asm; added NEW.COM; kept your own MINE.COM.");

    // A second start changes nothing and says nothing new but the kept file.
    let again=refresh(&source,&target);
    assert_eq!(again,Report{kept:vec!["MINE.COM".into()],..Report::default()});
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn refresh_without_either_folder_does_nothing(){
    let root=folder("missing");
    assert_eq!(refresh(&root.join("none"),&root),Report::default());
    assert_eq!(refresh(&root,&root.join("none")),Report::default());
    assert_eq!(Report::default().summary(b'D'),None);
    std::fs::remove_dir_all(&root).unwrap();
}
