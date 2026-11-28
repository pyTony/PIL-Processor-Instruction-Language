//! Host-side software catalog. Guest paths and descriptions are data, not engine rules.
use super::*;
#[path="catalog_instance.rs"]
mod instance;
/// Preserve configured drives; supply the conventional D: only when absent.
pub(super) fn open_program(shell:&str,tail:Option<&str>,config:Option<&str>)->Result<Session,String>{
    let defaults=crate::dos_config::console_settings(config)?;
    let fpu=defaults.fpu.map(|present|if present{"80387"}else{"none"});
    let cpu=defaults.cpu.map(|c|c.name());
    let mut session=crate::dos::session::open(shell,None,tail,config,fpu,cpu)?;
    if !session.machine().ok_or("session state lost")?.dos.drive.is_mounted(b'D')&&std::path::Path::new("driveD").is_dir(){
        session.mount_media(b'D',"driveD")?;
    }
    Ok(session)
}

/// Parse a catalog: one entry per line, six tab-separated fields
/// (group, title, description, folder, D:\...BAT launcher, reserved).
/// Blank lines and lines starting with `#` are skipped.
pub(super) fn parse(text:&str)->Result<Vec<Vec<&str>>,String>{
    let text=text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut entries=Vec::new();
    for (number,line) in text.lines().enumerate(){
        let line=line.strip_suffix('\r').unwrap_or(line);
        if line.trim().is_empty()||line.starts_with('#'){continue;}
        let mut fields:Vec<_>=line.split('\t').collect();
        // The sixth field is reserved; an editor that trims the trailing tab leaves five.
        if fields.len()==5{fields.push("");}
        if fields.len()!=6{return Err(format!("line {}: {} tab-separated fields, a catalog entry needs 6",number+1,fields.len()));}
        if fields.iter().any(|s|s.chars().any(|c|c.is_control())){return Err(format!("line {}: control character in a field",number+1));}
        if entries.len()>=200{return Err(format!("line {}: more than 200 entries",number+1));}
        entries.push(fields);
    }
    if entries.is_empty(){return Err("no entries (only blank or # lines)".into());}
    Ok(entries)
}

/// The catalog is a host UI, not a DOS guest. The native window reports
/// translated Windows keys as NativeKey edges, whereas terminal mode reports
/// Key events. Consume only the press, never the release, in this host menu.
fn catalog_event(event:Event)->Event{
    match event{
        Event::NativeKey{down:true,key:Some((key,mods)),..}=>Event::Key(key,mods),
        other=>other,
    }
}

/// Activate the row only when two distinct mouse presses target the same item.
/// A single press continues to select, and mouse movement never launches it.
fn double_click(last:Option<(usize,std::time::Instant)>,row:usize,now:std::time::Instant)->bool{
    last.is_some_and(|(previous,at)|previous==row&&now.saturating_duration_since(at)<=Duration::from_millis(500))
}

