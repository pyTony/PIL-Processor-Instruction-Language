use super::*;

fn keys(bytes:&[u8])->Vec<Event>{
    let mut parser=VtParser::default();let mut out=Vec::new();
    for &b in bytes{parser.feed(b,&mut out);}
    parser.flush(&mut out);
    out
}
fn words(bytes:&[u8])->Vec<u16>{
    keys(bytes).into_iter().filter_map(|e|match e{Event::Key(k,m)=>bios_word(k,m),_=>None}).collect()
}
const NONE:Mods=Mods{shift:false,ctrl:false,alt:false};
const CTRL:Mods=Mods{shift:false,ctrl:true,alt:false};
const ALT:Mods=Mods{shift:false,ctrl:false,alt:true};
const SHIFT:Mods=Mods{shift:true,ctrl:false,alt:false};

#[test]
fn bios_words_follow_the_at_bios_tables(){
    assert_eq!(bios_word(Key::Char('a'),NONE),Some(0x1e61));
    assert_eq!(bios_word(Key::Char('A'),SHIFT),Some(0x1e41));
    assert_eq!(bios_word(Key::Char('c'),CTRL),Some(0x2e03));
    assert_eq!(bios_word(Key::Char('x'),ALT),Some(0x2d00));
    assert_eq!(bios_word(Key::Char('1'),ALT),Some(0x7800));
    assert_eq!(bios_word(Key::Char('!'),SHIFT),Some(0x0221));
    assert_eq!(bios_word(Key::Char('|'),NONE),Some(0x2b7c));
    assert_eq!(bios_word(Key::Char(' '),NONE),Some(0x3920));
    assert_eq!(bios_word(Key::Enter,NONE),Some(0x1c0d));
    assert_eq!(bios_word(Key::Backspace,NONE),Some(0x0e08));
    assert_eq!(bios_word(Key::Escape,NONE),Some(0x011b));
    assert_eq!(bios_word(Key::Tab,SHIFT),Some(0x0f00));
    assert_eq!(bios_word(Key::Up,NONE),Some(0x4800));
    assert_eq!(bios_word(Key::Left,CTRL),Some(0x7300));
    assert_eq!(bios_word(Key::Delete,NONE),Some(0x5300));
    assert_eq!(bios_word(Key::F(1),NONE),Some(0x3b00));
    assert_eq!(bios_word(Key::F(9),CTRL),Some(0x6600));
    assert_eq!(bios_word(Key::F(10),SHIFT),Some(0x5d00));
    assert_eq!(bios_word(Key::F(2),ALT),Some(0x6900));
    assert_eq!(bios_word(Key::F(11),NONE),Some(0x8500));
    assert_eq!(bios_word(Key::F(12),ALT),Some(0x8c00));
    // Code page 437 beyond ASCII, with the Nordic key positions.
    assert_eq!(bios_word(Key::Char('ä'),NONE),Some(0x2884));
    assert_eq!(bios_word(Key::Char('Ö'),SHIFT),Some(0x2799));
    assert_eq!(bios_word(Key::Char('é'),NONE),Some(0x0082));
    assert_eq!(bios_word(Key::Char('€'),NONE),None);
}

#[test]
fn terminal_bytes_become_bios_keys(){
    assert_eq!(words(b"dir\r"),[0x2064,0x1769,0x1372,0x1c0d]);
    // Arrows (normal and application mode), the editing block, F-keys.
    assert_eq!(words(b"\x1b[A\x1bOB\x1b[C\x1b[D"),[0x4800,0x5000,0x4d00,0x4b00]);
    assert_eq!(words(b"\x1b[H\x1b[F\x1b[1~\x1b[4~\x1b[2~\x1b[3~\x1b[5~\x1b[6~"),[0x4700,0x4f00,0x4700,0x4f00,0x5200,0x5300,0x4900,0x5100]);
    assert_eq!(words(b"\x1bOP\x1b[15~\x1b[21~\x1b[24~"),[0x3b00,0x3f00,0x4400,0x8600]);
    // xterm modifiers: Ctrl+Left, Shift+F5.
    assert_eq!(words(b"\x1b[1;5D\x1b[15;2~"),[0x7300,0x5800]);
    // Backspace (DEL and BS), Tab, Ctrl+C, Alt+F (ESC f), Esc alone.
    assert_eq!(words(b"\x7f\x08\t\x03\x1bf"),[0x0e08,0x0e08,0x0f09,0x2e03,0x2100]);
    assert_eq!(words(b"\x1b"),[0x011b]);
    // UTF-8 from the host layout.
    assert_eq!(words("åö".as_bytes()),[0x1a86,0x2794]);
    // Ctrl+] ends the console.
    assert_eq!(keys(b"a\x1d"),[Event::Key(Key::Char('a'),NONE),Event::Quit]);
}

#[test]
fn an_unfinished_sequence_waits_for_its_end(){
    let mut parser=VtParser::default();let mut out=Vec::new();
    for &b in b"\x1b[1;"{parser.feed(b,&mut out);}
    assert!(out.is_empty()&&parser.waiting());
    for &b in b"5A"{parser.feed(b,&mut out);}
    assert_eq!(out,[Event::Key(Key::Up,CTRL)]);
    assert!(!parser.waiting());
}

