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

#[cfg(windows)]
#[test]
fn windows_control_characters_become_native_dos_keys(){
    use crate::dine_window::Input;
    for (ch,key,bios) in [
        ('\r',Key::Enter,0x1c0d),
        ('\n',Key::Enter,0x1c0d),
        ('\t',Key::Tab,0x0f09),
        ('\u{1b}',Key::Escape,0x011b),
        ('\u{8}',Key::Backspace,0x0e08),
    ] {
        let mapped=window_event(Input::Char{ch,shift:false,ctrl:false,alt:false});
        assert_eq!(mapped,Some(Event::Key(key,NONE)),"char {ch:?}");
        if let Some(Event::Key(mapped_key,mods))=mapped {
            assert_eq!(bios_word(mapped_key,mods),Some(bios),"char {ch:?}");
        }
    }
    // Preserve ordinary typed characters and function-key routing.
    assert_eq!(window_event(Input::Char{ch:'S',shift:true,ctrl:false,alt:false}),
        Some(Event::Key(Key::Char('S'),SHIFT)));
    assert_eq!(window_event(Input::Key{vk:0x79,shift:false,ctrl:false,alt:false}),
        Some(Event::Key(Key::F(10),NONE)));
}

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
fn text_pixels_draw_vga_cells_with_the_8x16_font_and_the_cursor(){
    let font=super::super::vga_text::font().unwrap();
    assert_eq!(font.len(),256*16);
    let mut cells=first_cells();
    cells[0]=b'A';cells[1]=0x1f;
    cells[4]=0xc4;cells[5]=0x07;   // a horizontal line (C0h-DFh) in column 2
    cells[6]=0xb3;cells[7]=0x07;   // a vertical line (B3h) in column 3
    let (w,h,pixels)=text_pixels(&cells,80,&font,Some((0,1,13,14)));
    assert_eq!((w,h),(720,400));
    // Each font line is one pixel row; the ninth column is the background.
    for y in 0..16{
        for x in 0..8{
            let lit=font[usize::from(b'A')*16+y]&(0x80>>x)!=0;
            assert_eq!(pixels[y*720+x],if lit{0xffffff}else{0x0000aa},"A row {y} column {x}");
        }
        assert_eq!(pixels[y*720+8],0x0000aa);
    }
    // Lines C0h-DFh repeat their eighth column in the ninth; B3h does not.
    let row=(0..16).find(|&y|font[0xc4*16+y]!=0).unwrap();
    assert!(pixels[row*720+18..row*720+27].iter().all(|p|*p==0xaaaaaa));
    assert!((0..16).all(|y|pixels[y*720+27+8]==0));
    // The cursor in cell (0,1): lines 13-14 in the cell's foreground, 15 not.
    assert!(pixels[13*720+9..13*720+18].iter().all(|p|*p==0xaaaaaa));
    assert!(pixels[14*720+9..14*720+18].iter().all(|p|*p==0xaaaaaa));
    assert!(pixels[15*720+9..15*720+18].iter().all(|p|*p==0));
    // 40 columns are drawn 360 wide and doubled to 720.
    let (w,h,wide)=text_pixels(&cells[..40*25*2],40,&font,None);
    assert_eq!((w,h),(720,400));
    assert_eq!(wide[0..2],[wide[0],wide[0]]);
}

