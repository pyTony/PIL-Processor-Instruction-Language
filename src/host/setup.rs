//! `pil --pildos`: the PILdos setup screen and launcher.
//!
//! An 80x25 text screen, drawn by the same front ends as `--dos-console`
//! (the Dine window on Windows, the host terminal elsewhere), edits the
//! selected configuration file (`--config`/PILDOS_CONFIG/./pildos.conf) in
//! its own format, keeping its lines as written (`dos_config::edit`). F10
//! saves and starts the configured program in the same front end; when the
//! program ends, the screen returns. Nothing here knows which program runs.
//!
//! When the file exists and names a program, `run` first shows a start page:
//! Enter starts the program at once, S (or F2, Tab) opens the setup screen,
//! and Esc on the setup screen comes back to it.
use super::host_console::{self,Event,Front,Key,Mods,Options};
use crate::dos_config::{self,edit::{self,Document,Kind,FIELDS}};
use std::path::{Path,PathBuf};
use std::sync::mpsc;
use std::time::Duration;

pub(crate) const COLUMNS:usize=80;
pub(crate) const ROWS:usize=25;
const FIRST_FIELD:usize=2;
const LABEL_COLUMN:usize=2;
const VALUE_COLUMN:usize=21;
const VALUE_WIDTH:usize=COLUMNS-VALUE_COLUMN-1;

const SCREEN:u8=0x17;
const TITLE:u8=0x1f;
const LABEL_SELECTED:u8=0x1e;
const VALUE:u8=0x1f;
const VALUE_UNSET:u8=0x13;
const VALUE_SELECTED:u8=0x70;
const VALUE_EDITING:u8=0x30;
const STATUS_OK:u8=0x1a;
const STATUS_ERROR:u8=0x4f;
const KEYS:u8=0x70;

/// What the screen asks its loop to do after a key.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum Action{Stay,Start,Quit}

#[derive(Clone,Debug)]
struct Editing{text:Vec<char>,cursor:usize}

/// The setup screen's state: the document and where the user is in it.
pub(crate) struct Setup{
    pub(crate) doc:Document,
    path:PathBuf,
    selected:usize,
    editing:Option<Editing>,
    dirty:bool,
    /// One line under the help text, and whether it reports an error.
    status:(String,bool),
    /// Esc was pressed once with unsaved changes.
    quit_armed:bool,
    /// The start page: `Some(text as saved)` once it exists, so that Esc on
    /// the setup screen goes back to it (dropping unsaved changes).
    front:Option<String>,
    /// The start page is showing.
    on_front:bool,
    /// Edit shared runtime settings without launching the config's program.
    edit_only:bool,
}

fn cp437(c:char)->u8{
    // 01h-1Fh are the IBM glyphs (arrows and such), as the font draws them.
    if (1..0x20).contains(&(c as u32)){return c as u8;}
    crate::dos_encoding::command_line(&c.to_string()).ok().and_then(|b|b.first().copied()).unwrap_or(b'?')
}
fn put(cells:&mut [u8],row:usize,column:usize,width:usize,text:&str,attr:u8){
    let mut chars=text.chars();
    for x in column..(column+width).min(COLUMNS){
        let at=(row*COLUMNS+x)*2;
        cells[at]=chars.next().map_or(b' ',cp437);
        cells[at+1]=attr;
    }
}
/// The end of `text` that fits `width`, with "..." in front when cut.
fn tail(text:&str,width:usize)->String{
    let n=text.chars().count();
    if n<=width{return text.to_string();}
    let keep:String=text.chars().skip(n-(width-3)).collect();
    format!("...{keep}")
}

