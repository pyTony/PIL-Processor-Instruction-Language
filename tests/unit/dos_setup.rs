use super::*;

const NONE:Mods=Mods{shift:false,ctrl:false,alt:false};
const EXAMPLE:&str="# Copy to pildos.conf and edit paths.\r\n#@program=images/FD.img::SHELL.EXE\r\nargs=/K D:\\START.BAT\r\nC=images/FD.img\r\nd: = driveD\r\ncountry=358\r\n";

fn setup(text:Option<&str>)->Setup{Setup::new(PathBuf::from("base/pildos.conf"),text,None)}
fn row_text(cells:&[u8],row:usize)->String{
    crate::dos_encoding::decode(&cells[row*COLUMNS*2..(row+1)*COLUMNS*2].chunks(2).map(|c|c[0]).collect::<Vec<_>>())
}
fn select(s:&mut Setup,key:&str){
    s.key(Key::Home,NONE);
    while FIELDS[s.selected].key!=key{s.key(Key::Down,NONE);}
}
fn typed(s:&mut Setup,text:&str){for c in text.chars(){s.key(Key::Char(c),NONE);}}

#[test]
fn every_documented_setting_has_a_field_and_the_screen_fits(){
    let keys:Vec<_>=FIELDS.iter().map(|f|f.key).collect();
    for key in ["program","args","a","b","c","d","e","country","fpu","mouse","ansi","utc_offset","com1","speaker_volume","headless","shell","rust_graphics","engine"]{
        assert!(keys.contains(&key),"{key}");
    }
    assert!(FIRST_FIELD+FIELDS.len()<ROWS-3);
    let s=setup(Some(EXAMPLE));
    let (cells,cursor)=s.render();
    assert_eq!(cells.len(),COLUMNS*ROWS*2);
    assert_eq!(cursor,None);
    assert!(row_text(&cells,0).contains("PILdos setup"));
    assert!(row_text(&cells,FIRST_FIELD).contains("images/FD.img::SHELL.EXE"));
    assert!(row_text(&cells,FIRST_FIELD+5).contains("driveD"));
    assert!(row_text(&cells,FIRST_FIELD+7).contains("358  Finland"));
    assert!(row_text(&cells,FIRST_FIELD+11).contains("this computer's time zone"));
    assert!(row_text(&cells,ROWS-1).contains("F10"));
}

#[test]
fn edits_keep_the_file_as_written(){
    let mut s=setup(Some(EXAMPLE));
    assert!(!s.dirty());
    // Edit D: (written "d: = driveD"): only its value changes.
    select(&mut s,"d");
    s.key(Key::Enter,NONE);
    for _ in 0.."driveD".len(){s.key(Key::Backspace,NONE);}
    typed(&mut s,"work files");
    let (_,cursor)=s.render();
    assert_eq!(cursor,Some(((FIRST_FIELD+5) as u32,(VALUE_COLUMN+10) as u32)));
    s.key(Key::Enter,NONE);
    // Country cycles 358 -> 1 on its own line.
    select(&mut s,"country");
    s.key(Key::Right,NONE);
    // A new boolean and an offset go at the end.
    select(&mut s,"mouse");s.key(Key::Enter,NONE);
    select(&mut s,"utc_offset");typed(&mut s,"+02:00");s.key(Key::Enter,NONE);
    // The program written with #@ keeps its prefix.
    select(&mut s,"program");s.key(Key::Enter,NONE);s.key(Key::End,NONE);
    typed(&mut s,"X");s.key(Key::Enter,NONE);
    assert!(s.dirty());
    assert_eq!(s.doc.text(),"# Copy to pildos.conf and edit paths.\r\n#@program=images/FD.img::SHELL.EXEX\r\nargs=/K D:\\START.BAT\r\nC=images/FD.img\r\nd: = work files\r\ncountry=1\r\nmouse=true\r\nutc_offset=+02:00\r\n");
    // Clearing removes the line; booleans toggle.
    select(&mut s,"args");s.key(Key::Delete,NONE);
    select(&mut s,"mouse");s.key(Key::Char(' '),NONE);
    assert_eq!(s.doc.get("args"),None);
    assert_eq!(s.doc.get("mouse").as_deref(),Some("false"));
}

#[test]
fn bad_values_are_refused_before_they_reach_the_file(){
    let mut s=setup(Some(EXAMPLE));
    select(&mut s,"utc_offset");typed(&mut s,"7200");s.key(Key::Enter,NONE);
    assert!(s.editing.is_some());assert!(s.status().contains("UTC offset"));
    s.key(Key::Escape,NONE);
    assert!(s.editing.is_none());assert_eq!(s.doc.get("utc_offset"),None);
    select(&mut s,"speaker_volume");typed(&mut s,"101");s.key(Key::Enter,NONE);
    assert!(s.editing.is_some());
    s.key(Key::Backspace,NONE);s.key(Key::Enter,NONE);
    assert_eq!(s.doc.get("speaker_volume").as_deref(),Some("10"));
    select(&mut s,"args");s.key(Key::Enter,NONE);typed(&mut s,"\u{e4}");s.key(Key::Enter,NONE);
    assert!(s.status().contains("ASCII"));
    s.key(Key::Escape,NONE);
    select(&mut s,"com1");typed(&mut s,"tcp:0");s.key(Key::Enter,NONE);
    assert!(s.editing.is_some());
}

#[test]
fn start_needs_a_program_and_esc_warns_about_unsaved_changes(){
    let mut s=setup(None);
    assert!(s.dirty());
    assert_eq!(s.key(Key::F(10),NONE),Action::Stay);
    assert!(s.status().contains("program"));
    assert_eq!(s.key(Key::Escape,NONE),Action::Stay);
    assert_eq!(s.key(Key::Escape,NONE),Action::Quit);
    let mut clean=setup(Some(EXAMPLE));
    assert_eq!(clean.key(Key::Escape,NONE),Action::Quit);
}

#[test]
fn save_writes_only_text_the_parser_accepts(){
    use std::time::{SystemTime,UNIX_EPOCH};
    let root=std::env::temp_dir().join(format!("pil-setup-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("pildos.conf.example"),"# example\nprogram=HELLO.COM\n").unwrap();
    let path=root.join("pildos.conf");
    let mut s=Setup::load(path.clone()).unwrap();
    assert!(s.dirty());assert!(s.status().contains("example"));
    assert_eq!(s.key(Key::F(2),NONE),Action::Stay);
    assert_eq!(std::fs::read_to_string(&path).unwrap(),"# example\nprogram=HELLO.COM\n");
    assert!(!s.dirty());
    // A duplicate in the file is reported and not written back.
    std::fs::write(&path,"\u{feff}program=A.COM\ncountry=1\ncountry=358\n").unwrap();
    let mut bad=Setup::load(path.clone()).unwrap();
    assert!(bad.status().contains("duplicate"));
    select(&mut bad,"mouse");bad.key(Key::Enter,NONE);
    bad.key(Key::F(2),NONE);
    assert!(bad.status().contains("duplicate"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(),"\u{feff}program=A.COM\ncountry=1\ncountry=358\n");
    // F10 with a valid change saves first; the BOM stays.
    std::fs::write(&path,"\u{feff}program=A.COM\n").unwrap();
    let mut good=Setup::load(path.clone()).unwrap();
    select(&mut good,"ansi");good.key(Key::Enter,NONE);
    assert_eq!(good.key(Key::F(10),NONE),Action::Start);
    assert_eq!(std::fs::read_to_string(&path).unwrap(),"\u{feff}program=A.COM\nansi=true\n");
    std::fs::remove_dir_all(root).unwrap();
}
