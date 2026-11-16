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
    let source=root.join("programs");let drive=root.join("drive");let target=drive.join(FOLDER);
    std::fs::create_dir_all(&source).unwrap();std::fs::create_dir_all(&target).unwrap();
    std::fs::write(source.join("NEW.COM"),b"new program").unwrap();
    std::fs::write(source.join("OLD.COM"),b"version 2").unwrap();
    std::fs::write(source.join("MINE.COM"),b"ours").unwrap();
    std::fs::write(source.join("SAME.BAS"),b"same").unwrap();
    std::fs::write(source.join("prog.asm"),b"; version 2").unwrap();
    std::fs::write(source.join("NOTES.TXT"),b"notes").unwrap();
    let manifest=format!("# comment\n{}  OLD.COM\n{}  OLD.COM\n{}  PROG.ASM\n",
        hex(&sha256(b"version 1")),hex(&sha256(b"version 2")),hex(&sha256(b"; version 1")));
    std::fs::write(source.join(MANIFEST),manifest).unwrap();
    // In the folder: an earlier version, the user's own file under one of
    // our names, an identical copy and a file of the user's that is not ours.
    std::fs::write(target.join("OLD.COM"),b"version 1").unwrap();
    std::fs::write(target.join("MINE.COM"),b"the user's own").unwrap();
    std::fs::write(target.join("SAME.BAS"),b"same").unwrap();
    std::fs::write(target.join("OTHER.EXE"),b"third party").unwrap();
    // In the drive's root, as earlier starts left it: our old copies (one in
    // other letter case, one a source) and the user's own files.
    std::fs::write(drive.join("old.com"),b"version 1").unwrap();
    std::fs::write(drive.join("PROG.ASM"),b"; version 1").unwrap();
    std::fs::write(drive.join("MINE.COM"),b"the user's own").unwrap();
    std::fs::write(drive.join("GAME.EXE"),b"third party").unwrap();

    let report=refresh(&source,&drive);
    assert_eq!(report,Report{added:vec!["NEW.COM".into()],updated:vec!["OLD.COM".into()],kept:vec!["MINE.COM".into()],
        failed:vec![],removed:vec!["OLD.COM".into(),"prog.asm".into()]});
    assert_eq!(std::fs::read(target.join("NEW.COM")).unwrap(),b"new program");
    assert_eq!(std::fs::read(target.join("OLD.COM")).unwrap(),b"version 2");
    assert_eq!(std::fs::read(target.join("MINE.COM")).unwrap(),b"the user's own");
    assert_eq!(std::fs::read(target.join("OTHER.EXE")).unwrap(),b"third party");
    let list=|d:&Path|{let mut n:Vec<_>=std::fs::read_dir(d).unwrap().map(|e|e.unwrap().file_name().into_string().unwrap()).collect();n.sort();n};
    // Programs only: no sources, notes or the version list.
    assert_eq!(list(&target),["MINE.COM","NEW.COM","OLD.COM","OTHER.EXE","SAME.BAS"]);
    assert_eq!(list(&drive),["GAME.EXE","MINE.COM",FOLDER]);
    assert_eq!(report.summary(b'D').unwrap(),"D:\\PIL from programs/dos: updated OLD.COM; added NEW.COM; kept your own MINE.COM; removed the old copies in D:\\ of OLD.COM, prog.asm.");

    // A second start changes nothing and says nothing new but the kept file.
    let again=refresh(&source,&drive);
    assert_eq!(again,Report{kept:vec!["MINE.COM".into()],..Report::default()});
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn refresh_creates_the_folder_or_uses_one_in_other_case(){
    let root=folder("create");
    let source=root.join("programs");let drive=root.join("drive");
    std::fs::create_dir_all(&source).unwrap();std::fs::create_dir_all(&drive).unwrap();
    std::fs::write(source.join("A.COM"),b"a").unwrap();
    assert_eq!(refresh(&source,&drive).added,["A.COM"]);
    assert_eq!(std::fs::read(drive.join(FOLDER).join("A.COM")).unwrap(),b"a");
    let other=root.join("other");std::fs::create_dir_all(other.join("pil")).unwrap();
    assert_eq!(refresh(&source,&other).added,["A.COM"]);
    assert!(other.join("pil").join("A.COM").is_file());
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
