//! X11 key translation for the Linux and Termux:X11 window (`dine_x11`).
//! Pure functions, no X connection: an X keycode and modifier state become
//! the same physical key (Windows virtual-key code, set-1 scan code, E0
//! prefix) and typed character the Win32 window hands the console, so one
//! input pipeline serves both windows. A booted Linux guest sees real key
//! edges from the physical part; a DOS program gets BIOS words.
//!
//! X keycodes are Linux evdev codes plus 8 (Xorg, Xvfb and Termux:X11 all
//! number them so). The evdev codes 1-88 are the set-1 scan codes; the keys
//! the PC/AT 101 keyboard sends behind E0 are listed in `scan_of`.

/// X11 core modifier bits in a key event's state.
pub(crate) const SHIFT:u16=0x01;
pub(crate) const LOCK:u16=0x02;
pub(crate) const CONTROL:u16=0x04;
pub(crate) const ALT:u16=0x08;
pub(crate) const NUM_LOCK:u16=0x10;
/// Mod5, where XKB puts ISO_Level3_Shift (AltGr) on the usual layouts.
pub(crate) const ALTGR:u16=0x80;

/// The keysym table of a server: `per` keysyms for each keycode from `min`.
#[derive(Clone,Debug,Default)]
pub(crate) struct Keymap{pub(crate) min:u8,pub(crate) per:usize,pub(crate) syms:Vec<u32>}

impl Keymap{
    /// Keysym of `code` in table column `column`, 0 when there is none.
    pub(crate) fn sym(&self,code:u8,column:usize)->u32{
        if self.per==0||column>=self.per||code<self.min{return 0;}
        let at=usize::from(code-self.min)*self.per+column;
        self.syms.get(at).copied().unwrap_or(0)
    }
}

/// One key event, as the Win32 window reports it.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) struct KeyInfo{
    /// Windows virtual-key code of the key itself (layout independent for
    /// letters), 0xFF for a key with none.
    pub(crate) vk:u16,
    /// Set-1 scan code, 0 when the key has none (it is then ignored).
    pub(crate) scan:u8,
    pub(crate) extended:bool,
    /// The character the key types with the held modifiers, if any.
    pub(crate) text:Option<char>,
    /// A dead accent: no character, and no key press of its own.
    pub(crate) dead:bool,
    pub(crate) shift:bool,
    pub(crate) ctrl:bool,
    pub(crate) alt:bool,
}

/// Set-1 scan code and E0 flag of an X keycode (evdev code plus 8).
pub(crate) fn scan_of(code:u8)->(u8,bool){
    let evdev=code.wrapping_sub(8);
    match evdev{
        1..=88=>(evdev,false),
        96=>(0x1c,true),    // keypad Enter
        97=>(0x1d,true),    // right Ctrl
        98=>(0x35,true),    // keypad /
        99=>(0x37,true),    // Print Screen
        100=>(0x38,true),   // right Alt (AltGr)
        102=>(0x47,true),   // Home
        103=>(0x48,true),   // Up
        104=>(0x49,true),   // Page Up
        105=>(0x4b,true),   // Left
        106=>(0x4d,true),   // Right
        107=>(0x4f,true),   // End
        108=>(0x50,true),   // Down
        109=>(0x51,true),   // Page Down
        110=>(0x52,true),   // Insert
        111=>(0x53,true),   // Delete
        125=>(0x5b,true),   // left Windows
        126=>(0x5c,true),   // right Windows
        127=>(0x5d,true),   // Menu
        _=>(0,false),
    }
}