impl Setup{
    /// The screen for the configuration at `path`. A file that does not exist
    /// yet starts from `template` (pildos.conf.example), if given.
    pub(crate) fn new(path:PathBuf,text:Option<&str>,template:Option<&str>)->Self{
        let (doc,dirty,status)=match (text,template){
            (Some(text),_)=>(Document::parse(text),false,(format!("Loaded {}.",path.display()),false)),
            (None,Some(t))=>(Document::parse(t),true,("New file from pildos.conf.example: F2 saves it.".to_string(),false)),
            (None,None)=>(Document::parse(""),true,("New file: F2 saves it.".to_string(),false)),
        };
        let mut setup=Self{doc,path,selected:0,editing:None,dirty,status,quit_armed:false,front:None,on_front:false,edit_only:false};
        if let Err(e)=setup.doc.check(setup.parent()){setup.status=(e,true);}
        setup
    }
    /// Read the file at `path`, or the example next to it if it is missing.
    pub(crate) fn load(path:PathBuf)->Result<Self,String>{
        if path.exists(){
            let meta=std::fs::metadata(&path).map_err(|e|format!("cannot read {}: {e}",path.display()))?;
            if meta.len()>32768{return Err(format!("{} is larger than 32 KiB",path.display()));}
            let text=std::fs::read_to_string(&path).map_err(|e|format!("cannot read {}: {e}",path.display()))?;
            return Ok(Self::new(path,Some(&text),None));
        }
        let parent=path.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let template=std::fs::read_to_string(parent.join("pildos.conf.example")).ok();
        Ok(Self::new(path,None,template.as_deref()))
    }
    /// Show the start page first, when the file is saved, valid and names a
    /// program; otherwise the setup screen stays. True if it is shown.
    pub(crate) fn open_front(&mut self)->bool{
        let ready=!self.dirty&&self.path.exists()&&self.program().is_some()&&self.doc.check(self.parent()).is_ok();
        if ready{self.front=Some(self.doc.text());self.on_front=true;}
        ready
    }
    /// After a run: the start page again, if there is one.
    pub(crate) fn back_to_front(&mut self){self.on_front=self.front.is_some();}
    #[cfg(test)]
    pub(crate) fn on_front(&self)->bool{self.on_front}
    fn program(&self)->Option<String>{self.doc.get("program").filter(|p|!p.is_empty())}
    fn parent(&self)->&Path{
        self.path.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."))
    }
    pub(crate) fn path(&self)->&Path{&self.path}
    pub(crate) fn dirty(&self)->bool{self.dirty}
    #[cfg(test)]
    pub(crate) fn status(&self)->&str{&self.status.0}
    pub(crate) fn report(&mut self,text:String,error:bool){self.status=(text,error);}

    /// What the selected field shows when it is not being edited.
    fn shown(&self,index:usize)->(String,bool){
        let f=&FIELDS[index];
        let value=self.doc.get(f.key).filter(|v|!v.is_empty());
        match f.kind{
            Kind::Bool=>{
                let on=value.as_deref().is_some_and(|v|matches!(v.to_ascii_lowercase().as_str(),"true"|"1"));
                (format!("[{}] {}",if on{'X'}else{' '},if on{"on"}else{"off"}),value.is_some())
            },
            Kind::Choice(_)=>{
                let name=|v:&str|match (f.key,v){("country","1")=>"USA",("country","358")=>"Finland",("fpu","none")=>"no coprocessor",("fpu","80387")=>"partial 80387",("host_pointer","auto")=>"hidden while the DOS cursor shows",("host_pointer","show")=>"always shown",("host_pointer","hide")=>"always hidden",("own_programs","none")=>"never updated",("own_programs",_)=>"updated from programs/dos",_=>""};
                match value{
                    Some(v)=>{let n=name(&v);(if n.is_empty(){v}else{format!("{v}  {n}")},true)},
                    None=>(match f.key{"country"=>"(default: 1 USA)","fpu"=>"(default: none)","host_pointer"=>"(default: auto)","own_programs"=>"(default: D)",_=>"(default)"}.to_string(),false),
                }
            },
            Kind::Text=>match value{
                Some(v)=>(v,true),
                None=>(match f.key{"utc_offset"=>"(this computer's time zone)","speaker_volume"=>"(default: 5)","engine"=>"(newest build)",_=>"(not set)"}.to_string(),false),
            },
        }
    }

