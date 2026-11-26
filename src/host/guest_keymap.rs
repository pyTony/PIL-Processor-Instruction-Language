//! The guest's keyboard layout, for text typed as key presses: a paste into
//! a booted machine and `pil --boot-probe --type`. An operating system that
//! reads scan codes (Linux with `loadkeys`) turns each key into a character
//! through its own keymap, so the text must be typed on the keys that give
//! those characters in that keymap, as a person at that keyboard would.
//! Layouts as the PC keyboards of the time print them (set-1 scan codes):
//! US, UK and Finnish (also Swedish). Dine's own DOS reads BIOS words and
//! needs none of this.
use super::host_console::{Key,Mods};

/// A guest keyboard layout.
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
pub(crate) enum Keymap{#[default]Us,Uk,Fi}

impl Keymap{
    pub(crate) fn parse(name:&str)->Result<Self,String>{
        match name.to_ascii_lowercase().as_str(){
            "us"=>Ok(Self::Us),"uk"|"gb"=>Ok(Self::Uk),"fi"|"se"|"sv"=>Ok(Self::Fi),
            _=>Err(format!("unknown guest keymap {name:?} (us, uk, fi)")),
        }
    }
    pub(crate) fn name(self)->&'static str{match self{Self::Us=>"us",Self::Uk=>"uk",Self::Fi=>"fi"}}
}

/// One key press that types a character.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
struct Stroke{scan:u8,shift:bool,altgr:bool,dead:bool}

const LETTERS:[(&str,u8);3]=[("qwertyuiop",0x10),("asdfghjkl",0x1e),("zxcvbnm",0x2c)];
const DIGITS:&str="1234567890";

/// (character, scan code, shift, AltGr, dead key) beyond letters and digits.
type Extra=(char,u8,bool,bool,bool);
const US:&[Extra]=&[
    ('-',0x0c,false,false,false),('=',0x0d,false,false,false),('[',0x1a,false,false,false),(']',0x1b,false,false,false),
    (';',0x27,false,false,false),('\'',0x28,false,false,false),('`',0x29,false,false,false),('\\',0x2b,false,false,false),
    (',',0x33,false,false,false),('.',0x34,false,false,false),('/',0x35,false,false,false),
    ('!',0x02,true,false,false),('@',0x03,true,false,false),('#',0x04,true,false,false),('$',0x05,true,false,false),
    ('%',0x06,true,false,false),('^',0x07,true,false,false),('&',0x08,true,false,false),('*',0x09,true,false,false),
    ('(',0x0a,true,false,false),(')',0x0b,true,false,false),('_',0x0c,true,false,false),('+',0x0d,true,false,false),
    ('{',0x1a,true,false,false),('}',0x1b,true,false,false),(':',0x27,true,false,false),('"',0x28,true,false,false),
    ('~',0x29,true,false,false),('|',0x2b,true,false,false),('<',0x33,true,false,false),('>',0x34,true,false,false),
    ('?',0x35,true,false,false),
];
const UK:&[Extra]=&[
    ('-',0x0c,false,false,false),('=',0x0d,false,false,false),('[',0x1a,false,false,false),(']',0x1b,false,false,false),
    (';',0x27,false,false,false),('\'',0x28,false,false,false),('`',0x29,false,false,false),('#',0x2b,false,false,false),
    (',',0x33,false,false,false),('.',0x34,false,false,false),('/',0x35,false,false,false),('\\',0x56,false,false,false),
    ('!',0x02,true,false,false),('"',0x03,true,false,false),('\u{a3}',0x04,true,false,false),('$',0x05,true,false,false),
    ('%',0x06,true,false,false),('^',0x07,true,false,false),('&',0x08,true,false,false),('*',0x09,true,false,false),
    ('(',0x0a,true,false,false),(')',0x0b,true,false,false),('_',0x0c,true,false,false),('+',0x0d,true,false,false),
    ('{',0x1a,true,false,false),('}',0x1b,true,false,false),(':',0x27,true,false,false),('@',0x28,true,false,false),
    ('\u{ac}',0x29,true,false,false),('~',0x2b,true,false,false),('<',0x33,true,false,false),('>',0x34,true,false,false),
    ('?',0x35,true,false,false),('|',0x56,true,false,false),
];
const FI:&[Extra]=&[
    ('+',0x0c,false,false,false),('\u{e5}',0x1a,false,false,false),('\u{f6}',0x27,false,false,false),('\u{e4}',0x28,false,false,false),
    ('\u{a7}',0x29,false,false,false),('\'',0x2b,false,false,false),(',',0x33,false,false,false),('.',0x34,false,false,false),
    ('-',0x35,false,false,false),('<',0x56,false,false,false),
    ('!',0x02,true,false,false),('"',0x03,true,false,false),('#',0x04,true,false,false),('\u{a4}',0x05,true,false,false),
    ('%',0x06,true,false,false),('&',0x07,true,false,false),('/',0x08,true,false,false),('(',0x09,true,false,false),
    (')',0x0a,true,false,false),('=',0x0b,true,false,false),('?',0x0c,true,false,false),('\u{c5}',0x1a,true,false,false),
    ('\u{d6}',0x27,true,false,false),('\u{c4}',0x28,true,false,false),('\u{bd}',0x29,true,false,false),('*',0x2b,true,false,false),
    (';',0x33,true,false,false),(':',0x34,true,false,false),('_',0x35,true,false,false),('>',0x56,true,false,false),
    ('@',0x03,false,true,false),('\u{a3}',0x04,false,true,false),('$',0x05,false,true,false),('{',0x08,false,true,false),
    ('[',0x09,false,true,false),(']',0x0a,false,true,false),('}',0x0b,false,true,false),('\\',0x0c,false,true,false),
    ('|',0x56,false,true,false),
    // Accents are dead keys; the character alone is the accent and a space.
    ('\u{b4}',0x0d,false,false,true),('`',0x0d,true,false,true),('\u{a8}',0x1b,false,false,true),('^',0x1b,true,false,true),
    ('~',0x1b,false,true,true),
];

fn stroke(map:Keymap,c:char)->Option<Stroke>{
    let plain=|scan|Some(Stroke{scan,shift:false,altgr:false,dead:false});
    // Backspace (BS or DEL, as terminals send it) and Esc are keys too.
    match c{'\n'=>return plain(0x1c),'\t'=>return plain(0x0f),' '=>return plain(0x39),
        '\u{8}'|'\u{7f}'=>return plain(0x0e),'\u{1b}'=>return plain(0x01),_=>{}}
    if let Some(i)=DIGITS.find(c){return plain(0x02+i as u8);}
    let lower=c.to_ascii_lowercase();
    if lower.is_ascii_lowercase(){
        let scan=LETTERS.iter().find_map(|(row,start)|row.find(lower).map(|i|start+i as u8))?;
        return Some(Stroke{scan,shift:c.is_ascii_uppercase(),altgr:false,dead:false});
    }
    let table=match map{Keymap::Us=>US,Keymap::Uk=>UK,Keymap::Fi=>FI};
    table.iter().find(|e|e.0==c).map(|&(_,scan,shift,altgr,dead)|Stroke{scan,shift,altgr,dead})
}

/// One set-1 key edge: scan code, E0-prefixed, press (else release), and
/// the BIOS word a press stores (for a BIOS keyboard handler).
pub(crate) type Edge=(u8,bool,bool,Option<u16>);
const SHIFT:u8=0x2a;
const ALT:u8=0x38;
const SPACE:u8=0x39;

/// The key edges that type `text` on `map`: Shift and AltGr (right Alt)
/// around the keys that need them, a space after a dead key. Line ends are
/// Enter, a CR LF pair one Enter. Characters the layout has no key for are
/// left out.
pub(crate) fn edges(map:Keymap,text:&str)->Vec<Edge>{
    let text=text.replace("\r\n","\n").replace('\r',"\n");
    let mut out=Vec::new();
    for c in text.chars(){
        let Some(s)=stroke(map,c) else{continue};
        let ascii=match c{'\n'=>0x0d,'\t'=>0x09,'\u{8}'|'\u{7f}'=>0x08,'\u{1b}'=>0x1b,_=>crate::dos_encoding::command_line(&c.to_string()).ok()
            .and_then(|b|if let [b]=b[..]{Some(b)}else{None}).unwrap_or(0)};
        let word=u16::from(s.scan)<<8|u16::from(ascii);
        if s.shift{out.push((SHIFT,false,true,None));}
        if s.altgr{out.push((ALT,true,true,None));}
        out.push((s.scan,false,true,Some(word)));
        out.push((s.scan,false,false,None));
        if s.altgr{out.push((ALT,true,false,None));}
        if s.shift{out.push((SHIFT,false,false,None));}
        if s.dead{out.push((SPACE,false,true,None));out.push((SPACE,false,false,None));}
    }
    out
}

/// The key edges of one host key from the terminal (`--tty`) on `map`, for
/// a booted machine, which reads scan codes through IRQ1: Shift, Ctrl and
/// Alt held around the key, the cursor and editing block as the enhanced
/// keyboard's E0h-prefixed keys (not the keypad), a control code as Ctrl
/// and its letter. `word` is the BIOS word a press stores for a named key.
pub(crate) fn key_edges(map:Keymap,key:Key,mods:Mods,word:Option<u16>)->Vec<Edge>{
    const CTRL:u8=0x1d;
    let mut mods=mods;
    let mut keys:Vec<Edge>=match key{
        Key::Char(c) if (1..=26).contains(&(c as u32))&&!matches!(c,'\u{8}'|'\t'|'\r'|'\n')=>{
            mods.ctrl=true;edges(map,&char::from(c as u8+0x60).to_string())
        },
        Key::Char(c) if mods.ctrl||mods.alt=>{
            // The key that types the character; a letter's case is the
            // Shift the host reported, a symbol brings its own Shift.
            let keys=edges(map,&c.to_ascii_lowercase().to_string());
            if keys.first()==Some(&(SHIFT,false,true,None)){mods.shift=false;}
            keys
        },
        Key::Char(c)=>{mods.shift=false;edges(map,&c.to_string())},
        named=>{
            let (scan,extended)=match named{
                Key::Enter=>(0x1c,false),Key::Backspace=>(0x0e,false),Key::Tab=>(0x0f,false),Key::Escape=>(0x01,false),
                Key::Up=>(0x48,true),Key::Down=>(0x50,true),Key::Left=>(0x4b,true),Key::Right=>(0x4d,true),
                Key::Home=>(0x47,true),Key::End=>(0x4f,true),Key::PageUp=>(0x49,true),Key::PageDown=>(0x51,true),
                Key::Insert=>(0x52,true),Key::Delete=>(0x53,true),
                Key::F(n @ 1..=10)=>(0x3a+n,false),Key::F(11)=>(0x57,false),Key::F(12)=>(0x58,false),
                Key::F(_)|Key::Char(_)=>return Vec::new(),
            };
            vec![(scan,extended,true,word),(scan,extended,false,None)]
        },
    };
    if keys.is_empty(){return keys;}
    // Modifiers pressed first (Ctrl, Alt, Shift) and released last.
    for (held,scan) in [(mods.shift,SHIFT),(mods.alt,ALT),(mods.ctrl,CTRL)]{
        if held{keys.insert(0,(scan,false,true,None));keys.push((scan,false,false,None));}
    }
    keys
}

/// Releases of every Shift, Ctrl and Alt key, sent before a paste: the
/// host shortcut that pasted (Ctrl+Shift+V, Shift+Insert) may still hold
/// some of them down in the guest. A release of a key that is up changes
/// nothing.
pub(crate) const RELEASE_MODIFIERS:[Edge;6]=[(0x2a,false,false,None),(0x36,false,false,None),
    (0x1d,false,false,None),(0x1d,true,false,None),(0x38,false,false,None),(0x38,true,false,None)];

#[cfg(test)]
mod tests{
    use super::*;
    fn makes(map:Keymap,text:&str)->Vec<(u8,bool)>{
        edges(map,text).into_iter().filter(|e|e.2).map(|e|(e.0,e.1)).collect()
    }
    #[test]
    fn text_is_typed_on_the_keys_of_the_guest_layout(){
        // US: '-' and '/' on their own keys; Finnish: '-' where US has '/',
        // '/' is Shift+7, '+' where US has '-'.
        assert_eq!(makes(Keymap::Us,"-/"),[(0x0c,false),(0x35,false)]);
        assert_eq!(makes(Keymap::Fi,"-/+"),[(0x35,false),(0x2a,false),(0x08,false),(0x0c,false)]);
        // AltGr (E0 38) around the key, released after it; '|' on the 102nd key.
        assert_eq!(edges(Keymap::Fi,"|"),[(0x38,true,true,None),(0x56,false,true,Some(0x5600|u16::from(b'|'))),
            (0x56,false,false,None),(0x38,true,false,None)]);
        // UK: '"' is Shift+2, '@' Shift+' and '#' its own key.
        assert_eq!(makes(Keymap::Uk,"\"@#"),[(0x2a,false),(0x03,false),(0x2a,false),(0x28,false),(0x2b,false)]);
        // A dead accent is the accent key and a space; CR LF is one Enter.
        assert_eq!(makes(Keymap::Fi,"^\r\n"),[(0x2a,false),(0x1b,false),(0x39,false),(0x1c,false)]);
        // Capitals with Shift; the word a BIOS handler stores is the character.
        assert_eq!(edges(Keymap::Us,"A")[1],(0x1e,false,true,Some(0x1e41)));
        assert_eq!(Keymap::parse("FI"),Ok(Keymap::Fi));assert!(Keymap::parse("de").is_err());
        // BS and DEL (what a terminal sends for Backspace) are key 0Eh, ESC is Esc.
        assert_eq!(makes(Keymap::Us,"a\u{7f}\u{8}\u{1b}"),[(0x1e,false),(0x0e,false),(0x0e,false),(0x01,false)]);
        assert_eq!(edges(Keymap::Us,"\u{8}\u{1b}")[0].3,Some(0x0e08));
        assert_eq!(edges(Keymap::Us,"\u{8}\u{1b}")[2].3,Some(0x011b));
    }
    #[test]
    fn terminal_keys_are_key_presses_with_their_modifiers(){
        let none=Mods::default();
        let shift=Mods{shift:true,..none};let ctrl=Mods{ctrl:true,..none};let alt=Mods{alt:true,..none};
        // Backspace is its key, not the DEL byte; arrows and the editing
        // block are the E0h keys, not the keypad.
        assert_eq!(key_edges(Keymap::Us,Key::Backspace,none,Some(0x0e08)),[(0x0e,false,true,Some(0x0e08)),(0x0e,false,false,None)]);
        for (key,scan) in [(Key::Up,0x48),(Key::Down,0x50),(Key::Left,0x4b),(Key::Right,0x4d),(Key::Home,0x47),
            (Key::End,0x4f),(Key::PageUp,0x49),(Key::PageDown,0x51),(Key::Insert,0x52),(Key::Delete,0x53)]{
            assert_eq!(key_edges(Keymap::Fi,key,none,None),[(scan,true,true,None),(scan,true,false,None)]);
        }
        assert_eq!(key_edges(Keymap::Us,Key::F(1),none,None)[0].0,0x3b);
        assert_eq!(key_edges(Keymap::Us,Key::F(12),none,None)[0].0,0x58);
        assert!(key_edges(Keymap::Us,Key::F(13),none,None).is_empty());
        // Shift+Tab holds Shift around Tab.
        assert_eq!(key_edges(Keymap::Us,Key::Tab,shift,None),[(0x2a,false,true,None),(0x0f,false,true,None),
            (0x0f,false,false,None),(0x2a,false,false,None)]);
        // A capital brings its own Shift once, whether or not the host reported it.
        assert_eq!(key_edges(Keymap::Us,Key::Char('A'),shift,None),edges(Keymap::Us,"A"));
        // A symbol is typed on the guest layout: '-' on Finnish is the US '/' key.
        assert_eq!(key_edges(Keymap::Fi,Key::Char('-'),none,None),edges(Keymap::Fi,"-"));
        // Ctrl+C, and the control code 03h a terminal sends for it, are Ctrl and C.
        let ctrl_c=[(0x1d,false,true,None),(0x2e,false,true,Some(0x2e63)),(0x2e,false,false,None),(0x1d,false,false,None)];
        assert_eq!(key_edges(Keymap::Us,Key::Char('c'),ctrl,None),ctrl_c);
        assert_eq!(key_edges(Keymap::Us,Key::Char('\u{3}'),none,None),ctrl_c);
        // Alt+X and Ctrl+Alt+Del: modifiers pressed first, released last.
        assert_eq!(key_edges(Keymap::Us,Key::Char('x'),alt,None)[0],(0x38,false,true,None));
        let cad=key_edges(Keymap::Us,Key::Delete,Mods{ctrl:true,alt:true,shift:false},None);
        assert_eq!(cad.iter().map(|e|(e.0,e.1,e.2)).collect::<Vec<_>>(),
            [(0x1d,false,true),(0x38,false,true),(0x53,true,true),(0x53,true,false),(0x38,false,false),(0x1d,false,false)]);
    }
}