/// The Windows virtual-key code for a key's unshifted keysym.
fn vk_of(sym:u32,code:u8)->u16{
    match sym{
        0x61..=0x7a=>sym as u16-0x20,         // a-z: A-Z
        0x41..=0x5a=>sym as u16,
        0x30..=0x39=>sym as u16,
        0x20=>0x20,
        0xff0d|0xff8d=>0x0d,                 // Return, KP_Enter
        0xff09=>0x09,0xff1b=>0x1b,0xff08=>0x08,
        0xffff|0xff9f=>0x2e,                  // Delete, KP_Delete
        0xff50|0xff95=>0x24,                  // Home
        0xff51|0xff96=>0x25,0xff52|0xff97=>0x26,0xff53|0xff98=>0x27,0xff54|0xff99=>0x28,
        0xff55|0xff9a=>0x21,0xff56|0xff9b=>0x22,
        0xff57|0xff9c=>0x23,                  // End
        0xff63|0xff9e=>0x2d,                  // Insert
        0xffbe..=0xffc9=>0x70+(sym-0xffbe) as u16, // F1-F12
        0xffe1=>0xa0,0xffe2=>0xa1,0xffe3=>0xa2,0xffe4=>0xa3,0xffe9=>0xa4,0xffea|0xfe03=>0xa5,
        0xffe5=>0x14,0xff7f=>0x90,0xff14=>0x91,0xff13=>0x13,
        0xffeb=>0x5b,0xffec=>0x5c,0xff67=>0x5d,
        0xffaa=>0x6a,0xffab=>0x6b,0xffad=>0x6d,0xffae=>0x6e,0xffaf=>0x6f, // KP * + - . /
        0xffb0..=0xffb9=>0x60+(sym-0xffb0) as u16,                        // KP 0-9
        _=>{let _=code;0xff},
    }
}

/// The character a keysym types (control keys give their control code).
fn char_of(sym:u32)->Option<char>{
    match sym{
        0xff0d|0xff8d=>Some('\r'),0xff09=>Some('\t'),0xff08=>Some('\u{8}'),0xff1b=>Some('\u{1b}'),
        0x20..=0x7e|0xa0..=0xff=>char::from_u32(sym),
        0xffaa=>Some('*'),0xffab=>Some('+'),0xffad=>Some('-'),0xffae=>Some('.'),0xffaf=>Some('/'),0xffbd=>Some('='),
        0xffb0..=0xffb9=>char::from_u32(0x30+(sym-0xffb0)),
        0x0100_0000..=0x0110_ffff=>char::from_u32(sym-0x0100_0000),
        _=>None,
    }
}

fn is_dead(sym:u32)->bool{(0xfe50..=0xfe6f).contains(&sym)}

/// A key event: `code` is the X keycode, `state` the modifier mask.
pub(crate) fn translate(map:&Keymap,code:u8,state:u16)->KeyInfo{
    let (scan,extended)=scan_of(code);
    let base=map.sym(code,0);
    let keypad=(0xff80..=0xffbd).contains(&base);
    let altgr=state&ALTGR!=0;
    let shift=state&SHIFT!=0;
    let letter=char_of(base).is_some_and(char::is_alphabetic);
    // Caps Lock swaps the case of letters only; Num Lock swaps the keypad's
    // numbers and navigation keys.
    let upper=if keypad{shift!=(state&NUM_LOCK!=0)}else{shift!=(letter&&state&LOCK!=0)};
    let mut column=usize::from(upper);
    if altgr&&!keypad{column+=if map.per>=6{4}else{2};}
    let mut sym=map.sym(code,column);
    if sym==0{sym=map.sym(code,usize::from(upper));}
    if sym==0{sym=base;}
    let mut ctrl=state&CONTROL!=0;
    let mut alt=state&ALT!=0;
    let mut text=char_of(sym);
    // Ctrl with a letter types its control code, as the Windows window's
    // WM_CHAR does; AltGr is no Ctrl+Alt.
    if ctrl&&!alt&&!altgr{
        if let Some(c)=text.filter(char::is_ascii_alphabetic){text=char::from_u32(u32::from(c.to_ascii_lowercase())&0x1f);}
    }
    if altgr{ctrl=false;alt=false;}
    // Alt is a modifier of the key, not a typed character (the Win32 window
    // reports WM_SYSCHAR with Alt set; the pipeline ignores it for letters).
    if !ctrl&&alt&&text.is_some_and(|c|c.is_control()){text=None;}
    let mut vk=vk_of(base,code);
    // A keypad key is what it resolves to: a number with Num Lock on, a
    // navigation key with it off.
    if keypad{vk=vk_of(sym,code);}
    KeyInfo{vk,scan,extended,text,dead:is_dead(sym),shift,ctrl,alt}
}

#[cfg(test)]
mod tests{
    use super::*;

    /// A tiny table: keycode 38 (A), 10 (1 and !, AltGr 2 gives @), 79
    /// (keypad 7 with Home), 36 (Return), 111 (Delete).
    fn map()->Keymap{
        let mut syms=vec![0;120*6];
        let mut set=|code:usize,row:[u32;6]|{syms[code*6..code*6+6].copy_from_slice(&row);};
        set(38,[0x61,0x41,0,0,0xe4,0xc4]);
        set(10,[0x31,0x21,0,0,0x40,0xa1]);
        set(36,[0xff0d,0,0,0,0,0]);
        set(79,[0xff95,0xffb7,0,0,0,0]);
        set(119,[0xffff,0,0,0,0,0]);
        set(50,[0xffe1,0,0,0,0,0]);
        set(67,[0xffbe,0,0,0,0,0]);
        Keymap{min:0,per:6,syms}
    }