    /// The start page: what will run, and the keys.
    fn render_front(&self)->Vec<u8>{
        let mut cells=vec![0u8;COLUMNS*ROWS*2];
        for row in 0..ROWS{put(&mut cells,row,0,COLUMNS,"",SCREEN);}
        let title=format!(" PILdos   File: {}",tail(&self.path.display().to_string(),COLUMNS-17));
        put(&mut cells,0,0,COLUMNS,&title,TITLE);
        let line:String=std::iter::repeat('\u{2500}').take(COLUMNS).collect();
        put(&mut cells,1,0,COLUMNS,&line,SCREEN);
        let mut rows:Vec<(&str,String)>=vec![("Program",self.program().unwrap_or_default())];
        if let Some(args)=self.doc.get("args").filter(|a|!a.is_empty()){rows.push(("Command line",args));}
        for (key,label) in [("a","Drive A:"),("b","Drive B:"),("c","Drive C:"),("d","Drive D:"),("e","Drive E:")]{
            if let Some(v)=self.doc.get(key).filter(|v|!v.is_empty()){rows.push((label,v));}
        }
        let mut row=4;
        for (label,value) in rows{
            put(&mut cells,row,LABEL_COLUMN,VALUE_COLUMN-LABEL_COLUMN-1,label,SCREEN);
            put(&mut cells,row,VALUE_COLUMN,VALUE_WIDTH,&format!(" {}",tail(&value,VALUE_WIDTH-1)),VALUE);
            row+=1;
        }
        let prompt="Press Enter to start.";
        put(&mut cells,row+2,(COLUMNS-prompt.len())/2,prompt.len(),prompt,LABEL_SELECTED);
        let (status,error)=&self.status;
        put(&mut cells,ROWS-2,0,COLUMNS,&format!(" {}",tail(status,COLUMNS-1)),if *error{STATUS_ERROR}else{STATUS_OK});
        put(&mut cells,ROWS-1,0,COLUMNS," Enter Start   S Setup   Esc Quit",KEYS);
        cells
    }
    /// The screen: character/attribute cells and the cursor (row, column).
    pub(crate) fn render(&self)->(Vec<u8>,Option<(u32,u32)>){
        if self.on_front{return (self.render_front(),None);}
        let mut cells=vec![0u8;COLUMNS*ROWS*2];
        for row in 0..ROWS{put(&mut cells,row,0,COLUMNS,"",SCREEN);}
        // Title and file share the top row, so every field fits above the help.
        let changed=if self.dirty{"  (changed)"}else{""};
        let title=format!(" PILdos setup   File: {}{changed}",tail(&self.path.display().to_string(),COLUMNS-35));
        put(&mut cells,0,0,COLUMNS,&title,TITLE);
        let line:String=std::iter::repeat('\u{2500}').take(COLUMNS).collect();
        put(&mut cells,1,0,COLUMNS,&line,SCREEN);
        // The rule under the fields, when it does not take the help row.
        if FIRST_FIELD+FIELDS.len()<ROWS-3{put(&mut cells,FIRST_FIELD+FIELDS.len(),0,COLUMNS,&line,SCREEN);}
        let mut cursor=None;
        for (i,f) in FIELDS.iter().enumerate(){
            let row=FIRST_FIELD+i;
            let here=i==self.selected;
            let label=format!("{}{}",f.label,if f.terminal_only{" *"}else{""});
            put(&mut cells,row,LABEL_COLUMN,VALUE_COLUMN-LABEL_COLUMN-1,&label,if here{LABEL_SELECTED}else{SCREEN});
            if here&&self.editing.is_some(){
                let e=self.editing.as_ref().unwrap();
                let start=e.cursor.saturating_sub(VALUE_WIDTH-1);
                let text:String=e.text.iter().skip(start).collect();
                put(&mut cells,row,VALUE_COLUMN,VALUE_WIDTH,&text,VALUE_EDITING);
                cursor=Some((row as u32,(VALUE_COLUMN+e.cursor-start) as u32));
                continue;
            }
            let (text,set)=self.shown(i);
            let text=if here&&matches!(f.kind,Kind::Choice(_)){format!("< {text} >")}else{text};
            let attr=if here{VALUE_SELECTED}else if set{VALUE}else{VALUE_UNSET};
            put(&mut cells,row,VALUE_COLUMN,VALUE_WIDTH,&format!(" {}",tail(&text,VALUE_WIDTH-1)),attr);
        }
        let f=&FIELDS[self.selected];
        let help=format!(" {}{}",f.help,if f.terminal_only{"  * Python terminal only."}else{""});
        put(&mut cells,ROWS-3,0,COLUMNS,&tail(&help,COLUMNS),SCREEN);
        let (status,error)=&self.status;
        put(&mut cells,ROWS-2,0,COLUMNS,&format!(" {}",tail(status,COLUMNS-1)),if *error{STATUS_ERROR}else{STATUS_OK});
        let keys=if self.editing.is_some(){" Enter Accept   Esc Cancel   Left/Right Home/End Move   Backspace/Del Erase"}
            else if self.edit_only{" \u{18}\u{19} Select  Enter Edit  Del Clear  F2 Save  F10 Save and close  Esc Quit"}
            else if self.front.is_some(){" \u{18}\u{19} Select  Enter Edit  Del Clear  F2 Save  F10 Save and start  Esc Back"}
            else{" \u{18}\u{19} Select  Enter Edit  Del Clear  F2 Save  F10 Save and start  Esc Quit"};
        put(&mut cells,ROWS-1,0,COLUMNS,keys,KEYS);
        (cells,cursor)
    }

