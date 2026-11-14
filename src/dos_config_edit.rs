//! A configuration file as its lines, for the setup screen of `pil --pildos`.
//!
//! The format is the one `dos_config` reads, unchanged. Editing keeps the
//! file as written: comments, blank lines, the order of settings, the key
//! spelling (`D:`, `utc-offset`, the `#@` prefix), the spacing around `=`,
//! the line ends and a leading BOM. A changed value rewrites only its own
//! line; a new setting goes at the end; a cleared one loses its line.
use std::path::Path;

/// What a setting holds, for the screen's editing rules.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum Kind{
    /// Free text (paths, DOS command tail, COM1 target).
    Text,
    /// One of these values (Delete removes the setting: the default).
    Choice(&'static [&'static str]),
    /// true or false; not set means false.
    Bool,
}
/// One setting of the format: its canonical key and how the screen shows it.
#[derive(Clone,Copy,Debug)]
pub(crate) struct Field{
    pub(crate) key:&'static str,
    pub(crate) label:&'static str,
    pub(crate) kind:Kind,
    pub(crate) help:&'static str,
    /// Read only by the Python terminal (tools/dos_terminal.py), not by
    /// `pil --dos-console`.
    pub(crate) terminal_only:bool,
}
const fn field(key:&'static str,label:&'static str,kind:Kind,help:&'static str,terminal_only:bool)->Field{
    Field{key,label,kind,help,terminal_only}
}
/// Every setting docs/DOS_CONFIG.md lists, in screen order.
pub(crate) const FIELDS:&[Field]=&[
    field("program","Program",Kind::Text,"Host DOS COM/EXE file, or IMAGE.img::PATH\\PROGRAM inside an image. Needed to start.",false),
    field("args","Command line",Kind::Text,"DOS command tail (printable ASCII, at most 125 characters).",false),
    field("a","Drive A:",Kind::Text,"Host folder (writable) or FAT .img/.ima image (read-only).",false),
    field("b","Drive B:",Kind::Text,"Host folder (writable) or FAT .img/.ima image (read-only).",false),
    field("c","Drive C:",Kind::Text,"Host folder (writable) or FAT .img/.ima image (read-only).",false),
    field("d","Drive D:",Kind::Text,"Host folder (writable) or FAT .img/.ima image (read-only).",false),
    field("e","Drive E:",Kind::Text,"Host folder (writable) or FAT .img/.ima image (read-only).",false),
    field("country","Country",Kind::Choice(&["1","358"]),"1 = USA (default), 358 = Finland.",false),
    field("fpu","Coprocessor",Kind::Choice(&["none","80387"]),"none (default) or the partial 80387 backend.",false),
    field("mouse","Mouse",Kind::Bool,"Experimental INT 33h mouse driver.",false),
    field("ansi","ANSI driver",Kind::Bool,"ANSI.SYS-style console driver: ESC [ colours and cursor moves.",false),
    field("utc_offset","UTC offset",Kind::Text,"Hours of the DOS clock from UTC: 2, -5, 5.5 or +02:00. Empty: this computer's.",false),
    field("com1","COM1",Kind::Text,"console or tcp:PORT.",true),
    field("speaker_volume","Speaker volume",Kind::Text,"PC speaker click volume 0..100 (0 mutes; default 5).",true),
    field("headless","Headless",Kind::Bool,"Run without a window, through the terminal.",true),
    field("shell","Shell",Kind::Bool,"The program is a command shell (for headless: /K CTTY COM1).",true),
    field("rust_graphics","Rust graphics",Kind::Bool,"Native Windows CGA/VGA with headless.",true),
    field("engine","Engine",Kind::Text,"Path to the pil executable (relative to this file).",true),
];

/// A setting's line: where its value starts and ends in the text.
#[derive(Clone,Debug)]
struct Setting{line:usize,key:String,value_start:usize}

/// A configuration file being edited.
#[derive(Clone,Debug)]
pub(crate) struct Document{
    lines:Vec<String>,
    bom:bool,
    crlf:bool,
    /// The last line ended with a line break.
    final_newline:bool,
}

