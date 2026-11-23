//! Unit tests for DOS 7 long filename (INT 21h AH=71h) services on host folders.
use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(tag: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("pil-lfn-{tag}-{}-{unique}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_ds(p: &mut Loaded, offset: u32, bytes: &[u8]) {
    let base = u32::from(p.selectors[SegmentRegister::Ds.index()]) * 16;
    p.ram.write_bytes(base + offset, bytes).unwrap();
}

fn read_es(p: &Loaded, offset: u32, count: usize) -> Vec<u8> {
    let base = u32::from(p.selectors[SegmentRegister::Es.index()]) * 16;
    (0..count as u32)
        .map(|i| p.ram.load(base + offset + i, MemWidth::Byte).unwrap() as u8)
        .collect()
}

fn read_ds(p: &Loaded, offset: u32, count: usize) -> Vec<u8> {
    let base = u32::from(p.selectors[SegmentRegister::Ds.index()]) * 16;
    (0..count as u32)
        .map(|i| p.ram.load(base + offset + i, MemWidth::Byte).unwrap() as u8)
        .collect()
}

fn cstr(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    crate::dos_encoding::decode(&bytes[..end])
}

fn call71(p: &mut Loaded, al: u8) {
    p.cpu.regs[0] = (p.cpu.regs[0] & 0xffff0000) | 0x7100 | u32::from(al);
    super::service(p).unwrap();
}

#[test]
fn volume_info_reports_lfn_on_host_drive_and_absence_without_host_folder() {
    // Without any host folder mounted, INT 21h AH=71h stays disabled (AX=7100h, CF=1).
    let r = crate::dos::run(
        crate::dos::load(&[0xb8, 0xa0, 0x71, 0xcd, 0x21, 0xfa, 0xf4], "nolfn.com").unwrap(),
        10,
    )
    .unwrap();
    assert_eq!(r.loaded.cpu.regs[0] & 0xffff, 0x7100);
    assert_eq!(r.loaded.cpu.eflags & 1, 1);

    let root = temp_dir("vol");
    let mut p = crate::dos::load_with_drives(
        &[0xf4],
        "PROBE.COM",
        vec![Source::Host(root.clone()), Source::Memory(crate::dos::MemDir::default())],
        None,
    )
    .unwrap();
    write_ds(&mut p, 0x1000, b"C:\\\0");
    p.cpu.regs[2] = 0x1000; // DX
    p.cpu.regs[7] = 0x2000; // DI
    p.cpu.regs[1] = 16; // CX
    call71(&mut p, 0xa0);
    assert_eq!(p.cpu.eflags & 1, 0);
    assert_eq!(p.cpu.regs[3] & 0xffff, 0x4006);
    assert_eq!(p.cpu.regs[1] & 0xffff, 255);
    assert_eq!(p.cpu.regs[2] & 0xffff, 260);
    assert_eq!(cstr(&read_es(&p, 0x2000, 16)), "FAT");

    // Memory drive Z: answers volume info without claiming LFN capabilities.
    write_ds(&mut p, 0x1000, b"Z:\\\0");
    p.cpu.regs[2] = 0x1000;
    p.cpu.regs[7] = 0x2000;
    p.cpu.regs[1] = 16;
    call71(&mut p, 0xa0);
    assert_eq!(p.cpu.eflags & 1, 0);
    assert_eq!(p.cpu.regs[3] & 0xffff, 0x0000);
    assert_eq!(p.cpu.regs[1] & 0xffff, 12);
    assert_eq!(p.cpu.regs[2] & 0xffff, 67);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn find_first_next_close_return_long_names_aliases_and_timestamps() {
    let root = temp_dir("find");
    std::fs::write(root.join("My Long Document.txt"), b"hello lfn").unwrap();
    std::fs::write(root.join("Project Notes 2026.json"), b"{}").unwrap();
    std::fs::write(root.join("SHORT.TXT"), b"83").unwrap();

    let mut p =
        crate::dos::load_with_drives(&[0xf4], "FIND.COM", vec![Source::Host(root.clone())], None)
            .unwrap();

    write_ds(&mut p, 0x1000, b"*.*\0");
    p.cpu.regs[2] = 0x1000; // DX
    p.cpu.regs[1] = 0x0027; // CL allowable
    p.cpu.regs[6] = 1; // SI=1 DOS date/time
    p.cpu.regs[7] = 0x2000; // DI
    call71(&mut p, 0x4e);
    assert_eq!(p.cpu.eflags & 1, 0);
    let handle = p.cpu.regs[0] & 0xffff;
    assert_ne!(handle, 0);

    let mut found = Vec::new();
    loop {
        let data = read_es(&p, 0x2000, 318);
        let long_name = cstr(&data[44..304]);
        let short_name = cstr(&data[304..318]);
        let size = u32::from_le_bytes(data[32..36].try_into().unwrap());
        found.push((long_name, short_name, size));
        p.cpu.regs[3] = handle;
        p.cpu.regs[6] = 1;
        p.cpu.regs[7] = 0x2000;
        call71(&mut p, 0x4f);
        if p.cpu.eflags & 1 != 0 {
            assert_eq!(p.cpu.regs[0] & 0xffff, 0x12);
            break;
        }
    }
    assert_eq!(
        found,
        vec![
            ("My Long Document.txt".into(), "MYLONG~1.TXT".into(), 9),
            ("Project Notes 2026.json".into(), "PROJEC~1.JSO".into(), 2),
            ("SHORT.TXT".into(), String::new(), 2),
        ]
    );
    p.cpu.regs[3] = handle;
    call71(&mut p, 0xa1);
    assert_eq!(p.cpu.eflags & 1, 0);
    // Closing twice fails with invalid handle (6).
    p.cpu.regs[3] = handle;
    call71(&mut p, 0xa1);
    assert_eq!(p.cpu.eflags & 1, 1);
    assert_eq!(p.cpu.regs[0] & 0xffff, 6);

    // Search via wildcard matching the 8.3 alias form too.
    write_ds(&mut p, 0x1000, b"PROJEC~1.*\0");
    p.cpu.regs[2] = 0x1000;
    p.cpu.regs[1] = 0x0027;
    p.cpu.regs[6] = 0; // SI=0 Win32 FILETIME
    p.cpu.regs[7] = 0x2000;
    call71(&mut p, 0x4e);
    assert_eq!(p.cpu.eflags & 1, 0);
    let data = read_es(&p, 0x2000, 318);
    assert_eq!(cstr(&data[44..304]), "Project Notes 2026.json");
    assert_eq!(cstr(&data[304..318]), "PROJEC~1.JSO");
    assert_ne!(&data[20..28], &[0u8; 8]);
    p.cpu.regs[3] = p.cpu.regs[0] & 0xffff;
    call71(&mut p, 0xa1);
    assert_eq!(p.cpu.eflags & 1, 0);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn canonical_short_and_long_paths_mkdir_chdir_and_getcwd_work_end_to_end() {
    let root = temp_dir("paths");
    let mut p =
        crate::dos::load_with_drives(&[0xf4], "PATH.COM", vec![Source::Host(root.clone())], None)
            .unwrap();

    // 7139h mkdir long name
    write_ds(&mut p, 0x1000, b"Long Directory Name\0");
    p.cpu.regs[2] = 0x1000;
    call71(&mut p, 0x39);
    assert_eq!(p.cpu.eflags & 1, 0);
    assert!(root.join("Long Directory Name").is_dir());

    // 713Bh chdir into long directory, then create file via 716Ch
    p.cpu.regs[2] = 0x1000;
    call71(&mut p, 0x3b);
    assert_eq!(p.cpu.eflags & 1, 0);

    // 7147h reports real long CWD without leading backslash
    p.cpu.regs[2] = 0; // DL=0 default drive
    p.cpu.regs[6] = 0x1200; // DS:SI
    call71(&mut p, 0x47);
    assert_eq!(p.cpu.eflags & 1, 0);
    assert_eq!(cstr(&read_ds(&p, 0x1200, 64)), "Long Directory Name");

    // 716Ch creates new long file, then 7160h CL=0/1/2 converts between long and 8.3 short paths.
    write_ds(&mut p, 0x1000, b"My Long Report.data\0");
    p.cpu.regs[6] = 0x1000; // DS:SI
    p.cpu.regs[3] = 2; // BX mode=read/write
    p.cpu.regs[1] = 0; // CX attr
    p.cpu.regs[2] = 0x0012; // DX: missing->create (1), exist->replace (2)
    call71(&mut p, 0x6c);
    assert_eq!(p.cpu.eflags & 1, 0);
    assert_eq!(p.cpu.regs[1] & 0xffff, 2); // action: created
    let file_handle = p.cpu.regs[0] & 0xffff;

    // 71A6h file info by handle on open handle
    p.cpu.regs[3] = file_handle;
    p.cpu.regs[2] = 0x1300;
    call71(&mut p, 0xa6);
    assert_eq!(p.cpu.eflags & 1, 0);
    assert_eq!(read_ds(&p, 0x1300, 4), 0x20u32.to_le_bytes());

    // Close via AH=3Eh
    p.cpu.regs[0] = 0x3e00;
    p.cpu.regs[3] = file_handle;
    crate::dos::services::dispatch(&mut p, 0x21, &mut crate::host_io::CaptureOutput::new(64)).unwrap();

    // 7160h CL=1 (short) on long path
    write_ds(&mut p, 0x1000, b"My Long Report.data\0");
    p.cpu.regs[6] = 0x1000;
    p.cpu.regs[7] = 0x2000;
    p.cpu.regs[1] = 0x0001;
    call71(&mut p, 0x60);
    assert_eq!(p.cpu.eflags & 1, 0);
    assert_eq!(cstr(&read_es(&p, 0x2000, 128)), r"C:\LONGDI~1\MYLONG~1.DAT");

    // 7160h CL=2 (long) from short alias!
    write_ds(&mut p, 0x1000, b"\\LONGDI~1\\MYLONG~1.DAT\0");
    p.cpu.regs[6] = 0x1000;
    p.cpu.regs[7] = 0x2000;
    p.cpu.regs[1] = 0x0002;
    call71(&mut p, 0x60);
    assert_eq!(p.cpu.eflags & 1, 0);
    assert_eq!(
        cstr(&read_es(&p, 0x2000, 128)),
        r"C:\Long Directory Name\My Long Report.data"
    );

    // 7160h CL=0 (truename) accepts not-yet-existing child inside existing long folder
    write_ds(&mut p, 0x1000, b"..\\LONGDI~1\\New Output File.txt\0");
    p.cpu.regs[6] = 0x1000;
    p.cpu.regs[7] = 0x2000;
    p.cpu.regs[1] = 0x0000;
    call71(&mut p, 0x60);
    assert_eq!(p.cpu.eflags & 1, 0);
    assert_eq!(
        cstr(&read_es(&p, 0x2000, 128)),
        r"C:\Long Directory Name\New Output File.txt"
    );

    // 7156h rename, 7141h delete, 713Bh cd .., 713Ah rmdir
    write_ds(&mut p, 0x1000, b"My Long Report.data\0");
    let base_es = u32::from(p.selectors[SegmentRegister::Es.index()]) * 16;
    p.ram.write_bytes(base_es + 0x2000, b"Renamed Long Report.data\0").unwrap();
    p.cpu.regs[2] = 0x1000;
    p.cpu.regs[7] = 0x2000;
    call71(&mut p, 0x56);
    assert_eq!(p.cpu.eflags & 1, 0);
    assert!(root.join("Long Directory Name").join("Renamed Long Report.data").is_file());

    write_ds(&mut p, 0x1000, b"Renamed Long Report.data\0");
    p.cpu.regs[2] = 0x1000;
    p.cpu.regs[6] = 0; // SI=0 exact delete
    call71(&mut p, 0x41);
    assert_eq!(p.cpu.eflags & 1, 0);
    assert!(!root.join("Long Directory Name").join("Renamed Long Report.data").exists());

    write_ds(&mut p, 0x1000, b"..\0");
    p.cpu.regs[2] = 0x1000;
    call71(&mut p, 0x3b);
    assert_eq!(p.cpu.eflags & 1, 0);

    write_ds(&mut p, 0x1000, b"Long Directory Name\0");
    p.cpu.regs[2] = 0x1000;
    call71(&mut p, 0x3a);
    assert_eq!(p.cpu.eflags & 1, 0);
    assert!(!root.join("Long Directory Name").exists());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn lfn_attributes_size_and_filetime_conversion_roundtrip() {
    let root = temp_dir("timeattr");
    let file_path = root.join("Timestamped Long File.bin");
    std::fs::write(&file_path, b"0123456789").unwrap();

    let mut p =
        crate::dos::load_with_drives(&[0xf4], "TIME.COM", vec![Source::Host(root.clone())], None)
            .unwrap();

    write_ds(&mut p, 0x1000, b"Timestamped Long File.bin\0");
    p.cpu.regs[2] = 0x1000;
    p.cpu.regs[3] = 0; // BL=0 get attr
    call71(&mut p, 0x43);
    assert_eq!(p.cpu.eflags & 1, 0);
    assert_eq!(p.cpu.regs[1] & 0xffff, 0x20);

    p.cpu.regs[2] = 0x1000;
    p.cpu.regs[3] = 2; // BL=2 get size
    call71(&mut p, 0x43);
    assert_eq!(p.cpu.eflags & 1, 0);
    assert_eq!(p.cpu.regs[0] & 0xffff, 10);
    assert_eq!(p.cpu.regs[2] & 0xffff, 0);

    // 71A7h BL=1 DOS -> FILETIME, then BL=0 FILETIME -> DOS roundtrip (2026-11-22 10:30:41.50 -> +1.50s = 150*10ms)
    let dos_date: u16 = ((2026 - 1980) << 9) | (11 << 5) | 22;
    let dos_time: u16 = (10 << 11) | (30 << 5) | (40 / 2);
    p.cpu.regs[3] = (150u32 << 8) | 1; // BH=150 (1.50s), BL=1
    p.cpu.regs[1] = u32::from(dos_time);
    p.cpu.regs[2] = u32::from(dos_date);
    p.cpu.regs[7] = 0x2000;
    call71(&mut p, 0xa7);
    assert_eq!(p.cpu.eflags & 1, 0);
    let ft = read_es(&p, 0x2000, 8);
    write_ds(&mut p, 0x1100, &ft);
    p.cpu.regs[3] = 0; // BL=0 FILETIME -> DOS
    p.cpu.regs[6] = 0x1100;
    call71(&mut p, 0xa7);
    assert_eq!(p.cpu.eflags & 1, 0);
    assert_eq!(p.cpu.regs[1] & 0xffff, u32::from(dos_time));
    assert_eq!(p.cpu.regs[2] & 0xffff, u32::from(dos_date));
    assert_eq!((p.cpu.regs[3] >> 8) & 0xff, 150);

    std::fs::remove_dir_all(root).unwrap();
}