    fn mark(&mut self){self.dirty=true;self.quit_armed=false;}
    /// The next (or previous) value of a choice; a boolean toggles, and
    /// "not set" counts as off. The line stays where it is.
    fn cycle(&mut self,forward:bool){
        let f=FIELDS[self.selected];
        let current=self.doc.get(f.key).unwrap_or_default().to_ascii_lowercase();
        let value=match f.kind{
            Kind::Bool=>if matches!(current.as_str(),"true"|"1"){"false"}else{"true"},
            Kind::Choice(values)=>{
                // From "not set", the first value (or the last going back).
                match values.iter().position(|v|*v==current){
                    Some(at)=>values[if forward{(at+1)%values.len()}else{(at+values.len()-1)%values.len()}],
                    None=>values[if forward{0}else{values.len()-1}],
                }
            },
            Kind::Text=>return,
        };
        self.doc.set(f.key,Some(value));
        self.mark();
    }
    fn begin_edit(&mut self){
        let text:Vec<char>=self.doc.get(FIELDS[self.selected].key).unwrap_or_default().chars().collect();
        self.editing=Some(Editing{cursor:text.len(),text});
    }
    /// Store the edited value; false (and an error line) if it is refused.
    fn commit(&mut self)->bool{
        let Some(e)=self.editing.as_ref() else{return true};
        let f=FIELDS[self.selected];
        let value:String=e.text.iter().collect();
        let value=value.trim().to_string();
        if !value.is_empty(){
            if let Err(err)=edit::check_value(f.key,&value){self.status=(err,true);return false;}
        }
        if self.doc.get(f.key).unwrap_or_default()!=value{
            self.doc.set(f.key,(!value.is_empty()).then_some(value.as_str()));
            self.mark();
        }
        self.editing=None;
        self.status=match self.missing(f.key){
            Some(path)=>(format!("Note: {path} does not exist (yet)."),false),
            None=>(String::new(),false),
        };
        true
    }
    /// The host path a program or drive setting names, if it is not there.
    fn missing(&self,key:&str)->Option<String>{
        if !(key=="program"||(key.len()==1&&key.as_bytes()[0].is_ascii_lowercase())){return None;}
        let value=self.doc.get(key)?;
        let host=crate::containers::split_container_path(&value).map_or(value.as_str(),|(outer,_)|outer);
        let path=dos_config::host_path(host);
        let full=if path.is_absolute(){path}else{self.parent().join(path)};
        (!full.exists()).then(||full.display().to_string())
    }
    /// Write the document after the parser accepts it.
    pub(crate) fn save(&mut self)->bool{
        if let Err(e)=self.doc.check(self.parent()){self.status=(e,true);return false;}
        match std::fs::write(&self.path,self.doc.text()){
            Ok(())=>{
                self.dirty=false;self.quit_armed=false;self.status=(format!("Saved {}.",self.path.display()),false);
                if self.front.is_some(){self.front=Some(self.doc.text());}
                true
            },
            Err(e)=>{self.status=(format!("cannot write {}: {e}",self.path.display()),true);false},
        }
    }
    fn move_by(&mut self,delta:isize){
        let n=FIELDS.len() as isize;
        self.selected=((self.selected as isize+delta).rem_euclid(n)) as usize;
    }

