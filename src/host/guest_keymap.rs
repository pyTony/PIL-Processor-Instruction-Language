//! The guest's keyboard layout, for text typed as key presses: a paste into
//! a booted machine and `pil --boot-probe --type`. An operating system that
//! reads scan codes (Linux with `loadkeys`) turns each key into a character
//! through its own keymap, so the text must be typed on the keys that give
//! those characters in that keymap, as a person at that keyboard would.
//! Layouts as the PC keyboards of the time print them (set-1 scan codes):
//! US, UK and Finnish (also Swedish). Dine's own DOS reads BIOS words and
//! needs none of this.

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
    match c{'\n'=>return plain(0x1c),'\t'=>return plain(0x0f),' '=>return plain(0x39),_=>{}}
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
        let ascii=match c{'\n'=>0x0d,'\t'=>0x09,_=>crate::dos_encoding::command_line(&c.to_string()).ok()
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
    }
}