/// The canonical key of a setting's written key, or None for keys outside
/// the format (the parser reports those).
fn canonical(key:&str)->Option<&'static str>{
    let lower=key.trim().to_ascii_lowercase().replace('-',"_");
    let lower=if lower.len()==2&&lower.ends_with(':'){lower[..1].to_string()}else{lower};
    FIELDS.iter().map(|f|f.key).find(|k|*k==lower)
}

impl Document{
    pub(crate) fn parse(text:&str)->Self{
        let bom=text.starts_with('\u{feff}');
        let text=text.trim_start_matches('\u{feff}');
        let crlf=text.contains("\r\n");
        let final_newline=text.is_empty()||text.ends_with('\n');
        let lines=text.lines().map(|l|l.trim_end_matches('\r').to_string()).collect();
        Self{lines,bom,crlf,final_newline}
    }
    pub(crate) fn text(&self)->String{
        let newline=if self.crlf{"\r\n"}else{"\n"};
        let mut out=String::new();
        if self.bom{out.push('\u{feff}');}
        out.push_str(&self.lines.join(newline));
        if self.final_newline&&!self.lines.is_empty(){out.push_str(newline);}
        out
    }
    /// The setting on line `i`, as the parser reads it.
    fn setting_at(&self,i:usize)->Option<Setting>{
        let line=&self.lines[i];
        let start=line.len()-line.trim_start().len();
        let rest=&line[start..];
        let (offset,rest)=if let Some(r)=rest.strip_prefix("#@"){(start+2,r)}else{
            if rest.trim().is_empty()||rest.starts_with('#')||rest.starts_with(';'){return None;}
            (start,rest)
        };
        let eq=rest.find('=')?;
        let key=rest[..eq].trim().to_string();
        let after=&rest[eq+1..];
        let value_start=offset+eq+1+(after.len()-after.trim_start().len());
        Some(Setting{line:i,key,value_start})
    }
    fn find(&self,key:&str)->Option<Setting>{
        (0..self.lines.len()).filter_map(|i|self.setting_at(i)).find(|s|canonical(&s.key)==Some(key))
    }
    /// The value of a setting (canonical key), trimmed, or None if absent.
    pub(crate) fn get(&self,key:&str)->Option<String>{
        self.find(key).map(|s|self.lines[s.line][s.value_start..].trim().to_string())
    }
    /// Set a setting (canonical key) to `value`, or remove its line for None.
    pub(crate) fn set(&mut self,key:&str,value:Option<&str>){
        match (self.find(key),value){
            (Some(s),Some(v))=>{
                let line=&mut self.lines[s.line];
                line.truncate(s.value_start);
                line.push_str(v.trim());
            },
            (Some(s),None)=>{self.lines.remove(s.line);},
            (None,Some(v))=>{
                let written=if key.len()==1{key.to_ascii_uppercase()}else{key.to_string()};
                self.lines.push(format!("{written}={}",v.trim()));
                self.final_newline=true;
            },
            (None,None)=>{},
        }
    }
    /// Check the text with the configuration parser, paths based on `parent`.
    pub(crate) fn check(&self,parent:&Path)->Result<(),String>{
        super::check(&self.text(),parent)
    }
}

/// Check one value as the parser will, before it goes into the document.
pub(crate) fn check_value(key:&str,value:&str)->Result<(),String>{
    let value=value.trim();
    match key{
        "utc_offset"=>super::parse_offset(value).map(|_|()),
        "speaker_volume"=>if value.parse::<u8>().is_ok_and(|v|v<=100){Ok(())}else{Err("speaker volume must be 0..100".into())},
        "args"=>{
            if !value.chars().all(|c|(' '..='~').contains(&c)){return Err("the command line must be printable ASCII".into());}
            if value.len()>125{return Err("the command line is longer than 125 characters".into());}
            Ok(())
        },
        "com1"=>{
            let lower=value.to_ascii_lowercase();
            if lower=="console"||lower.strip_prefix("tcp:").is_some_and(|p|p.parse::<u16>().is_ok_and(|p|p>0)){Ok(())}
            else{Err("COM1 must be console or tcp:PORT".into())}
        },
        _=>Ok(()),
    }
}