    /// One key on the screen.
    pub(crate) fn key(&mut self,key:Key,m:Mods)->Action{
        if self.on_front{
            match key{
                Key::Enter|Key::F(10)=>return self.start(),
                Key::Escape=>return Action::Quit,
                Key::F(2)|Key::Tab=>self.on_front=false,
                Key::Char('s'|'S') if !m.ctrl&&!m.alt=>self.on_front=false,
                _=>{},
            }
            if !self.on_front{self.status=(String::new(),false);}
            return Action::Stay;
        }
        if let Some(e)=self.editing.as_mut(){
            match key{
                Key::Char(c) if !m.ctrl&&!m.alt&&!c.is_control()=>{e.text.insert(e.cursor,c);e.cursor+=1;},
                Key::Backspace=>if e.cursor>0{e.cursor-=1;e.text.remove(e.cursor);},
                Key::Delete=>if e.cursor<e.text.len(){e.text.remove(e.cursor);},
                Key::Left=>e.cursor=e.cursor.saturating_sub(1),
                Key::Right=>e.cursor=(e.cursor+1).min(e.text.len()),
                Key::Home=>e.cursor=0,
                Key::End=>e.cursor=e.text.len(),
                Key::Escape=>{self.editing=None;self.status=(String::new(),false);},
                Key::Enter=>{self.commit();},
                Key::Up=>{if self.commit(){self.move_by(-1);}},
                Key::Down|Key::Tab=>{if self.commit(){self.move_by(if m.shift{-1}else{1});}},
                Key::F(2)=>{if self.commit(){self.save();}},
                Key::F(10)=>{if self.commit(){return self.start();}},
                _=>{},
            }
            return Action::Stay;
        }
        let kind=FIELDS[self.selected].kind;
        match key{
            Key::Up=>self.move_by(-1),
            Key::Down=>self.move_by(1),
            Key::Tab=>self.move_by(if m.shift{-1}else{1}),
            Key::Home|Key::PageUp=>self.selected=0,
            Key::End|Key::PageDown=>self.selected=FIELDS.len()-1,
            Key::Enter|Key::Char(' ') if kind!=Kind::Text=>self.cycle(true),
            Key::Right if kind!=Kind::Text=>self.cycle(true),
            Key::Left if kind!=Kind::Text=>self.cycle(false),
            Key::Enter=>self.begin_edit(),
            Key::Char(c) if kind==Kind::Text&&!m.ctrl&&!m.alt&&!c.is_control()=>{
                self.begin_edit();
                if let Some(e)=self.editing.as_mut(){e.text.insert(e.cursor,c);e.cursor+=1;}
            },
            Key::Delete|Key::Backspace=>{
                let key=FIELDS[self.selected].key;
                if self.doc.get(key).is_some(){self.doc.set(key,None);self.mark();}
            },
            Key::F(2)=>{self.save();},
            Key::F(10)=>return self.start(),
            Key::Escape=>{
                if let Some(saved)=self.front.as_ref().filter(|_|!self.dirty||self.quit_armed){
                    // Back to the start page, with the file as saved.
                    let discarded=self.dirty;
                    self.doc=Document::parse(saved);
                    self.dirty=false;self.quit_armed=false;self.on_front=true;
                    self.status=(if discarded{"Changes not saved."}else{""}.into(),false);
                    if self.program().is_none(){return Action::Quit;}
                    return Action::Stay;
                }
                if !self.dirty||self.quit_armed{return Action::Quit;}
                self.quit_armed=true;
                let then=if self.front.is_some(){"goes back"}else{"quits"};
                self.status=(format!("Unsaved changes: F2 saves them, Esc again {then} without saving."),true);
            },
            _=>{},
        }
        Action::Stay
    }
    fn start(&mut self)->Action{
        if !self.edit_only&&self.doc.get("program").filter(|p|!p.is_empty()).is_none(){
            self.selected=0;self.status=("Set the program to start first.".into(),true);
            return Action::Stay;
        }
        if self.dirty&&!self.save(){return Action::Stay;}
        Action::Start
    }
}