#[test]
fn ansi_frame_redraws_only_changed_rows(){
    let mut cells=vec![0u8;80*25*2];
    for c in cells.chunks_mut(2){c[0]=b' ';c[1]=0x07;}
    let first=ansi_frame(&cells,80,(0,0),true,None);
    assert_eq!(first.matches("\x1b[").filter(|_|true).count()>25,true);
    assert!(first.contains("\x1b[25;1H"));
    cells[2*80*3]=b'C';cells[2*80*3+1]=0x1e;cells[2*80*3+2]=0xc9;
    let next=ansi_frame(&cells,80,(3,1),false,Some(&first_cells()));
    assert!(next.starts_with("\x1b[4;1H\x1b[0;93;44mC\x1b[0;37;40m╔"),"{next:?}");
    assert!(!next.contains("\x1b[1;1H"));
    assert!(next.ends_with("\x1b[0m\x1b[4;2H\x1b[?25l"));
}
fn first_cells()->Vec<u8>{let mut c=vec![0u8;80*25*2];for x in c.chunks_mut(2){x[0]=b' ';x[1]=0x07;}c}

#[test]
fn text_pixels_draw_the_rom_font_and_the_cursor(){
    let font=super::super::console::graphics_text::font().unwrap();
    let mut cells=vec![0u8;80*25*2];
    cells[0]=b'A';cells[1]=0x1f;
    let (w,h,pixels)=text_pixels(&cells,80,&font,Some((0,1,14,15)));
    assert_eq!((w,h),(640,400));
    // Font line y of 'A' becomes pixel rows 2y and 2y+1.
    for y in 0..16{for x in 0..8{
        let lit=font[usize::from(b'A')*8+y/2]&(0x80>>x)!=0;
        assert_eq!(pixels[y*640+x],if lit{0xffffff}else{0x0000aa});
    }}
    // The cursor in cell (0,1): lines 14-15 in the cell's foreground (black on black here).
    assert!(pixels[14*640+8..14*640+16].iter().all(|p|*p==0));
}

/// FreeCOM-style editing works because keys arrive as BIOS words: a
/// program reading with INT 16h sees Up as 4800h, not COM1 bytes.
#[test]
fn console_keys_reach_int16_as_bios_words(){
    // MOV AH,0; INT 16h; MOV [0200h],AX; MOV AH,0; INT 16h; MOV [0202h],AX; MOV AX,4C00h; INT 21h
    let code=[0xb4,0x00,0xcd,0x16,0xa3,0x00,0x02,0xb4,0x00,0xcd,0x16,0xa3,0x02,0x02,0xb8,0x00,0x4c,0xcd,0x21];
    let loaded=crate::dos::load(&code,"KEYS.COM").unwrap();
    let mut session=Session::new(loaded);
    session.run(1000).unwrap();
    assert_eq!(session.status().0,"waiting_input");
    for (k,m) in [(Key::Up,NONE),(Key::F(3),NONE)]{
        let w=bios_word(k,m).unwrap();assert!(session.key(w as u8,(w>>8) as u8).unwrap());
    }
    session.run(1000).unwrap();
    assert_eq!(session.status().0,"exit");
    let p=session.machine().unwrap();
    let base=u32::from(p.selectors[3])*16;
    assert_eq!(p.ram.load(base+0x200,MemWidth::Word).unwrap(),0x4800);
    assert_eq!(p.ram.load(base+0x202,MemWidth::Word).unwrap(),0x3d00);
}

/// Issue #177: graphics a child program sets after EXEC from a parent
/// (a shell) reach the console's frame, and the parent resumes in text mode.
#[test]
fn a_child_started_by_exec_shows_its_graphics_frame(){
    // Parent: shrink to 1 KiB, EXEC CHILD.COM, exit. Child: mode 12h, wait
    // for a key, mode 3, exit. (Original fixtures, assembled with NASM.)
    let parent=[0xbc,0x00,0x04,0xb4,0x4a,0xbb,0x40,0x00,0xcd,0x21,0x8c,0x0e,0x2c,0x01,0xba,0x1e,0x01,0xbb,0x28,0x01,0xb8,0x00,0x4b,0xcd,0x21,0xb8,0x00,0x4c,0xcd,0x21,
        0x43,0x48,0x49,0x4c,0x44,0x2e,0x43,0x4f,0x4d,0x00,0x00,0x00,0x80,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00];
    let child=[0xb8,0x12,0x00,0xcd,0x10,0xb4,0x00,0xcd,0x16,0xb8,0x03,0x00,0xcd,0x10,0xb8,0x00,0x4c,0xcd,0x21];
    let unique=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let dir=std::env::temp_dir().join(format!("pil-console-exec-{}-{unique}",std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("CHILD.COM"),child).unwrap();
    let loaded=crate::dos::load_with_drives(&parent,"PARENT.COM",vec![crate::dos::Source::Host(dir.clone())],None).unwrap();
    let mut session=Session::new(loaded);
    assert!(!graphics_mode(session.machine().unwrap()));
    for _ in 0..50{session.run(20_000).unwrap();if session.status().0!="running"{break;}}
    assert_eq!(session.status().0,"waiting_input","{:?}",session.status());
    let p=session.machine().unwrap();
    assert!(graphics_mode(p));
    let (w,h,_)=crate::dos::desktop_vga_frame(p).expect("mode 12h frame");
    assert_eq!((w,h),(640,480));
    let esc=bios_word(Key::Escape,NONE).unwrap();
    assert!(session.key(esc as u8,(esc>>8) as u8).unwrap());
    for _ in 0..50{session.run(20_000).unwrap();if session.status().0!="running"{break;}}
    assert_eq!(session.status(),("exit","exit code 0".into()));
    assert!(!graphics_mode(session.machine().unwrap()));
    std::fs::remove_dir_all(dir).unwrap();
}
