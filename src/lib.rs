//! PIL and Dine: DOS runtime, host front ends and command-line implementation
//! over the CPU core (`crates/pil-cpu`, re-exported here as the crate root),
//! the PC machine (`crates/pil-pc`, module `pc`) and the BIOS (`crates/pil-bios`,
//! module `bios`).
pub(crate) use pil_cpu::*;
mod serial;
mod host_io;
mod debugger;
pub(crate) use pil_pc as pc;
pub(crate) use pil_bios as bios;
mod dos;
#[allow(unused_imports)]
use dos::LoadedOps;
#[cfg(any(feature = "dine-library",test))]
mod dine_ffi;
// Win32 window presentation for --dos-session (DINE_VIDEO=window).
#[cfg(windows)]
mod dine_display;
// Window code is Windows-only inside; the layout helpers are tested everywhere.
mod dine_window;
// X11 window (Linux, Termux:X11) for --dos-console --window; key translation
// is pure and tested everywhere.
#[cfg(all(unix,feature = "x11"))]
mod dine_x11;
#[cfg(any(all(unix,feature = "x11"),test))]
mod dine_x11_keys;
#[cfg(any(all(unix,feature = "x11"),test))]
mod dine_x11_menu;
mod block_compiler;
mod native_application;
#[path="dos/encoding.rs"]
mod dos_encoding;
#[path="dos/text_io.rs"]
mod dos_text_io;
#[path="host/config.rs"]
mod dos_config;
#[path="host/own_programs.rs"]
mod dos_own_programs;
mod containers;
mod inflate;
mod native;
mod mandelbrot;

/// The machines the precompiled forms are enumerated in: 16-bit real mode,
/// 32-bit protected mode with a 32-bit stack, 16-bit ring-3 protected mode,
/// and real mode with a 32-bit stack.
fn form_machines()->Result<Vec<(guest::Image,Ram)>,String>{
    let p=dos::load(&[0xcd,0x20],"FORMS.COM")?;
    let mut real16=p.image.clone();real16.end=0xff00;
    let code_base=real16.segments[1].base;
    let mut pm32=real16.clone();
    pm32.real_mode=false;pm32.msw=0x11;pm32.controls[0]=0x11;
    let code32=address::SegmentCache{base:code_base,limit:0xffff_ffff,access:address::SegmentAccess::Code{readable:true,conforming:false,big:true,dpl:0}};
    let data32=address::SegmentCache{base:0,limit:0xffff_ffff,access:address::SegmentAccess::Data{writable:true,expand_down:false,big:true,dpl:0}};
    pm32.segments=[data32,code32,data32,data32,data32,data32];
    pm32.selectors=[0x10,0x08,0x10,0x10,0x10,0x10];
    let mut pm16=pm32.clone();
    let code16=address::SegmentCache{base:code_base,limit:0xffff,access:address::SegmentAccess::Code{readable:true,conforming:false,big:false,dpl:3}};
    let data16=address::SegmentCache{base:0,limit:0xffff,access:address::SegmentAccess::Data{writable:true,expand_down:false,big:false,dpl:3}};
    pm16.segments=[data16,code16,data16,data16,data16,data16];
    pm16.selectors=[0x13,0x0b,0x13,0x13,0x13,0x13];
    let mut real32_stack=real16.clone();
    real32_stack.segments[2]=data32;
    Ok(vec![
        (real16,p.ram.clone()),
        (pm32,p.ram.clone()),
        (pm16,p.ram.clone()),
        (real32_stack,p.ram),
    ])
}

const CLI_HELP:&str = "PIL - Processor Instruction Language
Usage: pil [OPTIONS] COMMAND [ARGUMENTS]