/// Start the configured program in `front` until it ends; the closing line.
fn launch(path:&Path,window:bool,stop_log:&str,front:&mut Front,rx:&mpsc::Receiver<Event>)->Result<String,String>{
    // Replace the setup page before synchronous program/drive loading starts.
    let mut cells=vec![0;COLUMNS*ROWS*2];
    put(&mut cells,0,0,COLUMNS," PILdos",TITLE);
    put(&mut cells,2,2,COLUMNS-2,"Starting DOS...",VALUE);
    put(&mut cells,4,2,COLUMNS-2,"Loading the program and drives.",VALUE);
    front.present_text(&cells,None)?;
    let started=std::time::Instant::now();
    let config=path.to_str().ok_or("the configuration path is not Unicode")?;
    let (program,args)=dos_config::startup(Some(config))?;
    let settings=dos_config::console_settings(Some(config))?;
    let fpu=settings.fpu.map(|on|if on{"80387"}else{"none"});
    let mut session=super::session::open(&program,None,args.as_deref(),Some(config),fpu)?;
    #[cfg(windows)]
    crate::dine_display::debug_log(&format!("PILdos program and drives loaded in {:.1} ms",started.elapsed().as_secs_f64()*1000.0));
    #[cfg(not(windows))]
    let _=started;
    let mut logger=super::stoplog::Logger::new(stop_log,&program);
    let options=Options{window,mouse:settings.mouse,ansi:settings.ansi,country:None,utc_offset:settings.utc_offset,host_pointer:settings.host_pointer.unwrap_or_default(),speaker_volume:settings.speaker_volume.unwrap_or(super::host_console::speaker::DEFAULT_VOLUME),rtc_year:None,guest_keymap:None};
    let result=host_console::run_on(&mut session,&options,front,rx);
    session.log_stop(&mut logger);
    let line=match &result{Ok(line)=>line.clone(),Err(e)=>e.clone()};
    if !line.contains("closed"){pause(&session,&line,front,rx)?;}
    result
}
/// Keep the program's last screen until a key, with `line` at the bottom.
fn pause(session:&super::session::Session,line:&str,front:&mut Front,rx:&mpsc::Receiver<Event>)->Result<(),String>{
    let text_page=session.machine().is_some_and(|p|!host_console::graphics_mode(p));
    if text_page{
        if let Ok((mut cells,80,_))=session.screen(){
            cells.resize(COLUMNS*ROWS*2,0);
            put(&mut cells,ROWS-1,0,COLUMNS,&format!(" {}  -  press a key",tail(line,COLUMNS-20)),KEYS);
            front.present_text(&cells,None)?;
        }
    }
    loop{
        if front.closed(){return Ok(());}
        let mut events:Vec<Event>=rx.try_iter().collect();
        front.poll(&mut events)?;
        if events.iter().any(|e|matches!(e,Event::Key(..)|Event::NativeKey{down:true,key:Some(_),..}|Event::Quit)){return Ok(());}
        if events.is_empty(){
            if let Ok(event)=rx.recv_timeout(Duration::from_millis(30)){if matches!(event,Event::Key(..)|Event::NativeKey{down:true,key:Some(_),..}|Event::Quit){return Ok(());}}
        }
    }
}

/// `pil --pildos [--tty|--window] [CONFIG]`: the setup screen, until Esc.
pub(crate) fn run(explicit:Option<&str>,window:bool,stop_log:&str)->Result<String,String>{
    run_mode(explicit,window,stop_log,false)
}
#[cfg_attr(not(feature="dine-library"),allow(dead_code))]
pub(crate) fn edit(explicit:Option<&str>,window:bool)->Result<String,String>{
    run_mode(explicit,window,"",true)
}
fn run_mode(explicit:Option<&str>,window:bool,stop_log:&str,edit_only:bool)->Result<String,String>{
    let (path,_)=dos_config::selected_path(explicit);
    let mut setup=Setup::load(path)?;
    setup.edit_only=edit_only;
    if !edit_only{setup.open_front();}
    // The project's own programs on the drive follow programs/dos.
    if let Some(config)=setup.path().to_str().filter(|_|setup.path().exists()){
        match dos_config::refresh_own_programs(Some(config)){
            Ok(Some(line))=>{let error=line.contains("could not");setup.report(line,error);},
            Ok(None)|Err(_)=>{},
        }
    }
    let (tx,rx)=mpsc::channel();
    let mut front=Front::open(window,tx)?;
    front.reset();
    loop{
        let (cells,cursor)=setup.render();
        front.present_text(&cells,cursor)?;
        if front.closed(){return Ok("PILdos setup closed".into());}
        let mut events:Vec<Event>=rx.try_iter().collect();
        front.poll(&mut events)?;
        if events.is_empty(){
            if let Ok(event)=rx.recv_timeout(Duration::from_millis(30)){events.push(event);}
            front.poll(&mut events)?;
        }
        for event in events{
            let action=match event{
                Event::Key(key,m)=>setup.key(key,m),
                Event::NativeKey{down:true,key:Some((key,m)),..}=>setup.key(key,m),
                Event::NativeKey{..}=>Action::Stay,
                Event::Quit=>Action::Quit,
                Event::Break|Event::Mouse{..}|Event::Copy|Event::Paste|Event::HostMenu|Event::HostCommand(_)=>Action::Stay,
            };
            match action{
                Action::Stay=>{},
                Action::Quit=>return Ok(if setup.dirty(){"PILdos setup: quit without saving".into()}else{"PILdos setup: done".into()}),
                Action::Start=>{
                    if edit_only{return Ok("Dine settings saved".into());}
                    let (text,error)=match launch(setup.path(),window,stop_log,&mut front,&rx){
                        Ok(line)=>(line,false),
                        Err(e)=>(e,true),
                    };
                    // Keys typed for the program stay with it.
                    while rx.try_recv().is_ok(){}
                    setup.report(text,error);
                    setup.back_to_front();
                    front.reset();
                    break;
                },
            }
        }
    }
}
