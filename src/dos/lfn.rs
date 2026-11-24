//! DOS 7 long filename services (INT 21h AH=71h) on mounted host folders.
//! Falls back cleanly to 8.3 names for image and memory drives when at least
//! one host folder is mounted; when none is mounted, DOS reports AX=7100h CF=1.
use super::*;
use super::files::{
    self, device, dir_of, display, fcb, fail, host_listing, install, listing, ok, read_entry,
    set_ax, short, stamp, store, store_ds, unstamp, Dir, Drive, Drives, Entry, Loc, Name, Node,
    OpenFile, NUL,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_LFN_PATH: u32 = 260;
const MAX_LFN_SEARCHES: usize = 64;
const UNIX_TO_FILETIME_SECS: u64 = 11_644_473_600;
const TICKS_PER_SEC: u64 = 10_000_000;

#[derive(Clone, Debug)]
pub(super) struct LfnSearch {
    pub(super) drive: usize,
    entries: Vec<LfnSearchEntry>,
    index: usize,
}

#[derive(Clone, Debug)]
struct LfnSearchEntry {
    long_name: Vec<u8>,
    short_name: Option<Vec<u8>>,
    attr: u8,
    size: u32,
    time: u16,
    date: u16,
    filetime: u64,
}

pub(super) fn service(p: &mut Loaded) -> Result<(), String> {
    let al = p.cpu.regs[0] as u8;
    match al {
        0x0d => {
            ok(p);
            Ok(())
        }
        0x39 => mkdir(p),
        0x3a => rmdir(p),
        0x3b => chdir(p),
        0x41 => delete(p),
        0x43 => file_attr(p),
        0x47 => get_cwd(p),
        0x4e => find_first(p),
        0x4f => find_next(p),
        0x56 => rename(p),
        0x60 => canonical(p),
        0x6c => extended_open(p),
        0xa0 => volume_info(p),
        0xa1 => find_close(p),
        0xa6 => file_info_by_handle(p),
        0xa7 => convert_time(p),
        _ => {
            p.cpu.regs[0] &= !255;
            p.cpu.eflags |= 1;
            p.cpu.undefined_flags &= !1;
            Ok(())
        }
    }
}

fn lfn_asciiz(p: &Loaded, segment: SegmentRegister, offset: u32) -> Result<Vec<u8>, String> {
    let context = MemoryContext {
        segment,
        cache: p.image.segments[segment.index()],
    };
    let mut path = Vec::new();
    for i in 0..MAX_LFN_PATH {
        let Some(at) = offset.checked_add(i).filter(|&a| a <= 0xffff) else {
            return Err("DOS LFN path exceeds segment end before NUL terminator".into());
        };
        let addr = context
            .translate(at, MemWidth::Byte, false, p.cpu.eip)
            .map_err(|e| format!("DOS LFN path access: {e:?}"))?;
        let b = p
            .ram
            .load(addr, MemWidth::Byte)
            .map_err(|e| format!("DOS LFN path RAM access: {e:?}"))? as u8;
        if b == 0 {
            return Ok(path);
        }
        path.push(b);
    }
    Err("DOS LFN path has no NUL terminator within 260 bytes".into())
}

fn load_bytes(
    p: &Loaded,
    segment: SegmentRegister,
    offset: u32,
    count: usize,
) -> Result<Vec<u8>, String> {
    if offset as usize + count > 0x10000 {
        return Err("DOS buffer exceeds segment end".into());
    }
    let context = MemoryContext {
        segment,
        cache: p.image.segments[segment.index()],
    };
    let mut out = Vec::with_capacity(count);
    for n in 0..count as u32 {
        let addr = context
            .translate(offset + n, MemWidth::Byte, false, p.cpu.eip)
            .map_err(|e| format!("DOS buffer access: {e:?}"))?;
        out.push(
            p.ram
                .load(addr, MemWidth::Byte)
                .map_err(|e| format!("DOS buffer RAM access: {e:?}"))? as u8,
        );
    }
    Ok(out)
}

fn cp437_byte(c: char) -> u8 {
    if (' '..='~').contains(&c) {
        return c as u8;
    }
    for b in 128u8..=255 {
        if crate::dos_encoding::decode(&[b]).chars().next() == Some(c) {
            return b;
        }
    }
    b'_'
}

fn encode_cp437(text: &str) -> Vec<u8> {
    text.chars().map(cp437_byte).collect()
}

fn host_entry_name(entry: &Entry) -> Option<String> {
    match &entry.node {
        Node::Host(path) => path.file_name()?.to_str().map(str::to_owned),
        _ => None,
    }
}

fn entry_long_bytes(entry: &Entry) -> Vec<u8> {
    match host_entry_name(entry) {
        Some(name) => encode_cp437(&name),
        None => display(&entry.name),
    }
}

fn entry_short_alt(entry: &Entry) -> Option<Vec<u8>> {
    let Some(long_name) = host_entry_name(entry) else {
        return None;
    };
    if short(&long_name) == Some(entry.name) {
        None
    } else {
        Some(display(&entry.name))
    }
}

fn entry_filetime(entry: &Entry) -> u64 {
    if let Node::Host(path) = &entry.node {
        if let Ok(Some(modified)) = std::fs::symlink_metadata(path).map(|m| m.modified().ok()) {
            return system_time_to_filetime(modified);
        }
    }
    dos_to_filetime(entry.time, entry.date, 0)
}

fn system_time_to_filetime(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).ok().map_or(
        UNIX_TO_FILETIME_SECS * TICKS_PER_SEC,
        |d| {
            d.as_secs()
                .saturating_add(UNIX_TO_FILETIME_SECS)
                .saturating_mul(TICKS_PER_SEC)
                .saturating_add(u64::from(d.subsec_nanos() / 100))
        },
    )
}

fn dos_to_filetime(time: u16, date: u16, hundredths_10ms: u8) -> u64 {
    let base = system_time_to_filetime(unstamp(time, date));
    base.saturating_add(u64::from(hundredths_10ms.min(199)) * 100_000)
}