#[test]
fn bios_cursor_shapes_map_to_vga_cell_lines(){
    use super::super::vga_text::cursor_lines;
    assert_eq!(cursor_lines(6,7),Some((13,14)));     // the BIOS default underline
    assert_eq!(cursor_lines(0,7),Some((0,14)));      // a CGA block
    assert_eq!(cursor_lines(13,14),Some((13,14)));   // a VGA shape stays
    assert_eq!(cursor_lines(0x20,7),None);           // bit 5: hidden
    assert_eq!(cursor_lines(7,6),None);              // start after end: hidden
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

/// The window shows the picture at a whole-number size, centred, unless
/// that leaves most of the window empty.
#[test]
fn window_view_uses_whole_number_scaling(){
    use crate::dine_window::{view,initial_scale};
    assert_eq!(view(1440,800,720,400),(0,0,1440,800));
    assert_eq!(view(1500,900,720,400),(30,50,1440,800));
    assert_eq!(view(1439,800,720,400),(0,0,1439,799));        // one pixel short of 2x: fitted
    assert_eq!(view(1440,800,640,480),(187,0,1066,800));      // 1x would use 60%: fitted
    assert_eq!(view(1500,1000,640,480),(110,20,1280,960));    // 2x fits
    assert_eq!(view(1500,900,640,480),(150,0,1200,900));      // 1x 53%: fitted
    assert_eq!(view(1300,850,1200,800),(50,25,1200,800));     // 1x uses 94%: kept
    assert_eq!(view(360,300,720,400),(0,50,360,200));       // smaller: shrunk, proportions kept
    assert_eq!(view(0,0,720,400),(0,0,0,0));
    // 1920x1080 desktop less the window frame and taskbar.
    assert_eq!(initial_scale(None,1904,1001,720,400),2);
    assert_eq!(initial_scale(None,1904,1001,640,480),2);
    assert_eq!(initial_scale(None,1904,1001,320,200),5);
    assert_eq!(initial_scale(None,700,380,720,400),1);
    assert_eq!(initial_scale(Some("3"),1904,1001,720,400),3);
    assert_eq!(initial_scale(Some("0"),1904,1001,720,400),2);
}

#[test]
fn window_selection_follows_the_pointer_in_reading_order(){
    use select::*;
    // A 720x400 picture of 80x25 cells; the pointer clamps to the screen.
    assert_eq!(cell_at(0,0,720,400,80,25),(0,0));
    assert_eq!(cell_at(719,399,720,400,80,25),(24,79));
    assert_eq!(cell_at(-5,9999,720,400,80,25),(24,0));
    assert_eq!(cell_at(9*10+4,16*3+15,720,400,80,25),(3,10));
    // Press, drag backwards, release: the cells between, in reading order.
    let mut s=Selecting::default();
    assert_eq!(s.pointer((2,5),1),Gesture::None);assert!(s.dragging());
    s.pointer((1,70),1);s.pointer((1,70),0);assert!(!s.dragging());
    let sel=s.selection.unwrap();
    assert!(sel.contains(1,70)&&sel.contains(1,79)&&sel.contains(2,0)&&sel.contains(2,5));
    assert!(!sel.contains(1,69)&&!sel.contains(2,6));
    // A right press copies a selection; without one it pastes.
    assert_eq!(s.pointer((0,0),2),Gesture::Copy);s.pointer((0,0),0);
    s.clear();assert_eq!(s.pointer((0,0),2),Gesture::Paste);s.pointer((0,0),0);
    // A click that does not move clears the selection.
    s.pointer((3,3),1);s.pointer((3,3),0);assert_eq!(s.selection,None);
}

#[test]
fn copied_text_trims_lines_and_pasted_text_becomes_keys(){
    use select::*;
    // Four columns, three rows; CP437 bytes come out as host characters.
    let text=b"ab  cd\x84 \x00\x00\x00\x00";
    assert_eq!(copy_text(text,4,None),"ab\r\ncd\u{e4}");
    let sel=Selection{anchor:(1,1),head:(0,1)};
    assert_eq!(copy_text(text,4,Some(sel)),"b\r\ncd");
    // Past the screen: clamped.
    assert_eq!(copy_text(text,4,Some(Selection{anchor:(0,3),head:(9,9)})),"\r\ncd\u{e4}");
    assert_eq!(paste_keys("a\r\nB\t\u{e4}\u{263a}"),vec![0x1e61,0x1c0d,0x3042,0x0f09,0x2884]);
    // The selected cells are inverted on the picture.
    let mut frame=(4usize,2usize,vec![0x000000u32,0x0000aa,0xffffff,0x123456,0,0,0,0]);
    highlight(&mut frame,2,1,Selection{anchor:(0,1),head:(0,1)});
    assert_eq!(frame.2,vec![0x000000,0x0000aa,0x000000,0xedcba9,0,0,0xffffff,0xffffff]);
}

#[test]
fn altgr_characters_type_plainly_in_the_terminal(){
    // AltGr arrives as Ctrl+Alt with the typed character (Windows consoles).
    let altgr=Mods{shift:false,ctrl:true,alt:true};
    for (c,word) in [('@',0x0340u16),('\\',0x2b5c),('|',0x2b7c),('{',0x1a7b),(']',0x1b5d),('$',0x0524)]{
        assert_eq!(bios_word(Key::Char(c),typed_mods(c,altgr)),Some(word),"{c:?}");
    }
    // A real Ctrl or Alt combination keeps its modifiers.
    assert_eq!(typed_mods('\u{3}',altgr),altgr);
    assert_eq!(typed_mods(']',CTRL),CTRL);
    assert_eq!(typed_mods('x',ALT),ALT);
}

#[test]
fn catalog_launch_inherits_normal_configured_drives_and_ram_drive(){
    let nonce=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let root=std::env::temp_dir().join(format!("pil-catalog-{}-{nonce}",std::process::id()));
    std::fs::create_dir(&root).unwrap();
    for folder in ["cdrive","ddrive","edrive"]{
        std::fs::create_dir(root.join(folder)).unwrap();std::fs::write(root.join(folder).join("NOTE.TXT"),folder.as_bytes()).unwrap();
    }
    let program=root.join("sample.com");std::fs::write(&program,[0xf4]).unwrap();
    let config=root.join("normal.conf");std::fs::write(&config,"C=cdrive\nD=ddrive\nE=edrive\ncountry=358\nfpu=80387\n").unwrap();
    let session=catalog::open_program(program.to_str().unwrap(),None,Some(config.to_str().unwrap())).unwrap();
    let p=session.machine().unwrap();
    for (drive,expected) in [("C:\\NOTE.TXT","cdrive"),("D:\\NOTE.TXT","ddrive"),("E:\\NOTE.TXT","edrive")]{
        assert_eq!(&crate::dos::files::program(p,drive.as_bytes()).unwrap().unwrap().0[..],expected.as_bytes());
    }
    assert_eq!(p.drive.memory_letter(),Some(b'Z'));assert_eq!(p.country,358);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(windows)]
#[test]
fn host_media_shortcut_never_becomes_a_guest_enter(){
    use crate::dine_window::Input;
    assert_eq!(window_event(Input::Char{ch:'\r',shift:true,ctrl:true,alt:false}),Some(Event::HostMenu));
    assert_eq!(window_event(Input::Char{ch:'\r',shift:false,ctrl:false,alt:false}),Some(Event::Key(Key::Char('\r'),Mods::default())));
    assert_eq!(window_event(Input::Menu{id:104}),Some(Event::HostCommand(104)));
    assert_eq!(window_event(Input::Char{ch:'\u{1b}',shift:false,ctrl:false,alt:false}),Some(Event::Key(Key::Escape,Mods::default())));
}

#[derive(Default)]
struct Recorder{plays:Vec<usize>}
impl speaker::Player for Recorder{fn play(&mut self,wav:&std::sync::Arc<Vec<u8>>){self.plays.push(wav.len());}}
#[test]
fn speaker_clicks_follow_new_counts_at_most_five_a_second(){
    let t0=Instant::now();let at=|ms:u64|t0+Duration::from_millis(ms);
    let mut c=speaker::Clicks::new(5,3,Recorder::default());
    c.observe(3,at(0));assert!(c.player().plays.is_empty(),"the count at start is not a click");
    c.observe(4,at(0));assert_eq!(c.player().plays,vec![44+440]);
    c.observe(9,at(150));assert_eq!(c.player().plays.len(),1,"within 0.2 s");
    c.observe(9,at(300));assert_eq!(c.player().plays.len(),1,"no new count");
    c.observe(10,at(300));assert_eq!(c.player().plays.len(),2);
    // A new program counts from zero again.
    c.observe(0,at(600));c.observe(1,at(600));assert_eq!(c.player().plays.len(),3);
    let mut muted=speaker::Clicks::new(0,0,Recorder::default());
    muted.observe(5,at(0));assert!(muted.player().plays.is_empty());
}
#[test]
fn speaker_click_is_a_short_quiet_wav(){
    let wav=speaker::click(5);
    assert_eq!((&wav[0..4],&wav[8..16],&wav[36..40]),(&b"RIFF"[..],&b"WAVEfmt "[..],&b"data"[..]));
    assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()),22050);
    let samples:Vec<i16>=wav[44..].chunks(2).map(|b|i16::from_le_bytes([b[0],b[1]])).collect();
    assert_eq!(samples.len(),220);
    let peak=samples.iter().map(|s|s.unsigned_abs()).max().unwrap();
    assert!((1500..=1640).contains(&peak),"5% of full scale, got {peak}");
    assert_eq!((samples[0],samples[219]),(0,0));
    assert!(speaker::click(100).iter().zip(&wav).any(|(a,b)|a!=b));
}