pub(crate) fn run(shell:&str,catalog:&str)->Result<(),String>{
    let text=std::fs::read_to_string(catalog).map_err(|e|format!("catalog {catalog}: {e}"))?;
    let entries=parse(&text).map_err(|e|format!("catalog {catalog}: {e}"))?;
    let Some(_instance)=instance::Instance::acquire()? else{return Ok(())};
    let (tx,rx)=mpsc::channel();let mut front=Front::open(cfg!(windows),tx)?;
    let mut selected=0usize;let mut notice=String::new();
    let mut mouse_left_down=false;
    let mut last_click:Option<(usize,std::time::Instant)>=None;
    loop{
        front.title("PIL Retro Browser");
        let mut cells=vec![0u8;80*25*2];for cell in cells.chunks_mut(2){cell[0]=b' ';cell[1]=7;}
        let mut put=|row:usize,s:&str,attr:u8|{for (i,b) in s.bytes().take(78).enumerate(){cells[(row*80+i+1)*2]=if b.is_ascii(){b}else{b'?'};cells[(row*80+i+1)*2+1]=attr;}};
        put(0,"RETRO SOFTWARE BROWSER - host environment",0x1f);
        put(1,"Up/Down: select   Enter: launch   Esc: close   Click a row: select",7);
        let start=selected/16*16;
        for (row,entry) in entries.iter().enumerate().skip(start).take(16){
            put(3+row-start,&format!("{}  {}",entry[0],entry[1]),if row==selected{0x70}else{7});
        }
        let entry=&entries[selected];
        put(20,entry[2],0x0f);put(21,&format!("Folder: {}",entry[3]),7);
        let short=notice.strip_prefix("DOS session stopped after ").unwrap_or(&notice);
        let short=short.split_once("steps: ").map_or(short,|(_,detail)|detail);
        put(22,&short.chars().take(78).collect::<String>(),0x0e);
        put(23,&short.chars().skip(78).take(78).collect::<String>(),0x0e);
        put(24,"During DOS: Ctrl+Shift+M opens host drive controls.",7);
        front.present_text(&cells,None)?;
        let mut events=Vec::new();if let Ok(event)=rx.recv_timeout(Duration::from_millis(30)){events.push(event);}
        front.poll(&mut events)?;
        // Closing the native window sets its closed flag without generating
        // a guest key. Stop the host catalog loop as the DOS console does.
        if front.closed(){return Ok(());}
        let mut launch=false;
        for event in events{match catalog_event(event){
            Event::Quit|Event::Key(Key::Escape,_)=>return Ok(()),
            Event::Key(Key::Up,_)=>selected=selected.saturating_sub(1),
            Event::Key(Key::Down,_)=>selected=(selected+1).min(entries.len()-1),
            Event::Key(Key::PageUp,_)=>selected=selected.saturating_sub(16),
            Event::Key(Key::PageDown,_)=>selected=(selected+16).min(entries.len()-1),
            Event::Key(Key::Enter,_)|Event::Key(Key::Char('\r'),_)=>launch=true,
            Event::Mouse{y,height,buttons,..}=>{
                let down=buttons&1!=0;
                if down&&!mouse_left_down&&height>0{
                    let row=(y.max(0) as usize*25/height as usize).saturating_sub(3);
                    if row<16&&start+row<entries.len(){
                        let index=start+row;
                        let now=std::time::Instant::now();
                        if double_click(last_click,index,now){
                            selected=index;launch=true;last_click=None;
                        }else{
                            selected=index;last_click=Some((index,now));
                        }
                    }else{last_click=None;}
                }
                mouse_left_down=down;
            },_=>{},
        }}
        if launch{
            last_click=None;
            mouse_left_down=false;
            let entry=&entries[selected];
            // The catalog supplies the shell batch path, never executable host code.
            let batch=entry[4];
            if !batch.starts_with("D:\\")||!batch.to_ascii_uppercase().ends_with(".BAT")||batch.chars().any(|c|"&|<>\"".contains(c)){
                notice="Invalid catalog DOS launcher path".into();continue;
            }
            let result=open_program(shell,Some(&format!("/C {batch}")),None);
            match result{
                Ok(mut session)=>{
                    front.reset();
                    let settings=crate::dos_config::console_settings(None)?;
                    let options=Options{window:cfg!(windows),mouse:settings.mouse,ansi:settings.ansi,wait_on_exit:settings.wait_on_exit,country:None,utc_offset:settings.utc_offset,host_pointer:settings.host_pointer.unwrap_or_default(),speaker_volume:settings.speaker_volume.unwrap_or(speaker::DEFAULT_VOLUME),rtc_year:None,guest_keymap:None};
                    notice=run_on(&mut session,&options,&mut front,&rx).unwrap_or_else(|e|e);
                    session.log_stop(&mut crate::dos::stoplog::Logger::new("PILdos-stops.jsonl",shell));
                    if front.closed(){return Ok(());}front.reset();
                },Err(e)=>notice=e,
            }
        }
    }
}

#[cfg(test)]
mod catalog_input_tests{
    use super::*;

    #[test]
    fn native_window_keys_control_host_catalog(){
        let cases=[(0x48,Key::Up),(0x50,Key::Down),(0x49,Key::PageUp),
                   (0x51,Key::PageDown),(0x1c,Key::Enter),(0x01,Key::Escape)];
        for (scan,key) in cases{
            let mods=Mods::default();
            let press=Event::NativeKey{scan,extended:false,down:true,key:Some((key,mods))};
            assert_eq!(catalog_event(press),Event::Key(key,mods));
            let release=Event::NativeKey{scan,extended:false,down:false,key:None};
            assert_eq!(catalog_event(release),release);
        }
        let unknown=Event::NativeKey{scan:0x2a,extended:false,down:true,key:None};
        assert_eq!(catalog_event(unknown),unknown);
    }

    #[test]
    fn mouse_activates_only_on_double_click_of_same_catalog_row(){
        let now=std::time::Instant::now();
        assert!(!double_click(None,3,now));
        assert!(double_click(Some((3,now)),3,now+Duration::from_millis(250)));
        assert!(!double_click(Some((3,now)),4,now+Duration::from_millis(250)));
        assert!(!double_click(Some((3,now)),3,now+Duration::from_millis(600)));
    }
}