fn filetime_to_dos(ft: u64) -> (u16, u16, u8) {
    let epoch = UNIX_TO_FILETIME_SECS * TICKS_PER_SEC;
    if ft < epoch {
        return (0, 0x21, 0);
    }
    let unix_ft = ft - epoch;
    let secs = unix_ft / TICKS_PER_SEC;
    let sub_10ms = ((unix_ft % TICKS_PER_SEC) / 100_000) as u8;
    let bh = if secs & 1 != 0 { 100 + sub_10ms } else { sub_10ms };
    let (time, date) = stamp(Some(UNIX_EPOCH + Duration::from_secs(secs)));
    (time, date, bh)
}

fn drive_and_rest<'a>(d: &Drives, raw: &'a [u8]) -> Result<(usize, bool, &'a [u8]), u16> {
    let mut rest = raw;
    let mut drive = d.current;
    if rest.len() >= 2 && rest[1] == b':' {
        drive = d
            .list
            .iter()
            .position(|x| rest[0].eq_ignore_ascii_case(&x.letter))
            .ok_or(3u16)?;
        rest = &rest[2..];
    }
    let abs = rest.first().is_some_and(|&c| c == b'\\' || c == b'/');
    Ok((drive, abs, rest))
}

#[derive(Clone, Debug)]
struct ResolvedPath {
    drive: usize,
    dos_path: Vec<Name>,
    long_parts: Vec<Vec<u8>>,
    dir: Dir,
    dir_stamp: (u16, u16),
}

fn find_host_entry<'a>(entries: &'a [Entry], part: &str) -> Option<&'a Entry> {
    // Prefer exact long name, then case-insensitive long name, then 8.3 alias/short name.
    if let Some(hit) = entries
        .iter()
        .find(|e| host_entry_name(e).as_deref() == Some(part))
    {
        return Some(hit);
    }
    let lower = part.to_lowercase();
    if let Some(hit) = entries
        .iter()
        .find(|e| host_entry_name(e).is_some_and(|n| n.to_lowercase() == lower))
    {
        return Some(hit);
    }
    let part_83 = part.to_ascii_uppercase();
    entries.iter().find(|e| {
        String::from_utf8(display(&e.name))
            .is_ok_and(|s| s == part_83)
    })
}

fn resolve_dir_parts(
    d: &Drive,
    start_cwd: bool,
    parts: &[&[u8]],
) -> Result<(Vec<Name>, Vec<Vec<u8>>, Dir, (u16, u16)), u16> {
    match &d.source {
        Source::Host(root) => {
            let mut stack: Vec<(Option<Name>, Vec<u8>, PathBuf, (u16, u16))> =
                vec![(None, Vec::new(), root.clone(), (0, 0x21))];
            if start_cwd {
                for &seg in &d.cwd {
                    let cur = &stack.last().unwrap().2;
                    let entries = host_listing(cur)?;
                    let entry = entries
                        .into_iter()
                        .find(|e| e.name == seg && e.attr & 0x10 != 0)
                        .ok_or(3u16)?;
                    let Some(Dir::Host(next)) = entry.dir() else {
                        return Err(3);
                    };
                    let name = entry_long_bytes(&entry);
                    stack.push((Some(entry.name), name, next, (entry.time, entry.date)));
                }
            }
            for &raw_part in parts {
                match raw_part {
                    b"" | b"." => {}
                    b".." => {
                        if stack.len() <= 1 {
                            return Err(3);
                        }
                        stack.pop();
                    }
                    _ => {
                        let decoded = crate::dos_encoding::decode(raw_part);
                        let cur = &stack.last().unwrap().2;
                        let entries = host_listing(cur)?;
                        let entry = find_host_entry(&entries, &decoded).ok_or(3u16)?;
                        let Some(Dir::Host(next)) = entry.dir() else {
                            return Err(3);
                        };
                        let name = entry_long_bytes(entry);
                        stack.push((Some(entry.name), name, next, (entry.time, entry.date)));
                    }
                }
            }
            let top = stack.last().unwrap();
            let dir = Dir::Host(top.2.clone());
            let dir_stamp = top.3;
            let mut dos_path = Vec::new();
            let mut long_parts = Vec::new();
            for (dos, long, _, _) in stack.into_iter().skip(1) {
                dos_path.push(dos.unwrap());
                long_parts.push(long);
            }
            Ok((dos_path, long_parts, dir, dir_stamp))
        }
        Source::Empty | Source::Image(_) | Source::Memory(_) => {
            let mut dos_path = if start_cwd { d.cwd.clone() } else { Vec::new() };
            for &part in parts {
                match part {
                    b"" | b"." => {}
                    b".." => {
                        if dos_path.pop().is_none() {
                            return Err(3);
                        }
                    }
                    _ => dos_path.push(fcb(part)),
                }
            }
            let (dir, t, dt) = dir_of(d, &dos_path)?;
            let long_parts = dos_path.iter().map(display).collect();
            Ok((dos_path, long_parts, dir, (t, dt)))
        }
    }
}

fn split_lfn_parts(rest: &[u8]) -> Vec<&[u8]> {
    rest.split(|&c| c == b'\\' || c == b'/')
        .filter(|p| !p.is_empty())
        .collect()
}

fn resolve_lfn_dir(d: &Drives, raw: &[u8]) -> Result<ResolvedPath, u16> {
    let (drive, abs, rest) = drive_and_rest(d, raw)?;
    let parts = split_lfn_parts(rest);
    let (dos_path, long_parts, dir, dir_stamp) = resolve_dir_parts(&d.list[drive], !abs, &parts)?;
    Ok(ResolvedPath {
        drive,
        dos_path,
        long_parts,
        dir,
        dir_stamp,
    })
}