DOS and ROM:
  --pildos [--tty|--window] [CONFIG]  PILdos setup screen: edit pildos.conf
      (or CONFIG, or PILDOS_CONFIG) and start its program. With a program set
      it opens on a start page: Enter starts, S opens the setup screen.
  --pildos-startup [CONFIG]          Read configured program and arguments as JSON
  --dos-session [--config FILE] [--mount DIR] [--args TEXT] [--fpu none|80387] [--cpu 386|486dx] PROGRAM
  --dos-media SHELL_PROGRAM [IMAGE_OR_DISK_SET]  Native Windows media browser
      Interactive JSON service; PROGRAM can be IMAGE.img::INNER_PATH.
  --dos-text [--plain] [OPTIONS] [PROGRAM]  Rust terminal; config startup if omitted
  --dos-console [--tty|--window] [--mouse] [--ansi] [--host-pointer auto|show|hide] [OPTIONS] [PROGRAM]
      DOS session with its own keyboard and screen: the host terminal
      (--tty, default except on Windows) or a window (--window: Win32 on
      Windows; on Linux and Termux:X11 an X11 window, in a build made with
      --features x11).
      Ctrl+] in a terminal ends it. Same OPTIONS as --dos-session, plus
      --country 1|358 and --utc-offset SECONDS (default: host time zone).
      --rtc-year YEAR (1980-2099) runs the RTC and DOS date in that year
      from today's date and time, for systems that cannot read this
      century (Linux 0.99). --boot IMAGE, --boot-hd IMAGE and --hd-image
      DISK start a disk instead of DOS (docs/BOOT.md); --hd-folder DIR
      adds a host folder as a FAT16 fixed disk, changes copied back;
      --guest-keymap us|uk|fi is the booted system's keyboard layout for
      pasted text.
      Without PROGRAM, the configuration's program and args start; its
      fpu, mouse, ansi, utc_offset and host_pointer apply unless options
      are given. --host-pointer: the window's own arrow is hidden while the
      DOS mouse cursor shows (auto, default), never (show) or always (hide).
  --dos-probe [--steps N] PROGRAM...   Bounded DOS compatibility probe
  --run-rom PATH [--steps N]          Run a 64 or 128 KiB ROM
  --dos-list [--all] IMAGE|ZIP        List container entries
  --dos-read IMAGE|ZIP INNER [DEST]   Read/extract one entry
  --dos-extract IMAGE|ZIP NEW_DIR     Extract a container
  --dos-unpack-drive IMAGE NEW_DIR    Copy a FAT image into a new writable folder
  --dos-drive-name IMAGE              Suggest a labelled host-folder name

Debugger and checks:
  --repl [--load PATH]               Interactive debugger
  --debug-service [--load PATH]      JSON debugger service
  --selftest                        Run built-in execution checks
  --numeric-test                    Check numeric instructions
  --verify-backends                 Compare execution backends
  --compare-backends [N] [SAMPLES]   Compare and benchmark backends
  --bench [N] [SAMPLES] [--uncached] Interpreter benchmark
  --cpu-bench [N] [SAMPLES]          Native CPU benchmark
  --stress [PASSES]                 Factorial stress check
  --write-dos-mandel PATH            Write VGA Mandelbrot DOS program
  --mandelbrot [SIZE] [ITERATIONS]   Render Mandelbrot
  --compare-mandelbrot [SIZE] [ITERATIONS]

Code generation and examples:
  --emit-native-application INPUT OUTPUT.rs
                                    Prepare native application calls and relocated startup
  --emit-native-pil INPUT OUTPUT.rs
                                    Emit whole-block PIL-lowered Rust calling shared Dine library
  --emit-rust 8|16|32 [DESCRIPTION]  Emit a Rust instruction implementation
  --emit-suite-rust [N] [SAMPLES]    Emit backend verification suite
  --emit-loop-rust [N] [SAMPLES]     Emit small native loop benchmark
  --write-selftest PATH             Write a raw example
  --write-hello PATH                Write a raw hello example
  --write-factorials PATH           Write a factorial example
  --write-loop PATH                 Write a loop example
  --write-dos-hello com|exe PATH     Write a DOS hello (new file only)
  --demo-branch                     Show a conditional branch example

Options (before COMMAND):
  --stop-log PATH                   Append DOS-session/ROM diagnostics (default: PILdos-stops.jsonl)
  --profile JSON_PATH               Write DOS host timings on normal exit
  --ignore-unassigned-out           Ignore/log unassigned OUT ports
  --help, -h                        Show this help