    #[test]
    fn evdev_codes_are_set_one_scan_codes_with_e0_keys_marked(){
        assert_eq!(scan_of(9),(0x01,false));          // Esc
        assert_eq!(scan_of(38),(0x1e,false));         // A
        assert_eq!(scan_of(50),(0x2a,false));         // left Shift
        assert_eq!(scan_of(62),(0x36,false));         // right Shift
        assert_eq!(scan_of(67),(0x3b,false));         // F1
        assert_eq!(scan_of(95),(0x57,false));         // F11
        assert_eq!(scan_of(113),(0x4b,true));         // Left
        assert_eq!(scan_of(111),(0x48,true));         // Up
        assert_eq!(scan_of(119),(0x53,true));         // Delete
        assert_eq!(scan_of(108),(0x38,true));         // AltGr
        assert_eq!(scan_of(104),(0x1c,true));         // keypad Enter
        assert_eq!(scan_of(79),(0x47,false));         // keypad 7 / Home
        assert_eq!(scan_of(0),(0,false));
        assert_eq!(scan_of(250),(0,false));
    }

    #[test]
    fn letters_type_by_shift_caps_and_ctrl(){
        let m=map();
        let a=translate(&m,38,0);
        assert_eq!((a.vk,a.scan,a.text),(0x41,0x1e,Some('a')));
        assert_eq!(translate(&m,38,SHIFT).text,Some('A'));
        assert_eq!(translate(&m,38,LOCK).text,Some('A'));
        assert_eq!(translate(&m,38,LOCK|SHIFT).text,Some('a'));
        // Ctrl+A is the control code, with Ctrl set, as WM_CHAR gives it.
        let c=translate(&m,38,CONTROL);
        assert_eq!((c.text,c.ctrl),(Some('\u{1}'),true));
        // Alt+A keeps its letter vk; the pipeline ignores the Alt text of a letter.
        let alt=translate(&m,38,ALT);
        assert_eq!((alt.text,alt.alt,alt.vk),(Some('a'),true,0x41));
    }

    #[test]
    fn altgr_uses_the_third_level_and_is_not_ctrl_alt(){
        let m=map();
        let at=translate(&m,10,ALTGR);
        assert_eq!(at.text,Some('@'));
        assert!(!at.ctrl&&!at.alt);
        assert_eq!(translate(&m,10,0).text,Some('1'));
        assert_eq!(translate(&m,10,SHIFT).text,Some('!'));
        assert_eq!(translate(&m,38,ALTGR).text,Some('\u{e4}'));
    }

    #[test]
    fn control_keys_type_their_control_codes_and_navigation_types_nothing(){
        let m=map();
        let enter=translate(&m,36,0);
        assert_eq!((enter.vk,enter.text,enter.scan,enter.extended),(0x0d,Some('\r'),0x1c,false));
        let del=translate(&m,119,0);
        assert_eq!((del.vk,del.text,del.scan,del.extended),(0x2e,None,0x53,true));
        let f1=translate(&m,67,0);
        assert_eq!((f1.vk,f1.text),(0x70,None));
        let shift=translate(&m,50,0);
        assert_eq!((shift.vk,shift.scan,shift.text),(0xa0,0x2a,None));
    }

    #[test]
    fn the_keypad_follows_num_lock(){
        let m=map();
        let nav=translate(&m,79,0);
        assert_eq!((nav.vk,nav.text,nav.scan,nav.extended),(0x24,None,0x47,false));
        let digit=translate(&m,79,NUM_LOCK);
        assert_eq!((digit.vk,digit.text),(0x67,Some('7')));
        // Shift flips it, as on a PC.
        assert_eq!(translate(&m,79,NUM_LOCK|SHIFT).text,None);
    }

    #[test]
    fn unknown_keycodes_and_empty_maps_are_harmless(){
        let empty=Keymap::default();
        let k=translate(&empty,38,0);
        assert_eq!((k.vk,k.text,k.scan),(0xff,None,0x1e));
        assert_eq!(map().sym(200,0),0);
    }
}