fn resolve_lfn_parent<'a>(
    d: &Drives,
    raw: &'a [u8],
) -> Result<(ResolvedPath, &'a [u8]), u16> {
    let (drive, abs, rest) = drive_and_rest(d, raw)?;
    let parts: Vec<&[u8]> = rest
        .split(|&c| c == b'\\' || c == b'/')
        .filter(|p| !p.is_empty())
        .collect();
    let Some((&last, parent_parts)) = parts.split_last() else {
        return Err(3);
    };
    if matches!(last, b"." | b"..") {
        return Err(3);
    }
    let (dos_path, long_parts, dir, dir_stamp) =
        resolve_dir_parts(&d.list[drive], !abs, parent_parts)?;
    Ok((
        ResolvedPath {
            drive,
            dos_path,
            long_parts,
            dir,
            dir_stamp,
        },
        last,
    ))
}

fn lookup_lfn(d: &Drives, raw: &[u8]) -> Result<(usize, ResolvedPath, Entry), u16> {
    let (parent, last) = resolve_lfn_parent(d, raw)?;
    let drv = &d.list[parent.drive];
    match &drv.source {
        Source::Host(_) => {
            let decoded = crate::dos_encoding::decode(last);
            if decoded.contains('*') || decoded.contains('?') {
                return Err(2);
            }
            let entries = listing(drv, &parent.dir)?;
            let entry = find_host_entry(&entries, &decoded).cloned().ok_or(2u16)?;
            Ok((parent.drive, parent, entry))
        }
        Source::Empty | Source::Image(_) | Source::Memory(_) => {
            let pattern = fcb(last);
            if pattern.contains(&b'?') {
                return Err(2);
            }
            let entries = listing(drv, &parent.dir)?;
            let entry = entries
                .into_iter()
                .find(|e| e.name == pattern)
                .ok_or(2u16)?;
            Ok((parent.drive, parent, entry))
        }
    }
}