For a DOS window use: python tools/dos_terminal.py --help
";
/// Run the PIL command line using the shared CPU and DOS implementation.
pub fn run_cli() -> Result<(), String> {
    let mut dos_args:Vec<_>=std::env::args().collect();
    if dos_args.get(1).map(String::as_str)==Some("--pildos-startup"){
        if dos_args.len()>3{return Err("usage: pil --pildos-startup [CONFIG]".into());}
        let(program,args)=dos_config::startup(dos_args.get(2).map(String::as_str))?;
        println!("{{\"program\":{},\"args\":{}}}",dos::session::quote(&program),args.as_deref().map(dos::session::quote).unwrap_or_else(||"null".into()));return Ok(());
    }
    if dos_args.len()==1 || matches!(dos_args.get(1).map(String::as_str),Some("--help"|"-h")){
        if dos_args.len()>2{return Err("--help accepts no arguments".into());}
        print!("{CLI_HELP}");return Ok(());
    }
    let mut _profile=None;
    let mut stop_log=String::from("PILdos-stops.jsonl");
    let mut log_given=false;
    loop {match dos_args.get(1).map(String::as_str){
        Some("--stop-log")=>{if log_given{return Err("duplicate --stop-log".into());}stop_log=dos_args.get(2).ok_or("--stop-log requires a path (empty disables logging)")?.clone();log_given=true;dos_args.drain(1..3);},
        Some("--profile")=>{if _profile.is_some(){return Err("duplicate --profile".into());}let path=dos_args.get(2).ok_or("--profile requires an output JSON path")?;_profile=Some(profile::start(path)?);dos_args.drain(1..3);},
        Some("--ignore-unassigned-out")=>{pc::ports::IGNORE_UNASSIGNED_OUT.store(true,std::sync::atomic::Ordering::Relaxed);dos_args.remove(1);},
        _=>break,
    }}
    if dos_args.get(1).map(String::as_str)==Some("--dos-list"){
        if dos_args.len()<3||dos_args.len()>4{return Err("usage: pil --dos-list [--all] IMAGE.img|ARCHIVE.zip".into());}
        let(all,path)=if dos_args[2]=="--all"{(true,dos_args.get(3).ok_or("missing image path")?)}else{(false,&dos_args[2])};
        println!("{}",if all{containers::listing_all(std::path::Path::new(path))?}else{containers::listing(std::path::Path::new(path))?});return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--dos-read"){
        if dos_args.len()<4||dos_args.len()>5{return Err("usage: pil --dos-read IMAGE.img|ARCHIVE.zip INNER_PATH [DEST]".into());}
        let bytes=containers::read_entry(std::path::Path::new(&dos_args[2]),&dos_args[3])?;
        if let Some(dest)=dos_args.get(4){
            use std::io::Write;
            let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(dest).map_err(|e|e.to_string())?;
            f.write_all(&bytes).map_err(|e|e.to_string())?;println!("extracted {} ({} bytes) to {}",dos_args[3],bytes.len(),dest);
        }else{
            use std::io::Write;std::io::stdout().write_all(&bytes).map_err(|e|e.to_string())?;
        }return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--dos-extract"){
        if dos_args.len()!=4{return Err("usage: pil --dos-extract IMAGE.img|ARCHIVE.zip NEW_DIRECTORY".into());}
        println!("{}",containers::extract(std::path::Path::new(&dos_args[2]),std::path::Path::new(&dos_args[3]))?);return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--dos-unpack-drive"){
        if dos_args.len()!=4{return Err("usage: pil --dos-unpack-drive IMAGE.img NEW_DIRECTORY".into());}
        println!("{}",containers::extract_drive(std::path::Path::new(&dos_args[2]),std::path::Path::new(&dos_args[3]))?);return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--dos-drive-name"){
        if dos_args.len()!=3{return Err("usage: pil --dos-drive-name IMAGE.img".into());}
        println!("{}",containers::drive_image_name(std::path::Path::new(&dos_args[2]))?);return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--write-dos-mandel"){
        if dos_args.len()!=3{return Err("usage: pil --write-dos-mandel PATH".into());}
        use std::io::Write;
        let bytes=mandelbrot::guest_vga_bytes(128,32)?;
        let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(&dos_args[2]).map_err(|e|e.to_string())?;
        f.write_all(&bytes).map_err(|e|e.to_string())?;println!("wrote {}",dos_args[2]);return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--write-dos-hello"){
        if dos_args.len()!=4{return Err("usage: pil --write-dos-hello com|exe PATH (new file only)".into());}
        let bytes=dos::hello(&dos_args[2])?;
        use std::io::Write;
        let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(&dos_args[3]).map_err(|e|e.to_string())?;
        f.write_all(&bytes).map_err(|e|e.to_string())?;println!("wrote original DOS hello {} ({} bytes)",dos_args[3],bytes.len());return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--dos-catalog"){
        if dos_args.len()!=4{return Err("usage: pil --dos-catalog SHELL_PROGRAM CATALOG_TSV".into());}
        return dos::host_console::catalog::run(&dos_args[2],&dos_args[3]).map_err(Into::into);
    }
    if dos_args.get(1).map(String::as_str)==Some("--dos-media"){
        let usage="usage: pil --dos-media SHELL_PROGRAM [IMAGE_OR_DISK_SET]";
        if !(3..=4).contains(&dos_args.len()){return Err(usage.into());}
        return dos::host_console::media_launch::run(&dos_args[2],dos_args.get(3).map(String::as_str),&stop_log).map_err(Into::into);
    }
    if dos_args.get(1).map(String::as_str)==Some("--dos-session"){
        let usage="usage: pil --dos-session [--config FILE] [--mount HOST_FOLDER] [--args COMMAND_LINE] [--fpu none|80387] [--cpu 386|486dx] PROGRAM.COM|PROGRAM.EXE|IMAGE.img::INNER_PATH";
        let(mut mount,mut tail,mut config,mut fpu,mut cpu,mut rest)=(None,None,None,None,None,&dos_args[2..]);
        while let[flag,value,more@..]=rest{
            match flag.as_str(){"--mount" if mount.is_none()=>mount=Some(value.as_str()),"--args" if tail.is_none()=>tail=Some(value.as_str()),"--config" if config.is_none()=>config=Some(value.as_str()),"--fpu" if fpu.is_none()&&matches!(value.as_str(),"none"|"80387")=>fpu=Some(value.as_str()),"--cpu" if cpu.is_none()&&matches!(value.as_str(),"386"|"486dx")=>cpu=Some(value.as_str()),_=>break}
            rest=more;
        }
        let[program]=rest else{return Err(usage.into())};
        return dos::session::serve(program,mount,tail,&stop_log,config,fpu,cpu).map_err(Into::into);
    }
    if matches!(dos_args.get(1).map(String::as_str),Some("--dos-console"|"--dos-text")){
        let code=dos_text_io::cli(&dos_args[2..],dos_args[1]=="--dos-text",&stop_log)?;
        if code!=0{std::process::exit(code);}
        return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--pildos"){
        let usage="usage: pil --pildos [--tty|--window] [CONFIG]";
        let mut window=cfg!(windows);let mut config=None;
        for arg in &dos_args[2..]{
            match arg.as_str(){
                "--tty"=>window=false,
                "--window"=>window=true,
                a if a.starts_with("--")||config.is_some()=>return Err(usage.into()),
                a=>config=Some(a),
            }
        }
        println!("{}",dos::setup::run(config,window,&stop_log)?);
        return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--boot-probe"){
        let usage="usage: pil --boot-probe IMAGE [--hd] [--hd-image DISK] [--hd-folder DIR] [--steps N] [--keys TEXT] [--keys-after N] [--type STEPS:TEXT]... [--change-floppy STEPS:IMAGE]... [--script FILE] [--fpu none|80387] [--cpu 386|486dx] [--rtc-year YEAR] [--guest-keymap us|uk|fi]";
        let(mut image,mut drive,mut steps,mut at,mut keys,mut keys_after)=(None,0u8,10_000_000usize,2,String::new(),5_000_000usize);
        let mut hard_disk=None;let mut folder=None;let mut typed=Vec::new();let mut changes=Vec::new();let mut rtc_year=None;let mut script=None;let mut fpu:Option<bool>=None;let mut cpu:Option<address::CpuModel>=None;let mut keymap=dos::guest_keymap::Keymap::default();
        while at<dos_args.len(){
            match dos_args[at].as_str(){
                "--hd"=>drive=0x80,
                "--type"=>{
                    let value=dos_args.get(at+1).ok_or(usage)?;
                    let (when,text)=value.split_once(':').ok_or("--type takes STEPS:TEXT")?;
                    typed.push((usize::try_from(number(when)?).map_err(|_|"step count out of range")?,text.replace("\\r","\r")));at+=1;
                },
                "--change-floppy"=>{
                    let value=dos_args.get(at+1).ok_or(usage)?;
                    let (when,file)=value.split_once(':').ok_or("--change-floppy takes STEPS:IMAGE")?;
                    let bytes=std::fs::read(file).map_err(|e|format!("{file}: {e}"))?;
                    changes.push((usize::try_from(number(when)?).map_err(|_|"step count out of range")?,bytes));at+=1;
                },
                "--hd-image"=>{hard_disk=Some(dos_args.get(at+1).ok_or(usage)?.clone());at+=1;},
                "--fpu"=>{
                    fpu=Some(match dos_args.get(at+1).map(String::as_str){Some("80387")=>true,Some("none")=>false,_=>return Err("--fpu takes none or 80387".into())});at+=1;
                },
                "--cpu"=>{
                    cpu=Some(address::CpuModel::parse(dos_args.get(at+1).ok_or("--cpu takes 386 or 486dx")?)?);at+=1;
                },
                "--script"=>{
                    let file=dos_args.get(at+1).ok_or(usage)?;
                    script=Some(dos::boot_script(&std::fs::read_to_string(file).map_err(|e|format!("{file}: {e}"))?)?);at+=1;
                },
                "--hd-folder"=>{folder=Some(dos_args.get(at+1).ok_or(usage)?.clone());at+=1;},
                "--keys"=>{keys=dos_args.get(at+1).ok_or(usage)?.replace("\\r","\r");at+=1;},
                "--rtc-year"=>{rtc_year=Some(u32::try_from(number(dos_args.get(at+1).ok_or(usage)?)?).map_err(|_|"year out of range")?);at+=1;},
                "--guest-keymap"=>{keymap=dos::guest_keymap::Keymap::parse(dos_args.get(at+1).ok_or(usage)?)?;at+=1;},
                "--keys-after"=>{keys_after=usize::try_from(number(dos_args.get(at+1).ok_or(usage)?)?).map_err(|_|"step count out of range")?;at+=1;},
                "--steps"=>{steps=usize::try_from(number(dos_args.get(at+1).ok_or(usage)?)?).map_err(|_|"budget out of range")?;at+=1;},
                a if a.starts_with("--")||image.is_some()=>return Err(usage.into()),
                a=>image=Some(a),
            }
            at+=1;
        }
        let image=image.ok_or(usage)?;
        // A fixed disk (--hd, --hd-image) is written back to its file; a
        // folder disk's changes go back to the folder when the run ends.
        if script.is_some()&&(!keys.is_empty()||!typed.is_empty()||!changes.is_empty()){return Err("--script replaces --keys, --type and --change-floppy".into());}
        let folder=folder.map(|dir|dos::FolderDisk::create(std::path::Path::new(&dir))).transpose()?;
        if let Some((_,notes))=&folder{for note in notes{eprintln!("--hd-folder: {note}");}}
        let machine=dos::boot_command(image,drive,hard_disk.as_deref(),folder.as_ref().map(|f|f.0.image.as_path()));
        let machine=machine.and_then(|mut m|{if let Some(model)=cpu{m.select_cpu(model)?;}if let Some(present)=fpu{m.select_fpu(present)?;}Ok(m)});
        let report=machine.and_then(|m|match &script{
            Some(script)=>{
                let mut out=|text:&str|{use std::io::Write;let mut o=std::io::stdout();let _=o.write_all(text.as_bytes());let _=o.flush();};
                dos::boot_script_probe(m,steps,script,rtc_year,keymap,Some(&mut out))
            },
            None=>dos::boot_probe(m,steps,&keys,keys_after,&typed,&changes,rtc_year,keymap),
        });
        let back=folder.map(|(f,_)|f.finish());
        print!("{}",report?);
        if let Some(back)=back{println!("--hd-folder {}",back?);}
        return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--new-tar-floppy"){
        let usage="usage: pil --new-tar-floppy IMAGE [--size 360|720|1200|1440|2880] FILE... (a diskette holding a ustar archive of the files, read in the guest with `cpio -i -H ustar </dev/fd0` or `tar xf /dev/fd0`)";
        let image=dos_args.get(2).ok_or(usage)?;
        let mut kilobytes=1200usize;let mut files=Vec::new();let mut at=3;
        while at<dos_args.len(){
            if dos_args[at]=="--size"{kilobytes=dos_args.get(at+1).ok_or(usage)?.parse().map_err(|_|usage)?;at+=2;continue;}
            files.push(dos_args[at].clone());at+=1;
        }
        if files.is_empty()||![360,720,1200,1440,2880].contains(&kilobytes){return Err(usage.into());}
        use std::io::Write;let bytes=dos::tar_floppy(&files,kilobytes*1024)?;
        let mut f=std::fs::OpenOptions::new().write(true).create_new(true).open(image).map_err(|e|format!("{image}: {e}"))?;
        f.write_all(&bytes).map_err(|e|format!("{image}: {e}"))?;
        println!("{image}: {} file(s) in a {kilobytes} KB ustar diskette",files.len());return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--new-hd-image"){
        let usage="usage: pil --new-hd-image FILE MEGABYTES (a new, empty fixed-disk image: 16 heads, 63 sectors, up to 1024 cylinders)";
        let (Some(file),Some(size),None)=(dos_args.get(2),dos_args.get(3),dos_args.get(4))else{return Err(usage.into())};
        let megabytes:u64=size.parse().map_err(|_|usage)?;
        let cylinders=megabytes*1024*1024/(16*63*512);
        if !(1..=1024).contains(&cylinders){return Err(format!("{megabytes} MB gives {cylinders} cylinders; 1-1024 (1-504 MB) fit"));}
        let bytes=cylinders*16*63*512;
        let f=std::fs::OpenOptions::new().write(true).create_new(true).open(file).map_err(|e|format!("{file}: {e}"))?;
        f.set_len(bytes).map_err(|e|format!("{file}: {e}"))?;
        println!("{file}: {cylinders} cylinders, 16 heads, 63 sectors ({bytes} bytes)");return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--dos-probe"){
        let mut at=2;let mut budget=10000;
        if dos_args.get(at).map(String::as_str)==Some("--steps"){
            let n=dos_args.get(at+1).ok_or("--steps requires a budget")?;
            budget=usize::try_from(number(n)?).map_err(|_|"budget out of range")?;at+=2;
        }
        println!("{}",dos::report(&dos_args[at..],budget)?);return Ok(());
    }
    if dos_args.get(1).map(String::as_str)==Some("--run-rom"){
        if dos_args.len()<3||dos_args.len()>5{return Err("usage: pil --run-rom PATH [--steps N]".into());}
        let rom_path=std::path::Path::new(&dos_args[2]);
        let mut budget=dos::MAX_STEPS;
        if dos_args.len()==5&&dos_args[3]=="--steps"{
            budget=usize::try_from(number(&dos_args[4])?).map_err(|_|"budget out of range")?;
        }else if dos_args.len()!=3{
            return Err("usage: pil --run-rom PATH [--steps N]".into());
        }
        let probe=dos::run_rom(rom_path,budget)?;
        dos::stoplog::Logger::new(&stop_log,&dos_args[2]).record(&probe.loaded,match probe.stop{dos::Stop::Budget=>"running",dos::Stop::WaitingForInput=>"waiting_input",dos::Stop::Exit(_)=>"exit",_=>"stopped"},&format!("{:?}",probe.stop),probe.completed,&probe.exceptions,probe.exception_count);
        print!("{}",dos::format_rom_report(&probe));
        return Ok(());
    }

    let args = dos_args;
    if matches!(args.get(1).map(String::as_str),Some("--mandelbrot"|"--compare-mandelbrot")) {
        if args.len()>4{return Err("usage: pil --mandelbrot|--compare-mandelbrot [size 2..256 power of two] [iterations 1..64]".into());}
        let parse=|i:usize,default:u32|->R<u32>{if let Some(x)=args.get(i){u32::try_from(number(x.as_str())?).map_err(|_|"parameter out of range".into())}else{Ok(default)}};
        let (size,iterations)=(parse(2,256)?,parse(3,32)?);
        println!("{}",if args[1]=="--compare-mandelbrot"{native::compare_mandelbrot(size,iterations)?}else{mandelbrot::render_files(size,iterations)?});return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--verify-backends") {
        if args.len()!=2{return Err("usage: pil --verify-backends".into());}
        println!("{}",native::verify()?);return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--cpu-bench") {
        if args.len()>4{return Err("usage: pil --cpu-bench [iterations] [samples]".into());}
        let n=if let Some(x)=args.get(2){u32::try_from(number(x)?).map_err(|_|"iterations out of range")?}else{10000};
        let samples=if let Some(x)=args.get(3){u32::try_from(number(x)?).map_err(|_|"samples out of range")?}else{5};
        println!("{}",native::cpu_benchmark(n,samples)?);return Ok(());
    }
    if matches!(args.get(1).map(String::as_str),Some("--compare-backends"|"--emit-suite-rust"|"--emit-loop-rust")) {
        if args.len()>4{return Err("usage: pil --compare-backends|--emit-suite-rust|--emit-loop-rust [iterations] [samples]".into());}
        let n=if let Some(x)=args.get(2){u32::try_from(number(x)?).map_err(|_|"iterations out of range")?}else{10000};
        let samples=if let Some(x)=args.get(3){u32::try_from(number(x)?).map_err(|_|"samples out of range")?}else{5};
        match args[1].as_str() {
            "--emit-suite-rust" => print!("{}",native::emit_suite(n,samples)?),
            "--emit-loop-rust" => print!("{}",native::emit_loop(n,samples)?),
            _ => println!("{}",native::compare(n,samples)?),
        }
        return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--write-factorials") || args.get(1).map(String::as_str)==Some("--write-loop") {
        let path=args.get(2).ok_or("usage: pil --write-factorials|--write-loop PATH")?;
        let bytes=if args[1]=="--write-factorials"{guest::factorial_bytes()?}else{guest::loop_bytes(guest::LOOP_ITERATIONS)?};
        std::fs::write(path,bytes).map_err(|e|e.to_string())?;return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--numeric-test") {return debugger::numeric_test();}
    if args.get(1).map(String::as_str)==Some("--bench") {
        let mut counts=Vec::new();let mut cached=true;
        for arg in &args[2..] {
            if arg=="--uncached" && cached {cached=false;}else{counts.push(arg);}
        }
        if counts.len()>2{return Err("usage: pil --bench [iterations] [samples] [--uncached]".into());}
        let n=if let Some(x)=counts.first(){u32::try_from(number(x)?).map_err(|_|"iterations out of range")?}else{1000};
        let samples=if let Some(x)=counts.get(1){u32::try_from(number(x)?).map_err(|_|"samples out of range")?}else{3};
        println!("{}",debugger::benchmark_mode(n,samples,cached)?);return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--stress") {
        if args.len()>3{return Err("usage: pil --stress [passes]".into());}
        let passes=if let Some(x)=args.get(2){u32::try_from(number(x)?).map_err(|_|"passes out of range")?}else{25};
        println!("{}",debugger::factorial_stress(passes)?);return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--write-hello") {
        let path=args.get(2).ok_or("usage: pil --write-hello PATH")?;
        std::fs::write(path,guest::hello_bytes()?).map_err(|e|e.to_string())?;return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--write-selftest") {
        let path=args.get(2).ok_or("usage: pil --write-selftest PATH")?;
        std::fs::write(path,guest::selftest_bytes()?).map_err(|e|e.to_string())?;return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--selftest") {
        return debugger::selftest();
    }
    if args.get(1).map(String::as_str)==Some("--debug-service") {
        let path=match args.len(){2=>None,4 if args[2]=="--load"=>Some(args[3].as_str()),_=>return Err("usage: pil --debug-service [--load RAW_BINARY]".into())};
        return debugger::serve(std::io::stdin().lock(),std::io::stdout().lock(),path).map_err(|e|e.to_string());
    }
    if args.get(1).map(String::as_str)==Some("--repl") {
        if args.len()>2 {
            if args.len()!=4 || args[2]!="--load" {return Err("usage: pil --repl [--load PATH]".into());}
            return debugger::repl_image(std::io::stdin().lock(),std::io::stdout().lock(),&args[3]).map_err(|e|e.to_string());
        }
        return debugger::repl(std::io::stdin().lock(),std::io::stdout().lock()).map_err(|e|e.to_string());
    }
    if args.get(1).map(String::as_str)==Some("--demo-branch") {
        let cmp=compile(&instantiate(CMP,32),32)?;
        let jb=compile(&instantiate(description("jb")?,32),32)?;
        let mut cpu=Cpu::default();cpu.eip=0x100;cpu.regs[0]=10;cpu.regs[1]=20;
        let dst=View{register:0,low:0,width:32};let src=View{register:1,low:0,width:32};
        execute(&cmp,&mut cpu,dst,src)?;
        execute_flow(&jb,&mut cpu,dst,src,FlowContext{next_eip:0x106,displacement:-4,displacement_bits:8,code_limit:u32::MAX}).map_err(|e|format!("{e:?}"))?;
        println!("CMP 10,20; JB -4: EIP=0x{:08x} (expected 0x00000102)",cpu.eip);
        return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--emit-precompiled-forms") {
        let path=args.get(2).ok_or("usage: pil --emit-precompiled-forms PATH [--common] [--check]")?;
        let (mut check,mut common)=(false,false);
        for flag in &args[3..]{
            match flag.as_str(){"--check" if !check=>check=true,"--common" if !common=>common=true,_=>return Err("usage: pil --emit-precompiled-forms PATH [--common] [--check]".into())}
        }
        let set=if common{precompiled::FormSet::Common}else{precompiled::FormSet::Full};
        println!("{}",precompiled::emit_set(path,check,set,&form_machines()?)?);return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--emit-native-application"){
        if args.len()!=4{return Err("usage: pil --emit-native-application INPUT OUTPUT.rs".into());}
        println!("{}",native_application::emit(&args[2],&args[3])?);return Ok(());
    }
    if args.get(1).map(String::as_str)==Some("--emit-native-pil"){
        if args.len()!=4{return Err("usage: pil --emit-native-pil INPUT OUTPUT.rs".into());}
        println!("{}",native_application::emit_pil(&args[2],&args[3])?);return Ok(());
    }
    if args.get(1).map(String::as_str) == Some("--emit-rust") {
        let w = args.get(2).ok_or("usage: pil --emit-rust 8|16|32 [description]")?;
        let w = width(w)?;
        let name = args.get(3).map(String::as_str).unwrap_or("add");
        let p = compile(&instantiate(description(name)?,w),w)?;
        print!("{}",emit_rust(&p)); return Ok(());
    }
    Err(format!("unknown command or option: {}. Use pil --help for usage.",args.get(1).map(String::as_str).unwrap_or("(missing command)")))
}

#[cfg(test)]
#[path = "../tests/unit/host_cpu.rs"]
mod host_cpu_tests;

#[cfg(test)]
#[path = "../tests/unit/segments.rs"]
mod segment_transfer_tests;

#[cfg(test)]
#[path = "../tests/unit/calls_strings.rs"]
mod calls_strings_tests;

#[cfg(test)]
#[path = "../tests/unit/control_flags.rs"]
mod control_flag_tests;

#[cfg(test)]
#[path = "../tests/unit/compare_strings.rs"]
mod compare_string_tests;

#[cfg(test)]
#[path = "../tests/unit/retf.rs"]
mod retf_tests;

#[cfg(test)]
#[path = "../tests/unit/pop_msw.rs"]
mod pop_msw_tests;

#[cfg(test)]
#[path = "../tests/unit/call_far.rs"]
mod call_far_tests;

#[cfg(test)]
#[path = "../tests/unit/sst80386.rs"]
mod sst80386_tests;

#[cfg(test)]
#[path = "../tests/unit/fpu.rs"]
mod fpu_tests;

#[cfg(test)]
#[path = "../tests/unit/precompiled.rs"]
mod precompiled_tests;

#[cfg(test)]
#[path = "../tests/unit/cpu_486.rs"]
mod cpu_486_tests;