fn validate_new_host_name(raw: &[u8]) -> Result<String, u16> {
    let decoded = crate::dos_encoding::decode(raw);
    let trimmed = decoded.trim_end_matches([' ', '.']);
    if trimmed.is_empty()
        || trimmed.len() > 255
        || matches!(trimmed, "." | "..")
        || trimmed.chars().any(|c| {
            c < ' ' || matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
    {
        return Err(3);
    }
    Ok(trimmed.to_string())
}

fn place_lfn(d: &Drives, raw: &[u8]) -> Result<(usize, Loc, Option<Entry>), u16> {
    let (parent, last) = resolve_lfn_parent(d, raw)?;
    let drv = &d.list[parent.drive];
    match &parent.dir {
        Dir::Host(folder) => {
            let name = validate_new_host_name(last)?;
            let entries = listing(drv, &parent.dir)?;
            let existing = find_host_entry(&entries, &name).cloned();
            let loc = existing
                .as_ref()
                .and_then(|e| e.loc(parent.drive))
                .unwrap_or_else(|| Loc::Host(folder.join(&name)));
            Ok((parent.drive, loc, existing))
        }
        Dir::Memory(_) => {
            if last.contains(&b'*') || last.contains(&b'?') {
                return Err(3);
            }
            let name83 = fcb(last);
            if name83[0] == b' ' || name83[0] == b'.' {
                return Err(3);
            }
            // Validate 8.3 rules cleanly using dos_files rules.
            let mut rel = parent.dos_path;
            rel.push(name83);
            let entries = listing(drv, &parent.dir)?;
            let existing = entries.into_iter().find(|e| e.name == name83);
            let valid_83 = files::short(&crate::dos_encoding::decode(last)) == Some(name83);
            if !valid_83 && existing.is_none() {
                return Err(3);
            }
            Ok((parent.drive, Loc::Memory(parent.drive, rel), existing))
        }
        _ => Err(5),
    }
}

fn glob_match(pattern: &str, name: &str) -> bool {
    fn inner(p: &[char], n: &[char]) -> bool {
        match (p.first(), n.first()) {
            (None, None) => true,
            (Some(&'*'), _) => {
                inner(&p[1..], n) || (!n.is_empty() && inner(p, &n[1..]))
            }
            (Some(&'?'), Some(_)) => inner(&p[1..], &n[1..]),
            (Some(a), Some(b)) if a == b => inner(&p[1..], &n[1..]),
            _ => false,
        }
    }
    let pl = pattern.to_lowercase();
    let nl = name.to_lowercase();
    if pl == "*.*" || pl == "*" {
        return true;
    }
    let pc: Vec<char> = pl.chars().collect();
    let nc: Vec<char> = nl.chars().collect();
    if inner(&pc, &nc) {
        return true;
    }
    // Classic DOS rule: "FOO.*" also matches extensionless "FOO".
    if let Some(prefix) = pl.strip_suffix(".*") {
        let p2: Vec<char> = prefix.chars().collect();
        if !nl.contains('.') && inner(&p2, &nc) {
            return true;
        }
    }
    false
}

fn lfn_matches(pattern_raw: &[u8], long_bytes: &[u8], short_bytes: Option<&[u8]>) -> bool {
    let pattern = crate::dos_encoding::decode(pattern_raw);
    let long_str = crate::dos_encoding::decode(long_bytes);
    if glob_match(&pattern, &long_str) {
        return true;
    }
    if let Some(short_b) = short_bytes {
        let short_str = crate::dos_encoding::decode(short_b);
        return glob_match(&pattern, &short_str);
    }
    false
}

fn write_find_data(
    p: &mut Loaded,
    entry: &LfnSearchEntry,
    time_fmt: u16,
) -> Result<(), String> {
    let mut out = [0u8; 318];
    out[0..4].copy_from_slice(&u32::from(entry.attr).to_le_bytes());
    let (ts0, ts1) = if time_fmt == 1 {
        let mut ts = [0u8; 8];
        ts[0..2].copy_from_slice(&entry.time.to_le_bytes());
        ts[2..4].copy_from_slice(&entry.date.to_le_bytes());
        (ts, ts)
    } else {
        let ft = entry.filetime.to_le_bytes();
        (ft, ft)
    };
    out[4..12].copy_from_slice(&ts0);
    out[12..20].copy_from_slice(&ts0);
    out[20..28].copy_from_slice(&ts1);
    out[32..36].copy_from_slice(&entry.size.to_le_bytes());
    let long_len = entry.long_name.len().min(259);
    out[44..44 + long_len].copy_from_slice(&entry.long_name[..long_len]);
    if let Some(short_b) = &entry.short_name {
        let short_len = short_b.len().min(13);
        out[304..304 + short_len].copy_from_slice(&short_b[..short_len]);
    }
    store(p, SegmentRegister::Es, p.cpu.regs[7] & 0xffff, &out)
}

fn find_first(p: &mut Loaded) -> Result<(), String> {
    let raw = lfn_asciiz(p, SegmentRegister::Ds, p.cpu.regs[2] & 0xffff)?;
    if raw.is_empty() {
        fail(p, 2);
        return Ok(());
    }
    let cl = (p.cpu.regs[1] & 0xff) as u8;
    let ch = ((p.cpu.regs[1] >> 8) & 0xff) as u8;
    let time_fmt = (p.cpu.regs[6] & 0xffff) as u16;

    let (parent, pattern) = match resolve_lfn_parent(&p.dos.drive, &raw) {
        Ok(found) => found,
        Err(code) => {
            fail(p, code);
            return Ok(());
        }
    };
    let drv = &p.dos.drive.list[parent.drive];

    let mut matched = Vec::new();
    if cl == 0x08 || ch == 0x08 {
        if let Source::Image(tree) = &drv.source {
            if let Some(label) = tree.label {
                let mut name = label[..8].to_vec();
                if label[8..].iter().any(|&c| c != b' ') {
                    name.push(b'.');
                    name.extend(&label[8..]);
                }
                while name.last() == Some(&b' ') {
                    name.pop();
                }
                if lfn_matches(pattern, &name, None) {
                    matched.push(LfnSearchEntry {
                        long_name: name,
                        short_name: None,
                        attr: 0x08,
                        size: 0,
                        time: 0,
                        date: 0x21,
                        filetime: dos_to_filetime(0, 0x21, 0),
                    });
                }
            }
        }
    } else {
        let mut candidates: Vec<(Entry, Vec<u8>, Option<Vec<u8>>, u64)> = Vec::new();
        if !parent.dos_path.is_empty() {
            let ft = dos_to_filetime(parent.dir_stamp.0, parent.dir_stamp.1, 0);
            for dots in [&b"."[..], b".."] {
                let e = Entry {
                    name: fcb(dots),
                    attr: 0x10,
                    size: 0,
                    time: parent.dir_stamp.0,
                    date: parent.dir_stamp.1,
                    node: Node::Host(PathBuf::new()),
                };
                candidates.push((e, dots.to_vec(), None, ft));
            }
        }
        match listing(drv, &parent.dir) {
            Ok(list) => {
                for e in list {
                    let long_b = entry_long_bytes(&e);
                    let short_b = entry_short_alt(&e);
                    let ft = entry_filetime(&e);
                    candidates.push((e, long_b, short_b, ft));
                }
            }
            Err(code) => {
                fail(p, code);
                return Ok(());
            }
        }
        for (e, long_b, short_b, ft) in candidates {
            if (e.attr & ch) != ch || (e.attr & 0x16 & !cl) != 0 {
                continue;
            }
            let short_match = if e.name == fcb(b".") || e.name == fcb(b"..") {
                None
            } else {
                Some(display(&e.name))
            };
            if !lfn_matches(pattern, &long_b, short_match.as_deref()) {
                continue;
            }
            matched.push(LfnSearchEntry {
                long_name: long_b,
                short_name: short_b,
                attr: e.attr,
                size: e.size,
                time: e.time,
                date: e.date,
                filetime: ft,
            });
        }
    }

    let Some(first) = matched.first().cloned() else {
        fail(p, 2);
        return Ok(());
    };
    let slot = if let Some(pos) = p.dos.drive.lfn_searches.iter().position(Option::is_none) {
        pos
    } else if p.dos.drive.lfn_searches.len() < MAX_LFN_SEARCHES {
        p.dos.drive.lfn_searches.push(None);
        p.dos.drive.lfn_searches.len() - 1
    } else {
        fail(p, 4);
        return Ok(());
    };
    write_find_data(p, &first, time_fmt)?;
    p.dos.drive.lfn_searches[slot] = Some(LfnSearch {
        drive: parent.drive,
        entries: matched,
        index: 1,
    });
    set_ax(p, (slot + 1) as u32);
    p.cpu.regs[1] &= 0xffff0000;
    ok(p);
    Ok(())
}

fn find_next(p: &mut Loaded) -> Result<(), String> {
    let handle = (p.cpu.regs[3] & 0xffff) as usize;
    let time_fmt = (p.cpu.regs[6] & 0xffff) as u16;
    let Some(entry) = handle
        .checked_sub(1)
        .and_then(|i| p.dos.drive.lfn_searches.get(i))
        .and_then(Option::as_ref)
        .and_then(|s| s.entries.get(s.index).cloned())
    else {
        let valid = handle
            .checked_sub(1)
            .and_then(|i| p.dos.drive.lfn_searches.get(i))
            .is_some_and(Option::is_some);
        fail(p, if valid { 0x12 } else { 6 });
        return Ok(());
    };
    write_find_data(p, &entry, time_fmt)?;
    if let Some(Some(search)) = p.dos.drive.lfn_searches.get_mut(handle - 1) {
        search.index += 1;
    }
    p.cpu.regs[1] &= 0xffff0000;
    ok(p);
    Ok(())
}

fn find_close(p: &mut Loaded) -> Result<(), String> {
    let handle = (p.cpu.regs[3] & 0xffff) as usize;
    let Some(slot) = handle
        .checked_sub(1)
        .and_then(|i| p.dos.drive.lfn_searches.get_mut(i))
        .filter(|s| s.is_some())
    else {
        fail(p, 6);
        return Ok(());
    };
    *slot = None;
    ok(p);
    Ok(())
}

fn volume_info(p: &mut Loaded) -> Result<(), String> {
    let raw = lfn_asciiz(p, SegmentRegister::Ds, p.cpu.regs[2] & 0xffff)?;
    let buf_len = (p.cpu.regs[1] & 0xffff) as usize;
    let drive = if raw.len() >= 2 && raw[1] == b':' {
        let Some(i) = p
            .dos
            .drive
            .list
            .iter()
            .position(|d| raw[0].eq_ignore_ascii_case(&d.letter))
        else {
            fail(p, 0x0f);
            return Ok(());
        };
        i
    } else if raw.is_empty() || matches!(raw[0], b'\\' | b'/') {
        p.dos.drive.current
    } else {
        fail(p, 3);
        return Ok(());
    };
    let host_drive = matches!(p.dos.drive.list[drive].source, Source::Host(_));
    let fs_name = b"FAT\0";
    if buf_len > 0 {
        let mut written = fs_name[..fs_name.len().min(buf_len)].to_vec();
        if written.last() != Some(&0) {
            *written.last_mut().unwrap() = 0;
        }
        store(p, SegmentRegister::Es, p.cpu.regs[7] & 0xffff, &written)?;
    }
    let (bx, cx, dx) = if host_drive {
        (0x4006u32, 255u32, 260u32)
    } else {
        (0x0000u32, 12u32, 67u32)
    };
    p.cpu.regs[3] = (p.cpu.regs[3] & 0xffff0000) | bx;
    p.cpu.regs[1] = (p.cpu.regs[1] & 0xffff0000) | cx;
    p.cpu.regs[2] = (p.cpu.regs[2] & 0xffff0000) | dx;
    ok(p);
    Ok(())
}

fn get_cwd(p: &mut Loaded) -> Result<(), String> {
    let dl = p.cpu.regs[2] as u8;
    let Some(drive) = p.dos.drive.number(dl) else {
        fail(p, 0x0f);
        return Ok(());
    };
    let d = &p.dos.drive.list[drive];
    let text = match &d.source {
        Source::Host(_) => {
            let Ok((dos_path, long_parts, ..)) = resolve_dir_parts(d, true, &[]) else {
                fail(p, 3);
                return Ok(());
            };
            let _ = dos_path;
            let mut out = long_parts.join(&b'\\');
            out.push(0);
            out
        }
        _ => {
            let mut out = d
                .cwd
                .iter()
                .map(display)
                .collect::<Vec<_>>()
                .join(&b'\\');
            out.push(0);
            out
        }
    };
    store_ds(p, p.cpu.regs[6] & 0xffff, &text)?;
    ok(p);
    Ok(())
}

fn chdir(p: &mut Loaded) -> Result<(), String> {
    let raw = lfn_asciiz(p, SegmentRegister::Ds, p.cpu.regs[2] & 0xffff)?;
    if raw.is_empty() {
        fail(p, 3);
        return Ok(());
    }
    match resolve_lfn_dir(&p.dos.drive, &raw) {
        Ok(resolved) => {
            p.dos.drive.list[resolved.drive].cwd = resolved.dos_path;
            ok(p);
        }
        Err(code) => fail(p, code),
    }
    Ok(())
}

fn canonical(p: &mut Loaded) -> Result<(), String> {
    let raw = lfn_asciiz(p, SegmentRegister::Ds, p.cpu.regs[6] & 0xffff)?;
    let mode = p.cpu.regs[1] as u8;
    let ch = (p.cpu.regs[1] >> 8) as u8;
    let _ = ch;
    if raw.is_empty() {
        fail(p, 3);
        return Ok(());
    }
    if raw.len() >= 2
        && raw[1] == b':'
        && !p
            .dos
            .drive
            .list
            .iter()
            .any(|d| raw[0].eq_ignore_ascii_case(&d.letter))
    {
        fail(p, 0x0f);
        return Ok(());
    }
    let di = p.cpu.regs[7] & 0xffff;
    match mode {
        0 => {
            let (drive, abs, rest) = match drive_and_rest(&p.dos.drive, &raw) {
                Ok(v) => v,
                Err(code) => {
                    fail(p, code);
                    return Ok(());
                }
            };
            let drv = &p.dos.drive.list[drive];
            let base_parts: Vec<Vec<u8>> = if abs {
                Vec::new()
            } else {
                match resolve_dir_parts(drv, true, &[]) {
                    Ok((_, parts, ..)) => parts,
                    Err(code) => {
                        fail(p, code);
                        return Ok(());
                    }
                }
            };
            let mut logical: Vec<Vec<u8>> = base_parts;
            for part in rest.split(|&c| c == b'\\' || c == b'/') {
                match part {
                    b"" | b"." => {}
                    b".." => {
                        if logical.pop().is_none() {
                            fail(p, 3);
                            return Ok(());
                        }
                    }
                    other => logical.push(other.to_vec()),
                }
            }
            // If existing components match real host entries on a Host drive, use their exact casing/names.
            let mut full_parts: Vec<Vec<u8>> = Vec::with_capacity(logical.len());
            if let Source::Host(root) = &drv.source {
                let mut cur_dir = Some(root.clone());
                for seg in &logical {
                    let mut chosen = None;
                    if let Some(ref dir) = cur_dir {
                        if let Ok(entries) = host_listing(dir) {
                            let decoded = crate::dos_encoding::decode(seg);
                            if let Some(entry) = find_host_entry(&entries, &decoded) {
                                chosen = Some(entry_long_bytes(entry));
                                cur_dir = entry.dir().and_then(|d| match d {
                                    Dir::Host(p) => Some(p),
                                    _ => None,
                                });
                            } else {
                                cur_dir = None;
                            }
                        } else {
                            cur_dir = None;
                        }
                    }
                    full_parts.push(chosen.unwrap_or_else(|| seg.clone()));
                }
            } else {
                for seg in &logical {
                    full_parts.push(display(&fcb(seg)));
                }
            }
            let mut full = vec![drv.letter, b':', b'\\'];
            full.extend(full_parts.join(&b'\\'));
            full.push(0);
            if full.len() > 261 {
                fail(p, 3);
                return Ok(());
            }
            store(p, SegmentRegister::Es, di, &full)?;
            set_ax(p, 0);
            ok(p);
        }
        1 | 2 => {
            let (drive, dos_parts, long_parts) = match resolve_lfn_dir(&p.dos.drive, &raw) {
                Ok(dir_res) => (dir_res.drive, dir_res.dos_path, dir_res.long_parts),
                Err(_) => match lookup_lfn(&p.dos.drive, &raw) {
                    Ok((drive, mut parent, entry)) => {
                        parent.dos_path.push(entry.name);
                        parent.long_parts.push(entry_long_bytes(&entry));
                        (drive, parent.dos_path, parent.long_parts)
                    }
                    Err(code) => {
                        fail(p, code);
                        return Ok(());
                    }
                },
            };
            let mut full = vec![p.dos.drive.list[drive].letter, b':', b'\\'];
            if mode == 1 {
                full.extend(
                    dos_parts
                        .iter()
                        .map(display)
                        .collect::<Vec<_>>()
                        .join(&b'\\'),
                );
            } else {
                full.extend(long_parts.join(&b'\\'));
            }
            full.push(0);
            if full.len() > 261 {
                fail(p, 3);
                return Ok(());
            }
            store(p, SegmentRegister::Es, di, &full)?;
            set_ax(p, 0);
            ok(p);
        }
        _ => fail(p, 1),
    }
    Ok(())
}

fn file_attr(p: &mut Loaded) -> Result<(), String> {
    let bl = p.cpu.regs[3] as u8;
    let raw = lfn_asciiz(p, SegmentRegister::Ds, p.cpu.regs[2] & 0xffff)?;
    if raw.is_empty() {
        fail(p, 3);
        return Ok(());
    }
    let (drive, entry_opt) = if let Ok(dir_res) = resolve_lfn_dir(&p.dos.drive, &raw) {
        if dir_res.dos_path.is_empty() {
            (
                dir_res.drive,
                Some(Entry {
                    name: [b' '; 11],
                    attr: 0x10,
                    size: 0,
                    time: 0,
                    date: 0x21,
                    node: Node::Host(PathBuf::new()),
                }),
            )
        } else {
            match lookup_lfn(&p.dos.drive, &raw) {
                Ok((d, _, e)) => (d, Some(e)),
                Err(code) => {
                    fail(p, code);
                    return Ok(());
                }
            }
        }
    } else {
        match lookup_lfn(&p.dos.drive, &raw) {
            Ok((d, _, e)) => (d, Some(e)),
            Err(code) => {
                fail(p, code);
                return Ok(());
            }
        }
    };
    let entry = entry_opt.unwrap();
    match bl {
        0 => {
            p.cpu.regs[1] = (p.cpu.regs[1] & 0xffff0000) | u32::from(entry.attr);
            ok(p);
        }
        1 | 5 | 7 => fail(p, 5),
        2 => {
            if entry.attr & 0x18 != 0 {
                fail(p, 2);
                return Ok(());
            }
            set_ax(p, entry.size & 0xffff);
            p.cpu.regs[2] = (p.cpu.regs[2] & 0xffff0000) | (entry.size >> 16);
            ok(p);
        }
        3 => {
            let when = (p.cpu.regs[1] as u16, p.cpu.regs[7] as u16);
            match entry.loc(drive) {
                Some(loc) => match p.dos.drive.set_time(&loc, when) {
                    Ok(()) => ok(p),
                    Err(code) => fail(p, code),
                },
                None => fail(p, 5),
            }
        }
        4 => {
            p.cpu.regs[1] = (p.cpu.regs[1] & 0xffff0000) | u32::from(entry.time);
            p.cpu.regs[7] = (p.cpu.regs[7] & 0xffff0000) | u32::from(entry.date);
            ok(p);
        }
        6 => {
            p.cpu.regs[1] &= 0xffff0000;
            p.cpu.regs[7] = (p.cpu.regs[7] & 0xffff0000) | u32::from(entry.date);
            ok(p);
        }
        8 => {
            p.cpu.regs[1] = (p.cpu.regs[1] & 0xffff0000) | u32::from(entry.time);
            p.cpu.regs[7] = (p.cpu.regs[7] & 0xffff0000) | u32::from(entry.date);
            p.cpu.regs[6] &= 0xffff0000;
            ok(p);
        }
        _ => fail(p, 1),
    }
    Ok(())
}

fn mkdir(p: &mut Loaded) -> Result<(), String> {
    let raw = lfn_asciiz(p, SegmentRegister::Ds, p.cpu.regs[2] & 0xffff)?;
    if raw.is_empty() {
        fail(p, 3);
        return Ok(());
    }
    let result = place_lfn(&p.dos.drive, &raw).and_then(|(_, loc, existing)| {
        if existing.is_some() {
            return Err(5);
        }
        p.dos.drive.make_dir(&loc)
    });
    match result {
        Ok(()) => ok(p),
        Err(code) => fail(p, code),
    }
    Ok(())
}

fn rmdir(p: &mut Loaded) -> Result<(), String> {
    let raw = lfn_asciiz(p, SegmentRegister::Ds, p.cpu.regs[2] & 0xffff)?;
    if raw.is_empty() {
        fail(p, 3);
        return Ok(());
    }
    let result = lookup_lfn(&p.dos.drive, &raw).and_then(|(drive, mut parent, entry)| {
        let loc = entry.loc(drive).filter(|_| entry.attr & 0x10 != 0).ok_or(5u16)?;
        parent.dos_path.push(entry.name);
        if p.dos.drive.list[drive].cwd.starts_with(&parent.dos_path) {
            return Err(0x10);
        }
        p.dos.drive.remove(&loc, true)
    });
    match result {
        Ok(()) => ok(p),
        Err(code) => fail(p, code),
    }
    Ok(())
}

fn delete(p: &mut Loaded) -> Result<(), String> {
    let raw = lfn_asciiz(p, SegmentRegister::Ds, p.cpu.regs[2] & 0xffff)?;
    if raw.is_empty() {
        fail(p, 3);
        return Ok(());
    }
    let allow_wild = (p.cpu.regs[6] & 0xffff) == 1;
    let cl = (p.cpu.regs[1] & 0xff) as u8;
    let ch = ((p.cpu.regs[1] >> 8) & 0xff) as u8;
    if !allow_wild {
        let result = lookup_lfn(&p.dos.drive, &raw).and_then(|(drive, _, entry)| {
            let loc = entry.loc(drive).filter(|_| entry.attr & 0x19 == 0).ok_or(5u16)?;
            p.dos.drive.remove(&loc, false)
        });
        match result {
            Ok(()) => ok(p),
            Err(code) => fail(p, code),
        }
        return Ok(());
    }
    let result = resolve_lfn_parent(&p.dos.drive, &raw).and_then(|(parent, pattern)| {
        let drv = &p.dos.drive.list[parent.drive];
        let entries = listing(drv, &parent.dir)?;
        let mut to_remove = Vec::new();
        for e in entries {
            if e.attr & 0x18 != 0 {
                continue;
            }
            if (e.attr & ch) != ch || (e.attr & 0x16 & !cl) != 0 {
                continue;
            }
            let long_b = entry_long_bytes(&e);
            let short_b = display(&e.name);
            if !lfn_matches(pattern, &long_b, Some(&short_b)) {
                continue;
            }
            let Some(loc) = e.loc(parent.drive) else {
                return Err(5);
            };
            if e.attr & 1 != 0 {
                return Err(5);
            }
            to_remove.push(loc);
        }
        if to_remove.is_empty() {
            return Err(2);
        }
        for loc in to_remove {
            p.dos.drive.remove(&loc, false)?;
        }
        Ok(())
    });
    match result {
        Ok(()) => ok(p),
        Err(code) => fail(p, code),
    }
    Ok(())
}

fn rename(p: &mut Loaded) -> Result<(), String> {
    let old_raw = lfn_asciiz(p, SegmentRegister::Ds, p.cpu.regs[2] & 0xffff)?;
    let new_raw = lfn_asciiz(p, SegmentRegister::Es, p.cpu.regs[7] & 0xffff)?;
    if old_raw.is_empty() || new_raw.is_empty() {
        fail(p, 3);
        return Ok(());
    }
    let result = lookup_lfn(&p.dos.drive, &old_raw).and_then(|(drive, _, entry)| {
        let from = entry.loc(drive).filter(|_| entry.attr & 0x0d == 0).ok_or(5u16)?;
        let (to_drive, to, existing) = place_lfn(&p.dos.drive, &new_raw)?;
        if to_drive != drive {
            return Err(0x11);
        }
        if let Some(ex) = existing {
            let same_case_change = match (&from, ex.loc(to_drive)) {
                (Loc::Host(a), Some(Loc::Host(b))) => a == &b && to != from,
                _ => false,
            };
            if !same_case_change {
                return Err(5);
            }
        }
        p.dos.drive.rename(&from, &to)
    });
    match result {
        Ok(()) => ok(p),
        Err(code) => fail(p, code),
    }
    Ok(())
}

fn extended_open(p: &mut Loaded) -> Result<(), String> {
    let raw = lfn_asciiz(p, SegmentRegister::Ds, p.cpu.regs[6] & 0xffff)?;
    let bx = (p.cpu.regs[3] & 0xffff) as u16;
    let dx = (p.cpu.regs[2] & 0xffff) as u16;
    let mode = (bx & 7) as u8;
    // Bit 2 of BL can mean read-only without updating last-access in DOS 7; map mode 4 to 0.
    let access_mode = if mode == 4 { 0 } else { mode };
    if access_mode > 2 {
        fail(p, 0x0c);
        return Ok(());
    }
    let exist_act = dx & 0x0f;
    let miss_act = (dx >> 4) & 0x0f;
    if exist_act > 2 || miss_act > 1 {
        fail(p, 1);
        return Ok(());
    }
    if serial::device_name(&raw, b"NUL") {
        if exist_act == 0 {
            fail(p, 0x50);
            return Ok(());
        }
        device(p, NUL)?;
        if p.cpu.eflags & 1 == 0 {
            p.cpu.regs[1] = (p.cpu.regs[1] & 0xffff0000) | 1;
        }
        return Ok(());
    }
    if serial::is_device(p, &raw) {
        if exist_act == 0 {
            fail(p, 0x50);
            return Ok(());
        }
        device(p, 3)?;
        if p.cpu.eflags & 1 == 0 {
            p.cpu.regs[1] = (p.cpu.regs[1] & 0xffff0000) | 1;
        }
        return Ok(());
    }
    if p.com1.is_some() && serial::device_name(&raw, b"CON") {
        if exist_act == 0 {
            fail(p, 0x50);
            return Ok(());
        }
        device(p, 1)?;
        if p.cpu.eflags & 1 == 0 {
            p.cpu.regs[1] = (p.cpu.regs[1] & 0xffff0000) | 1;
        }
        return Ok(());
    }
    let own = !raw.is_empty() && base_name(&raw) == p.dos.own_name;
    if own && exist_act != 0 {
        if access_mode != 0 {
            return Err("opening the program file for writing is not modelled".into());
        }
        if p.dos.own_file.is_empty() {
            fail(p, 2);
            return Ok(());
        }
        let drive = p.dos.drive.letter();
        install(
            p,
            OpenFile {
                sft: 0,
                data: p.dos.own_file.clone(),
                pos: 0,
                drive,
                loc: None,
                mode: 0,
                written: false,
                stamp: (0, 0x21),
            },
        )?;
        if p.cpu.eflags & 1 == 0 {
            p.cpu.regs[1] = (p.cpu.regs[1] & 0xffff0000) | 1;
        }
        return Ok(());
    }
    if raw.is_empty() {
        fail(p, 3);
        return Ok(());
    }
    let (open_file, action) = match lookup_lfn(&p.dos.drive, &raw) {
        Ok((drive, _, entry)) => {
            if entry.attr & 0x18 != 0 {
                fail(p, 5);
                return Ok(());
            }
            let letter = p.dos.drive.list[drive].letter;
            match exist_act {
                0 => {
                    fail(p, 0x50);
                    return Ok(());
                }
                1 => {
                    if access_mode != 0 && (matches!(entry.node, Node::Image(_)) || entry.attr & 1 != 0)
                    {
                        fail(p, 5);
                        return Ok(());
                    }
                    let data = read_entry(&p.dos.drive.list[drive], &entry)?;
                    (
                        OpenFile {
                            sft: 0,
                            data,
                            pos: 0,
                            drive: letter,
                            loc: entry.loc(drive),
                            mode: access_mode,
                            written: false,
                            stamp: (entry.time, entry.date),
                        },
                        1u32,
                    )
                }
                2 => {
                    if matches!(entry.node, Node::Image(_)) || entry.attr & 1 != 0 {
                        fail(p, 5);
                        return Ok(());
                    }
                    let Some(loc) = entry.loc(drive) else {
                        fail(p, 5);
                        return Ok(());
                    };
                    if let Err(code) = p.dos.drive.make_file(&loc) {
                        fail(p, code);
                        return Ok(());
                    }
                    (
                        OpenFile {
                            sft: 0,
                            data: Arc::new(Vec::new()),
                            pos: 0,
                            drive: letter,
                            loc: Some(loc),
                            mode: access_mode,
                            written: false,
                            stamp: (0, 0x21),
                        },
                        3u32,
                    )
                }
                _ => {
                    fail(p, 1);
                    return Ok(());
                }
            }
        }
        Err(2) => {
            if miss_act == 0 {
                fail(p, 2);
                return Ok(());
            }
            let (drive, loc, existing) = match place_lfn(&p.dos.drive, &raw) {
                Ok(v) => v,
                Err(code) => {
                    fail(p, code);
                    return Ok(());
                }
            };
            if existing.is_some() {
                fail(p, 0x50);
                return Ok(());
            }
            if let Err(code) = p.dos.drive.make_file(&loc) {
                fail(p, code);
                return Ok(());
            }
            let letter = p.dos.drive.list[drive].letter;
            (
                OpenFile {
                    sft: 0,
                    data: Arc::new(Vec::new()),
                    pos: 0,
                    drive: letter,
                    loc: Some(loc),
                    mode: access_mode,
                    written: false,
                    stamp: (0, 0x21),
                },
                2u32,
            )
        }
        Err(code) => {
            fail(p, code);
            return Ok(());
        }
    };
    install(p, open_file)?;
    if p.cpu.eflags & 1 == 0 {
        p.cpu.regs[1] = (p.cpu.regs[1] & 0xffff0000) | action;
    }
    Ok(())
}

fn file_info_by_handle(p: &mut Loaded) -> Result<(), String> {
    let handle = (p.cpu.regs[3] & 0xffff) as u16;
    let Some(sft) = files::system_file(p, handle)? else {
        fail(p, 6);
        return Ok(());
    };
    let Some(file) = p.dos.drive.files.iter().find(|f| f.sft == sft) else {
        fail(p, 6);
        return Ok(());
    };
    let ft = file
        .loc
        .as_ref()
        .and_then(|loc| {
            if let Loc::Host(path) = loc {
                std::fs::metadata(path).and_then(|m| m.modified()).ok().map(system_time_to_filetime)
            } else {
                let (t, d) = p.dos.drive.time(loc)?;
                Some(dos_to_filetime(t, d, 0))
            }
        })
        .unwrap_or_else(|| dos_to_filetime(file.stamp.0, file.stamp.1, 0));
    let mut out = [0u8; 52];
    out[0..4].copy_from_slice(&0x20u32.to_le_bytes());
    out[4..12].copy_from_slice(&ft.to_le_bytes());
    out[12..20].copy_from_slice(&ft.to_le_bytes());
    out[20..28].copy_from_slice(&ft.to_le_bytes());
    out[36..40].copy_from_slice(&(file.data.len() as u32).to_le_bytes());
    out[40..44].copy_from_slice(&1u32.to_le_bytes());
    out[48..52].copy_from_slice(&u32::from(sft).to_le_bytes());
    store_ds(p, p.cpu.regs[2] & 0xffff, &out)?;
    ok(p);
    Ok(())
}

fn convert_time(p: &mut Loaded) -> Result<(), String> {
    let bl = p.cpu.regs[3] as u8;
    match bl {
        0 => {
            let bytes = load_bytes(p, SegmentRegister::Ds, p.cpu.regs[6] & 0xffff, 8)?;
            let ft = u64::from_le_bytes(bytes.try_into().unwrap());
            let (time, date, bh) = filetime_to_dos(ft);
            p.cpu.regs[1] = (p.cpu.regs[1] & 0xffff0000) | u32::from(time);
            p.cpu.regs[2] = (p.cpu.regs[2] & 0xffff0000) | u32::from(date);
            p.cpu.regs[3] = (p.cpu.regs[3] & 0xffff00ff) | (u32::from(bh) << 8);
            ok(p);
        }
        1 => {
            let bh = (p.cpu.regs[3] >> 8) as u8;
            let time = p.cpu.regs[1] as u16;
            let date = p.cpu.regs[2] as u16;
            let ft = dos_to_filetime(time, date, bh);
            store(p, SegmentRegister::Es, p.cpu.regs[7] & 0xffff, &ft.to_le_bytes())?;
            ok(p);
        }
        _ => fail(p, 1),
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/unit/dos_lfn.rs"]
mod tests;
