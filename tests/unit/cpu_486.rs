//! 486DX unit tests: CpuModel profile, EFLAGS.AC (bit 18) across
//! PUSHFD/POPFD/IRETD/TSS task switches, 486DX instructions (BSWAP, XADD,
//! CMPXCHG, INVLPG, INVD, WBINVD), generated-Rust parity, and x86-64 oracle.
use super::*;
use crate::address::{CpuModel, SegmentAccess, SegmentCache};
use crate::dos::LoadedOps;
use crate::interrupt::{deliver, Event};
use crate::task::{switch, Entry};

fn setup_cpu(bytes: &[u8], cpu_model: CpuModel, big_code: bool, big_stack: bool) -> (guest::Image, Ram, Cpu) {
    let (mut image, _) = guest::Image::load(bytes).unwrap();
    image.cpu_model = cpu_model;
    let mut ram = Ram::new(0x20000);
    ram.write_bytes(0x100, bytes).unwrap();
    if big_code {
        image.real_mode = false;
        image.msw = 0x11;
        image.segments[1] = SegmentCache {
            base: 0,
            limit: 0xffff_ffff,
            access: SegmentAccess::Code { readable: true, conforming: false, big: true, dpl: 0 },
        };
    }
    if big_stack {
        image.segments[2] = SegmentCache {
            base: 0,
            limit: 0xffff_ffff,
            access: SegmentAccess::Data { writable: true, expand_down: false, big: true, dpl: 0 },
        };
    } else {
        image.segments[2] = SegmentCache::real_mode(0);
    }
    let cpu = Cpu {
        regs: [0, 0, 0, 0, 0x8000, 0, 0, 0],
        eflags: 0x0202,
        undefined_flags: 0,
        eip: 0x100,
    };
    (image, ram, cpu)
}

fn step_one(image: &mut guest::Image, ram: &mut Ram, cpu: &mut Cpu) -> Result<(), ExecError> {
    let decoded = image.decode(ram, cpu.eip).unwrap();
    let guest::Operation::Pil { program, dst, src, flow, address } = decoded.operation else {
        panic!("expected PIL operation, got {:?}", decoded.label);
    };
    let mut machine = image.segment_state();
    let res = execute_machine(&program, cpu, dst, src, flow, ram, address, &mut machine);
    if res.is_ok() {
        image.set_segment_state(machine);
    }
    res
}

#[test]
fn eflags_ac_probe_distinguishes_386_from_486dx() {
    // Classic 386 vs 486 detection sequence in 32-bit operand mode:
    // 66 9C             pushfd
    // 66 58             pop eax
    // 66 89 C1          mov ecx, eax
    // 66 35 00 00 04 00 xor eax, 0x40000
    // 66 50             push eax
    // 66 9D             popfd
    // 66 9C             pushfd
    // 66 58             pop eax
    // 66 31 C8          xor eax, ecx
    // 66 51             push ecx
    // 66 9D             popfd
    let probe = [
        0x66, 0x9c,
        0x66, 0x58,
        0x66, 0x89, 0xc1,
        0x66, 0x35, 0x00, 0x00, 0x04, 0x00,
        0x66, 0x50,
        0x66, 0x9d,
        0x66, 0x9c,
        0x66, 0x58,
        0x66, 0x31, 0xc8,
        0x66, 0x51,
        0x66, 0x9d,
    ];
    for (model, expected_ac) in [(CpuModel::I386, 0u32), (CpuModel::I486Dx, 0x40000u32)] {
        let (mut image, mut ram, mut cpu) = setup_cpu(&probe, model, false, false);
        let end = 0x100 + probe.len() as u32;
        while cpu.eip < end {
            step_one(&mut image, &mut ram, &mut cpu).unwrap();
        }
        assert_eq!(cpu.regs[0] & 0x40000, expected_ac, "model={model:?}");
        // Final PUSH ECX; POPFD restored AC back to 0.
        assert_eq!(cpu.eflags & 0x40000, 0, "model={model:?}");
    }
}

#[test]
fn popf16_and_iret16_preserve_ac_while_popfd_toggles_it() {
    // 66 68 02 02 04 00  push dword 0x40202
    // 66 9D              popfd
    // 68 46 02           push word 0x0246
    // 9D                 popf (16-bit: preserves AC)
    // 9C                 pushf (16-bit: pushes low 16 bits only)
    let code = [
        0x66, 0x68, 0x02, 0x02, 0x04, 0x00,
        0x66, 0x9d,
        0x68, 0x46, 0x02,
        0x9d,
        0x9c,
    ];
    let (mut image, mut ram, mut cpu) = setup_cpu(&code, CpuModel::I486Dx, false, false);
    let end = 0x100 + code.len() as u32;
    while cpu.eip < end {
        step_one(&mut image, &mut ram, &mut cpu).unwrap();
    }
    assert_ne!(cpu.eflags & 0x40000, 0, "16-bit POPF must preserve AC");
    assert_eq!(cpu.eflags & 0xff, 0x46);
    let pushed16 = ram.load(cpu.regs[4], MemWidth::Word).unwrap();
    assert_eq!(pushed16 & 0xff, 0x46);

    // 16-bit IRET preserves AC on 486DX too.
    let (mut image, mut ram, mut cpu) = setup_cpu(&[0xcf], CpuModel::I486Dx, false, false);
    cpu.eflags |= 0x40000;
    ram.store(0x8000, MemWidth::Word, 0x0150).unwrap();
    ram.store(0x8002, MemWidth::Word, 0x0000).unwrap();
    ram.store(0x8004, MemWidth::Word, 0x0202).unwrap();
    step_one(&mut image, &mut ram, &mut cpu).unwrap();
    assert_eq!(cpu.eip, 0x150);
    assert_ne!(cpu.eflags & 0x40000, 0, "16-bit IRET must preserve AC");
}

#[test]
fn iretd_restores_ac_in_real_protected_and_v86_modes() {
    // Real mode IRETD (66 CF)
    for (model, expect_set) in [(CpuModel::I386, false), (CpuModel::I486Dx, true)] {
        let (mut image, mut ram, mut cpu) = setup_cpu(&[0x66, 0xcf], model, false, false);
        ram.store(0x8000, MemWidth::Dword, 0x0120).unwrap();
        ram.store(0x8004, MemWidth::Dword, 0x0000).unwrap();
        ram.store(0x8008, MemWidth::Dword, 0x0004_0202).unwrap();
        step_one(&mut image, &mut ram, &mut cpu).unwrap();
        assert_eq!(cpu.eip, 0x120);
        assert_eq!(cpu.eflags & 0x40000 != 0, expect_set, "real IRETD {model:?}");
    }

    // Protected mode IRETD (CF in 32-bit CS) at CPL 0 and CPL 3
    for dpl in [0u8, 3u8] {
        for (model, expect_set) in [(CpuModel::I386, false), (CpuModel::I486Dx, true)] {
            let (mut image, mut ram, mut cpu) = setup_cpu(&[0xcf], model, true, true);
            let sel = 0x08 | u16::from(dpl);
            let dsel = 0x10 | u16::from(dpl);
            image.selectors = [dsel, sel, dsel, dsel, dsel, dsel];
            image.segments[1].access = SegmentAccess::Code { readable: true, conforming: false, big: true, dpl };
            image.segments[2].access = SegmentAccess::Data { writable: true, expand_down: false, big: true, dpl };
            // GDT with descriptor 08h (code) and 10h (data) at DPL `dpl`
            image.tables[0] = (0x20, 0x1000);
            let access_code = 0x9au64 | (u64::from(dpl) << 5);
            let access_data = 0x92u64 | (u64::from(dpl) << 5);
            let desc_code = 0x00cf_0000_0000_ffffu64 | (access_code << 40);
            let desc_data = 0x00cf_0000_0000_ffffu64 | (access_data << 40);
            ram.store(0x1008, MemWidth::Dword, desc_code as u32).unwrap();
            ram.store(0x100c, MemWidth::Dword, (desc_code >> 32) as u32).unwrap();
            ram.store(0x1010, MemWidth::Dword, desc_data as u32).unwrap();
            ram.store(0x1014, MemWidth::Dword, (desc_data >> 32) as u32).unwrap();
            ram.store(0x8000, MemWidth::Dword, 0x0140).unwrap();
            ram.store(0x8004, MemWidth::Dword, u32::from(sel)).unwrap();
            ram.store(0x8008, MemWidth::Dword, 0x0004_0202).unwrap();
            step_one(&mut image, &mut ram, &mut cpu).unwrap();
            assert_eq!(cpu.eip, 0x140);
            assert_eq!(cpu.eflags & 0x40000 != 0, expect_set, "PM CPL {dpl} IRETD {model:?}");
        }
    }

    // V86 mode IRETD with IOPL 3 (66 CF)
    for (model, expect_set) in [(CpuModel::I386, false), (CpuModel::I486Dx, true)] {
        let (mut image, mut ram, mut cpu) = setup_cpu(&[0x66, 0xcf], model, false, false);
        image.real_mode = false;
        image.msw = 0x11;
        image.segments = [0u16; 6].map(SegmentCache::v86);
        cpu.eflags = 0x0002_3202; // VM=1, IOPL=3
        ram.store(0x8000, MemWidth::Dword, 0x0130).unwrap();
        ram.store(0x8004, MemWidth::Dword, 0x0000).unwrap();
        ram.store(0x8008, MemWidth::Dword, 0x0006_3202).unwrap();
        step_one(&mut image, &mut ram, &mut cpu).unwrap();
        assert_eq!(cpu.eip, 0x130);
        assert_eq!(cpu.eflags & 0x40000 != 0, expect_set, "V86 IRETD {model:?}");
        assert_ne!(cpu.eflags & 0x20000, 0);
    }
}

#[test]
fn tss_task_switch_handles_eflags_ac_by_cpu_model() {
    for (model, expect_ac) in [(CpuModel::I386, false), (CpuModel::I486Dx, true)] {
        let (mut image, mut ram, mut cpu) = setup_cpu(&[0x90], model, true, true);
        image.tables[0] = (0x40, 0x1000);
        // 08h: flat 32-bit code, 10h: flat 32-bit data, 18h: old TSS (busy), 20h: new TSS (avail)
        let code = 0x00cf_9a00_0000_ffffu64;
        let data = 0x00cf_9200_0000_ffffu64;
        let old_tss = 0x0000_8b00_2000_0067u64;
        let new_tss = 0x0000_8900_2100_0067u64;
        for (idx, desc) in [(1, code), (2, data), (3, old_tss), (4, new_tss)] {
            ram.store(0x1000 + idx * 8, MemWidth::Dword, desc as u32).unwrap();
            ram.store(0x1000 + idx * 8 + 4, MemWidth::Dword, (desc >> 32) as u32).unwrap();
        }
        image.ldt_tr[1] = crate::address::SystemSegment { selector: 0x18, base: 0x2000, limit: 0x67, kind: 0x0b };
        // Populate new TSS at 0x2100: EIP=0x150, EFLAGS=0x40202, ESP=0x9000, CS=0x08, DS/ES/SS/FS/GS=0x10
        ram.store(0x2120, MemWidth::Dword, 0x150).unwrap();
        ram.store(0x2124, MemWidth::Dword, 0x0004_0202).unwrap();
        ram.store(0x2138, MemWidth::Dword, 0x9000).unwrap();
        for (off, sel) in [(0x48, 0x10), (0x4c, 0x08), (0x50, 0x10), (0x54, 0x10), (0x58, 0x10), (0x5c, 0x10)] {
            ram.store(0x2100 + off, MemWidth::Word, sel).unwrap();
        }
        let state = image.segment_state();
        let transfer = switch(&state, &ram, &cpu.regs, cpu.eflags, 0x105, 0x20, Entry::Jump, None, 0, 0x100).unwrap();
        let mut next = state;
        transfer.commit(&mut cpu.regs, &mut cpu.eflags, &mut cpu.undefined_flags, &mut cpu.eip, &mut next, &mut ram).unwrap();
        assert_eq!(cpu.eip, 0x150);
        assert_eq!(cpu.eflags & 0x40000 != 0, expect_ac, "TSS {model:?}");
    }

    // Also verify deliver() works cleanly with an AC-setting IRETD frame.
    let (mut image, mut ram, mut cpu) = setup_cpu(&[0x90], CpuModel::I486Dx, true, true);
    image.tables = [(0x20, 0x1000), (0x200, 0x3000)];
    let code = 0x00cf_9a00_0000_ffffu64;
    let data = 0x00cf_9200_0000_ffffu64;
    ram.store(0x1008, MemWidth::Dword, code as u32).unwrap();
    ram.store(0x100c, MemWidth::Dword, (code >> 32) as u32).unwrap();
    ram.store(0x1010, MemWidth::Dword, data as u32).unwrap();
    ram.store(0x1014, MemWidth::Dword, (data >> 32) as u32).unwrap();
    image.selectors = [0x10, 0x08, 0x10, 0x10, 0x10, 0x10];
    let gate = 0x0000_8e00_0008_0200u64; // INT386 to 0x08:0x0200
    ram.store(0x3000 + 3 * 8, MemWidth::Dword, gate as u32).unwrap();
    ram.store(0x3000 + 3 * 8 + 4, MemWidth::Dword, (gate >> 32) as u32).unwrap();
    cpu.eflags = 0x0004_0202;
    deliver(&mut cpu, &mut image, &mut ram, Event::software(3, 0x101)).unwrap();
    let pushed_flags = ram.load(cpu.regs[4] + 8, MemWidth::Dword).unwrap();
    assert_eq!(pushed_flags & 0x40000, 0x40000);
}

#[test]
fn decode_cache_and_code_context_distinguish_386_from_486dx() {
    let bytes = [0x66, 0x9c]; // PUSHFD
    let (image, ram, _cpu) = setup_cpu(&bytes, CpuModel::I386, false, false);
    let mut cache = crate::cache::DecodeCache::default();
    let d386 = cache.fetch(&image, &ram, 0x100).unwrap();
    let guest::Operation::Pil { program: p386, .. } = &d386.operation else { panic!() };
    let mut other = image.clone();
    other.cpu_model = CpuModel::I486Dx;
    assert!(!image.same_code_context(&other));
    let d486 = cache.fetch(&other, &ram, 0x100).unwrap();
    let guest::Operation::Pil { program: p486, .. } = &d486.operation else { panic!() };
    assert_eq!(cache.stats().misses, 2);
    let (_, c386) = crate::precompiled::shape(p386);
    let (_, c486) = crate::precompiled::shape(p486);
    assert_ne!(c386, c486);
}

#[test]
fn dos_session_and_config_wire_486dx_and_integrated_fpu() {
    // 1. LoadedOps::select_cpu(I486Dx) enables FPU, BDA bit 1, MSW ET|MP, and rejects select_fpu(false)
    let mut loaded = dos::load(&[0xcd, 0x20], "CPU486.COM").unwrap();
    assert_eq!(loaded.image.cpu_model, CpuModel::I386);
    assert!(!loaded.fpu.available());
    loaded.select_cpu(CpuModel::I486Dx).unwrap();
    assert_eq!(loaded.image.cpu_model, CpuModel::I486Dx);
    assert!(loaded.fpu.available());
    let eq = loaded.ram.load(0x410, MemWidth::Word).unwrap();
    assert_ne!(eq & 2, 0);
    assert_eq!(loaded.image.msw & 0x16, 0x12);
    assert!(loaded.select_fpu(false).is_err());

    // 2. Session command `cpu 486dx` before start, snapshot, and rejection after run
    let loaded = dos::load(&[0x90, 0xb8, 0x00, 0x4c, 0xcd, 0x21], "SESS486.COM").unwrap();
    let mut session = dos::session::Session::new(loaded);
    let (reply, _) = session.command(1, "cpu 486dx").unwrap();
    assert!(reply.contains("\"ok\":true"), "{reply}");
    let (snap, _) = session.command(2, "snapshot").unwrap();
    assert!(snap.contains("\"cpu_model\":\"486dx\""), "{snap}");
    assert!(snap.contains("\"fpu_present\":true"), "{snap}");
    let (run_reply, _) = session.command(3, "run 10").unwrap();
    assert!(run_reply.contains("\"ok\":true"), "{run_reply}");
    let (late_reply, _) = session.command(4, "cpu 386").unwrap();
    assert!(late_reply.contains("\"ok\":false"), "{late_reply}");

    // 3. Config parser accepts `cpu=486dx`, rejects invalid values, rejects `cpu=486dx` + `fpu=none`
    let root = std::env::temp_dir().join(format!("pil-cpu-config-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let ok_path = root.join("ok.conf");
    std::fs::write(&ok_path, "cpu=486dx\n").unwrap();
    let settings = crate::dos_config::console_settings(Some(ok_path.to_str().unwrap())).unwrap();
    assert_eq!(settings.cpu, Some(CpuModel::I486Dx));

    let bad_path = root.join("bad.conf");
    std::fs::write(&bad_path, "cpu=pentium\n").unwrap();
    assert!(crate::dos_config::console_settings(Some(bad_path.to_str().unwrap())).is_err());

    let conflict_path = root.join("conflict.conf");
    std::fs::write(&conflict_path, "cpu=486dx\nfpu=none\n").unwrap();
    assert!(crate::dos_config::console_settings(Some(conflict_path.to_str().unwrap())).is_err());
    std::fs::remove_dir_all(&root).unwrap();
}

const ARITH_6_FLAGS: u32 = 0x08d5; // CF | PF | AF | ZF | SF | OF

fn make_pm_paged_ram(cpu_model: CpuModel, bytes: &[u8], dpl: u8, v86: bool) -> (guest::Image, Ram, Cpu) {
    let (mut image, mut ram, mut cpu) = setup_cpu(bytes, cpu_model, !v86, !v86);
    image.real_mode = false;
    image.msw = 0x11;
    image.controls[0] = 0x8000_0011;
    image.controls[1] = 0x10000;
    image.paging = true;
    if v86 {
        image.segments = [SegmentCache::v86(0); 6];
        cpu.eflags = 0x0002_0202;
    } else {
        let code_access = SegmentAccess::Code { readable: true, conforming: false, big: true, dpl };
        let data_access = SegmentAccess::Data { writable: true, expand_down: false, big: true, dpl };
        image.segments = [SegmentCache { base: 0, limit: 0xffff_ffff, access: data_access }; 6];
        image.segments[1] = SegmentCache { base: 0, limit: 0xffff_ffff, access: code_access };
    }
    // PDE[0] -> PT at 0x11000 (present, writable, user).
    ram.store(0x10000, MemWidth::Dword, 0x11007).unwrap();
    // PTE[0] -> 0x00000 (code/stack), PTE[2] -> 0x02000, PTE[3] -> 0x03000; PTE[9] left 0.
    ram.store(0x11000, MemWidth::Dword, 0x00007).unwrap();
    ram.store(0x11000 + 2 * 4, MemWidth::Dword, 0x02007).unwrap();
    ram.store(0x11000 + 3 * 4, MemWidth::Dword, 0x03007).unwrap();
    ram.set_paging(Some(memory::Paging { directory: 0x10000, user: dpl == 3 || v86 }));
    (image, ram, cpu)
}

#[test]
fn i386_rejects_486_instructions() {
    let cases: &[(&str, &[u8])] = &[
        ("BSWAP EAX", &[0x0f, 0xc8]),
        ("BSWAP EDI", &[0x0f, 0xcf]),
        ("XADD AL, CL", &[0x0f, 0xc0, 0xc8]),
        ("XADD [0x1000], ECX", &[0x0f, 0xc1, 0x0e, 0x00, 0x10]),
        ("CMPXCHG CL, AL", &[0x0f, 0xb0, 0xc1]),
        ("CMPXCHG [0x1000], EDX", &[0x0f, 0xb1, 0x16, 0x00, 0x10]),
        ("INVLPG [0x1000]", &[0x0f, 0x01, 0x3e, 0x00, 0x10]),
        ("INVD", &[0x0f, 0x08]),
        ("WBINVD", &[0x0f, 0x09]),
        ("LOCK XADD [0x1000], ECX", &[0xf0, 0x0f, 0xc1, 0x0e, 0x00, 0x10]),
        ("LOCK CMPXCHG [0x1000], EDX", &[0xf0, 0x0f, 0xb1, 0x16, 0x00, 0x10]),
    ];
    for (name, bytes) in cases {
        let (image, ram, cpu) = setup_cpu(bytes, CpuModel::I386, true, true);
        let err = image.decode(&ram, cpu.eip).unwrap_err();
        assert!(matches!(err, guest::GuestError::Decode(_)), "386 should reject {name}, got {err:?}");
    }
}

#[test]
fn bswap_32_and_16_semantics() {
    let preserved_flags = 0x0004_0202 | ARITH_6_FLAGS;
    for reg in 0..8 {
        let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xc8 + reg as u8], CpuModel::I486Dx, true, true);
        cpu.regs = [0x1111_1111; 8];
        cpu.regs[reg] = 0x1234_5678;
        cpu.eflags = preserved_flags;
        cpu.undefined_flags = 0x15;
        step_one(&mut image, &mut ram, &mut cpu).unwrap();
        assert_eq!(cpu.regs[reg], 0x7856_3412, "reg {reg}");
        for other in 0..8 {
            if other != reg {
                assert_eq!(cpu.regs[other], 0x1111_1111);
            }
        }
        assert_eq!(cpu.eflags, preserved_flags);
        assert_eq!(cpu.undefined_flags, 0x15);
    }

    // 16-bit BSWAP zeroes low 16 bits while preserving high 16 bits and flags.
    for (big_code, bytes) in [(true, &[0x66, 0x0f, 0xc8][..]), (false, &[0x0f, 0xc8][..])] {
        let (mut image, mut ram, mut cpu) = setup_cpu(bytes, CpuModel::I486Dx, big_code, big_code);
        cpu.regs[0] = 0xabcd_1234;
        cpu.eflags = preserved_flags;
        step_one(&mut image, &mut ram, &mut cpu).unwrap();
        assert_eq!(cpu.regs[0], 0xabcd_0000);
        assert_eq!(cpu.eflags, preserved_flags);
    }

    // LOCK BSWAP is invalid even on 486DX.
    let (image, ram, cpu) = setup_cpu(&[0xf0, 0x0f, 0xc8], CpuModel::I486Dx, true, true);
    assert!(image.decode(&ram, cpu.eip).is_err());
}

#[test]
fn xadd_register_memory_flags_and_fault_rollback() {
    // 32-bit XADD EAX, EBX (0F C1 D8): 0x7fff_ffff + 1 -> 0x8000_0000, old EAX -> EBX.
    let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xc1, 0xd8], CpuModel::I486Dx, true, true);
    cpu.regs[0] = 0x7fff_ffff;
    cpu.regs[3] = 1;
    step_one(&mut image, &mut ram, &mut cpu).unwrap();
    assert_eq!(cpu.regs[0], 0x8000_0000);
    assert_eq!(cpu.regs[3], 0x7fff_ffff);
    assert_eq!(cpu.eflags & ARITH_6_FLAGS, 0x0894); // OF | SF | AF | PF

    // Self-register 32-bit XADD EAX, EAX (0F C1 C0): doubles EAX without InvalidBinding.
    let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xc1, 0xc0], CpuModel::I486Dx, true, true);
    cpu.regs[0] = 0x1122_3344;
    step_one(&mut image, &mut ram, &mut cpu).unwrap();
    assert_eq!(cpu.regs[0], 0x2244_6688);
    assert_eq!(cpu.eflags & ARITH_6_FLAGS, 0x0004); // PF (0x88 has even parity)

    // 16-bit XADD AX, CX (0F C1 C8): preserves upper 16 bits of both registers.
    let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xc1, 0xc8], CpuModel::I486Dx, false, false);
    cpu.regs[0] = 0xaa55_8000;
    cpu.regs[1] = 0xbb66_8000;
    step_one(&mut image, &mut ram, &mut cpu).unwrap();
    assert_eq!(cpu.regs[0], 0xaa55_0000);
    assert_eq!(cpu.regs[1], 0xbb66_8000);
    assert_eq!(cpu.eflags & ARITH_6_FLAGS, 0x0845); // OF | ZF | PF | CF

    // 8-bit XADD AL, AL (0F C0 C0), XADD AH, AL (0F C0 C4), and XADD AL, AH (0F C0 E0).
    let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xc0, 0xc0], CpuModel::I486Dx, true, true);
    cpu.regs[0] = 0x1234_5680;
    step_one(&mut image, &mut ram, &mut cpu).unwrap();
    assert_eq!(cpu.regs[0], 0x1234_5600);
    assert_eq!(cpu.eflags & ARITH_6_FLAGS, 0x0845);

    let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xc0, 0xc4], CpuModel::I486Dx, true, true);
    cpu.regs[0] = 0x1122_1234; // AH=0x12, AL=0x34 -> AH=0x46, AL=0x12
    step_one(&mut image, &mut ram, &mut cpu).unwrap();
    assert_eq!(cpu.regs[0], 0x1122_4612);

    let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xc0, 0xe0], CpuModel::I486Dx, true, true);
    cpu.regs[0] = 0x1122_1234; // AL=0x34, AH=0x12 -> AL=0x46, AH=0x34
    step_one(&mut image, &mut ram, &mut cpu).unwrap();
    assert_eq!(cpu.regs[0], 0x1122_3446);

    // Memory XADD [0x1000], ECX with and without LOCK in 32-bit mode, plus readonly fault rollback.
    let flat_ds = SegmentCache {
        base: 0,
        limit: 0xffff_ffff,
        access: SegmentAccess::Data { writable: true, expand_down: false, big: true, dpl: 0 },
    };
    for locked in [false, true] {
        let bytes: &[u8] = if locked {
            &[0xf0, 0x0f, 0xc1, 0x0d, 0x00, 0x10, 0x00, 0x00]
        } else {
            &[0x0f, 0xc1, 0x0d, 0x00, 0x10, 0x00, 0x00]
        };
        let (mut image, mut ram, mut cpu) = setup_cpu(bytes, CpuModel::I486Dx, true, true);
        image.segments[3] = flat_ds;
        ram.store(0x1000, MemWidth::Dword, 0x0000_000f).unwrap();
        cpu.regs[1] = 0x0000_0001;
        step_one(&mut image, &mut ram, &mut cpu).unwrap();
        assert_eq!(ram.load(0x1000, MemWidth::Dword).unwrap(), 0x0000_0010);
        assert_eq!(cpu.regs[1], 0x0000_000f);
        assert_eq!(cpu.eflags & ARITH_6_FLAGS, 0x0010); // AF
    }

    let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xc1, 0x0d, 0x00, 0x10, 0x00, 0x00], CpuModel::I486Dx, true, true);
    image.segments[3] = flat_ds;
    ram.store(0x1000, MemWidth::Dword, 0x1234_5678).unwrap();
    ram.protect(0x1000, 4).unwrap();
    cpu.regs[1] = 0x9abc_def0;
    let saved = (cpu.clone(), image.clone());
    assert!(matches!(step_one(&mut image, &mut ram, &mut cpu), Err(ExecError::MemoryAccess { write: true, .. })));
    assert_eq!((cpu, image), saved);
    assert_eq!(ram.load(0x1000, MemWidth::Dword).unwrap(), 0x1234_5678);

    // LOCK XADD reg, reg is invalid.
    let (image, ram, cpu) = setup_cpu(&[0xf0, 0x0f, 0xc1, 0xd8], CpuModel::I486Dx, true, true);
    assert!(image.decode(&ram, cpu.eip).is_err());
}

#[test]
fn cmpxchg_register_memory_flags_and_fault_rollback() {
    let flat_ds = SegmentCache {
        base: 0,
        limit: 0xffff_ffff,
        access: SegmentAccess::Data { writable: true, expand_down: false, big: true, dpl: 0 },
    };
    // 32-bit register match: CMPXCHG EBX, ECX (0F B1 CB). EAX == EBX -> EBX = ECX, ZF = 1.
    let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xb1, 0xcb], CpuModel::I486Dx, true, true);
    cpu.regs[0] = 0xdead_beef;
    cpu.regs[3] = 0xdead_beef;
    cpu.regs[1] = 0xcafe_babe;
    step_one(&mut image, &mut ram, &mut cpu).unwrap();
    assert_eq!(cpu.regs[0], 0xdead_beef);
    assert_eq!(cpu.regs[3], 0xcafe_babe);
    assert_eq!(cpu.regs[1], 0xcafe_babe);
    assert_eq!(cpu.eflags & ARITH_6_FLAGS, 0x0044); // ZF | PF

    // 32-bit register mismatch: CMPXCHG EBX, ECX (0F B1 CB). EAX != EBX -> EAX = EBX, EBX unchanged.
    let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xb1, 0xcb], CpuModel::I486Dx, true, true);
    cpu.regs[0] = 0x8000_0000;
    cpu.regs[3] = 1;
    cpu.regs[1] = 0xcafe_babe;
    step_one(&mut image, &mut ram, &mut cpu).unwrap();
    assert_eq!(cpu.regs[0], 1);
    assert_eq!(cpu.regs[3], 1);
    assert_eq!(cpu.eflags & ARITH_6_FLAGS, 0x0814); // OF | AF | PF (0x80000000 - 1 = 0x7fffffff)

    // 16-bit match & mismatch preserve high bits of EAX and destination.
    let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xb1, 0xcb], CpuModel::I486Dx, false, false);
    cpu.regs[0] = 0x1234_0010;
    cpu.regs[3] = 0x5678_0020;
    cpu.regs[1] = 0x9abc_00ff;
    step_one(&mut image, &mut ram, &mut cpu).unwrap();
    assert_eq!(cpu.regs[0], 0x1234_0020);
    assert_eq!(cpu.regs[3], 0x5678_0020);
    assert_eq!(cpu.eflags & ARITH_6_FLAGS, 0x0085); // SF | PF | CF (0x10 - 0x20 = 0xfff0)

    // 8-bit high-byte combinations: CMPXCHG BH, AH (0F B0 E7), CMPXCHG AH, AL (0F B0 C4),
    // CMPXCHG AL, AH (0F B0 E0), CMPXCHG AL, AL (0F B0 C0).
    let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xb0, 0xe7], CpuModel::I486Dx, true, true);
    cpu.regs[0] = 0x1122_7755; // AH=0x77, AL=0x55
    cpu.regs[3] = 0x3344_5566; // BH=0x55, BL=0x66 (BH == AL)
    step_one(&mut image, &mut ram, &mut cpu).unwrap();
    assert_eq!(cpu.regs[3], 0x3344_7766); // BH <- AH (0x77)
    assert_eq!(cpu.regs[0], 0x1122_7755);

    let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xb0, 0xe7], CpuModel::I486Dx, true, true);
    cpu.regs[0] = 0x1122_7710; // AL=0x10
    cpu.regs[3] = 0x3344_9966; // BH=0x99 (BH != AL)
    step_one(&mut image, &mut ram, &mut cpu).unwrap();
    assert_eq!(cpu.regs[0], 0x1122_7799); // AL <- BH (0x99)
    assert_eq!(cpu.regs[3], 0x3344_9966);

    let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xb0, 0xc4], CpuModel::I486Dx, true, true);
    cpu.regs[0] = 0x1122_3344; // AL=0x44, AH=0x33 (AH != AL -> AL <- AH)
    step_one(&mut image, &mut ram, &mut cpu).unwrap();
    assert_eq!(cpu.regs[0], 0x1122_3333);

    let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xb0, 0xe0], CpuModel::I486Dx, true, true);
    cpu.regs[0] = 0x1122_7777; // AL=0x77, AH=0x77 (match)
    step_one(&mut image, &mut ram, &mut cpu).unwrap();
    assert_eq!(cpu.regs[0], 0x1122_7777);
    assert_eq!(cpu.eflags & 0x40, 0x40);

    let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xb0, 0xc0], CpuModel::I486Dx, true, true);
    cpu.regs[0] = 0x1122_3344;
    step_one(&mut image, &mut ram, &mut cpu).unwrap();
    assert_eq!(cpu.regs[0], 0x1122_3344);
    assert_eq!(cpu.eflags & 0x40, 0x40);

    // Memory CMPXCHG [0x1000], ECX match & mismatch with LOCK prefix, plus 8-bit AH source.
    let (mut image, mut ram, mut cpu) = setup_cpu(&[0xf0, 0x0f, 0xb1, 0x0d, 0x00, 0x10, 0x00, 0x00], CpuModel::I486Dx, true, true);
    image.segments[3] = flat_ds;
    ram.store(0x1000, MemWidth::Dword, 0xaaaa_bbbb).unwrap();
    cpu.regs[0] = 0xaaaa_bbbb;
    cpu.regs[1] = 0x1234_5678;
    step_one(&mut image, &mut ram, &mut cpu).unwrap();
    assert_eq!(ram.load(0x1000, MemWidth::Dword).unwrap(), 0x1234_5678);
    assert_eq!(cpu.regs[0], 0xaaaa_bbbb);
    assert_eq!(cpu.eflags & 0x40, 0x40);

    let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xb0, 0x25, 0x00, 0x10, 0x00, 0x00], CpuModel::I486Dx, true, true);
    image.segments[3] = flat_ds;
    ram.store(0x1000, MemWidth::Byte, 0x88).unwrap();
    cpu.regs[0] = 0x0000_aa88; // AL == 0x88, AH == 0xaa
    step_one(&mut image, &mut ram, &mut cpu).unwrap();
    assert_eq!(ram.load(0x1000, MemWidth::Byte).unwrap(), 0xaa);

    // Read-only memory faults on BOTH match and mismatch with full rollback (no EAX or flag write on mismatch!).
    for eax_val in [0x2222_3333u32, 0x9999_9999u32] {
        let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xb1, 0x0d, 0x00, 0x10, 0x00, 0x00], CpuModel::I486Dx, true, true);
        image.segments[3] = flat_ds;
        ram.store(0x1000, MemWidth::Dword, 0x2222_3333).unwrap();
        ram.protect(0x1000, 4).unwrap();
        cpu.regs[0] = eax_val;
        cpu.regs[1] = 0x4444_5555;
        cpu.eflags = 0x0202;
        let saved = (cpu.clone(), image.clone());
        assert!(matches!(step_one(&mut image, &mut ram, &mut cpu), Err(ExecError::MemoryAccess { write: true, .. })));
        assert_eq!((cpu, image), saved);
        assert_eq!(ram.load(0x1000, MemWidth::Dword).unwrap(), 0x2222_3333);
    }

    // LOCK CMPXCHG reg, reg is invalid.
    let (image, ram, cpu) = setup_cpu(&[0xf0, 0x0f, 0xb1, 0xcb], CpuModel::I486Dx, true, true);
    assert!(image.decode(&ram, cpu.eip).is_err());
}

#[test]
fn invlpg_evicts_single_page_and_checks_privilege() {
    // 1. Paged PM CPL 0: INVLPG [0x2000] evicts 0x2000 from TLB while keeping 0x3000 cached and flags preserved.
    let (mut image, mut ram, mut cpu) = make_pm_paged_ram(CpuModel::I486Dx, &[0x0f, 0x01, 0x3d, 0x00, 0x20, 0x00, 0x00], 0, false);
    ram.store(0x2000, MemWidth::Byte, 0xaa).unwrap();
    ram.store(0x3000, MemWidth::Byte, 0xbb).unwrap();
    assert_eq!(ram.load(0x3000, MemWidth::Byte).unwrap(), 0xbb);
    assert_eq!(ram.load(0x2000, MemWidth::Byte).unwrap(), 0xaa);
    assert!(ram.tlb_contains(0x2000));
    assert!(ram.tlb_contains(0x3000));
    let preserved = 0x0004_0202 | ARITH_6_FLAGS;
    cpu.eflags = preserved;
    step_one(&mut image, &mut ram, &mut cpu).unwrap();
    assert!(!ram.tlb_contains(0x2000));
    assert!(ram.tlb_contains(0x3000));
    assert_eq!(cpu.eflags, preserved);

    // 2. Real mode INVLPG [0x2000] also succeeds cleanly (CPL == 0) preserving flags.
    let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0x01, 0x3e, 0x00, 0x20], CpuModel::I486Dx, false, false);
    cpu.eflags = 0x0246;
    step_one(&mut image, &mut ram, &mut cpu).unwrap();
    assert_eq!(cpu.eflags, 0x0246);

    // 3. Unmapped linear address (PTE[9] = 0): INVLPG never raises #PF.
    let (mut image, mut ram, mut cpu) = make_pm_paged_ram(CpuModel::I486Dx, &[0x0f, 0x01, 0x3d, 0x00, 0x90, 0x00, 0x00], 0, false);
    step_one(&mut image, &mut ram, &mut cpu).unwrap();

    // 4. CPL 3 and V86 raise #GP(0).
    let (mut image, mut ram, mut cpu) = make_pm_paged_ram(CpuModel::I486Dx, &[0x0f, 0x01, 0x3d, 0x00, 0x20, 0x00, 0x00], 3, false);
    assert_eq!(step_one(&mut image, &mut ram, &mut cpu), Err(ExecError::GeneralProtection { eip: 0x100, error_code: 0 }));

    let (mut image, mut ram, mut cpu) = make_pm_paged_ram(CpuModel::I486Dx, &[0x0f, 0x01, 0x3e, 0x00, 0x20], CpuModel::I486Dx as u8 * 0 + 3, true);
    assert_eq!(step_one(&mut image, &mut ram, &mut cpu), Err(ExecError::GeneralProtection { eip: 0x100, error_code: 0 }));

    // 5. Register operand (0F 01 F8) and LOCK INVLPG are rejected at decode.
    let (image, ram, cpu) = setup_cpu(&[0x0f, 0x01, 0xf8], CpuModel::I486Dx, true, true);
    assert!(image.decode(&ram, cpu.eip).is_err());
    let (image, ram, cpu) = setup_cpu(&[0xf0, 0x0f, 0x01, 0x3d, 0x00, 0x20, 0x00, 0x00], CpuModel::I486Dx, true, true);
    assert!(image.decode(&ram, cpu.eip).is_err());
}

#[test]
fn invd_and_wbinvd_privilege_and_noop() {
    for second in [0x08u8, 0x09u8] {
        // Real mode & PM CPL 0 succeed as flag/register-preserving no-ops.
        let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, second], CpuModel::I486Dx, false, false);
        cpu.regs = [0x1122_3344; 8];
        cpu.eflags = 0x0004_02d7;
        step_one(&mut image, &mut ram, &mut cpu).unwrap();
        assert_eq!(cpu.regs, [0x1122_3344; 8]);
        assert_eq!(cpu.eflags, 0x0004_02d7);

        let (mut image, mut ram, mut cpu) = make_pm_paged_ram(CpuModel::I486Dx, &[0x0f, second], 0, false);
        step_one(&mut image, &mut ram, &mut cpu).unwrap();

        // CPL 3 and V86 raise #GP(0).
        let (mut image, mut ram, mut cpu) = make_pm_paged_ram(CpuModel::I486Dx, &[0x0f, second], 3, false);
        assert_eq!(step_one(&mut image, &mut ram, &mut cpu), Err(ExecError::GeneralProtection { eip: 0x100, error_code: 0 }));

        let (mut image, mut ram, mut cpu) = make_pm_paged_ram(CpuModel::I486Dx, &[0x0f, second], 3, true);
        assert_eq!(step_one(&mut image, &mut ram, &mut cpu), Err(ExecError::GeneralProtection { eip: 0x100, error_code: 0 }));

        // LOCK is rejected at decode.
        let (image, ram, cpu) = setup_cpu(&[0xf0, 0x0f, second], CpuModel::I486Dx, true, true);
        assert!(image.decode(&ram, cpu.eip).is_err());
    }
}

#[test]
fn emitted_486_instructions_match_interpreter() {
    use std::{fs, process::Command, time::{SystemTime, UNIX_EPOCH}};
    struct Case {
        name: &'static str,
        bytes: &'static [u8],
        regs: [u32; 8],
        mem: Option<(u32, u32)>,
        paged: bool,
    }
    let cases = [
        Case { name: "bswap", bytes: &[0x0f, 0xca], regs: [0, 0, 0x1234_5678, 0, 0x8000, 0, 0, 0], mem: None, paged: false },
        Case { name: "xadd_reg", bytes: &[0x0f, 0xc1, 0xd8], regs: [0x7fff_ffff, 0, 0, 1, 0x8000, 0, 0, 0], mem: None, paged: false },
        Case { name: "xadd_mem", bytes: &[0x0f, 0xc1, 0x0d, 0x00, 0x10, 0x00, 0x00], regs: [0, 0x22, 0, 0, 0x8000, 0, 0, 0], mem: Some((0x1000, 0x33)), paged: false },
        Case { name: "cmpxchg_match", bytes: &[0x0f, 0xb1, 0xcb], regs: [0x55, 0x77, 0, 0x55, 0x8000, 0, 0, 0], mem: None, paged: false },
        Case { name: "cmpxchg_miss", bytes: &[0x0f, 0xb1, 0xcb], regs: [0x11, 0x77, 0, 0x55, 0x8000, 0, 0, 0], mem: None, paged: false },
        Case { name: "cmpxchg_mem", bytes: &[0x0f, 0xb1, 0x0d, 0x00, 0x10, 0x00, 0x00], regs: [0x99, 0x44, 0, 0, 0x8000, 0, 0, 0], mem: Some((0x1000, 0x99)), paged: false },
        Case { name: "invlpg", bytes: &[0x0f, 0x01, 0x3d, 0x00, 0x20, 0x00, 0x00], regs: [0, 0, 0, 0, 0x8000, 0, 0, 0], mem: Some((0x2000, 0xaa)), paged: true },
        Case { name: "invd", bytes: &[0x0f, 0x08], regs: [0x12, 0x34, 0, 0, 0x8000, 0, 0, 0], mem: None, paged: false },
    ];

    let mut source = emit_rust_support();
    let mut runner = String::from("fn main() {\nuse address::{*, EffectiveAddress::*, SegmentAccess::*, SegmentRegister::*};\n");
    for case in &cases {
        let (mut image, mut ram, mut cpu) = if case.paged {
            make_pm_paged_ram(CpuModel::I486Dx, case.bytes, 0, false)
        } else {
            let (mut image, ram, cpu) = setup_cpu(case.bytes, CpuModel::I486Dx, true, true);
            image.segments[3] = SegmentCache {
                base: 0,
                limit: 0xffff_ffff,
                access: SegmentAccess::Data { writable: true, expand_down: false, big: true, dpl: 0 },
            };
            (image, ram, cpu)
        };
        cpu.regs = case.regs;
        if let Some((addr, val)) = case.mem {
            ram.store(addr, MemWidth::Dword, val).unwrap();
        }
        if case.paged {
            ram.store(0x3000, MemWidth::Dword, 0xbb).unwrap();
            let _ = ram.load(0x3000, MemWidth::Dword).unwrap();
            let _ = ram.load(0x2000, MemWidth::Dword).unwrap();
        }
        let decoded = image.decode(&ram, cpu.eip).unwrap();
        let guest::Operation::Pil { program, dst, src, flow, address } = decoded.operation else {
            panic!("expected PIL op for {}", case.name);
        };
        source.push_str(&format!("\nmod m_{} {{ use super::*;\n{}\n}}\n", case.name, emit_rust_functions(&program)));

        let init_regs = cpu.regs;
        let init_flags = cpu.eflags;
        let init_undef = cpu.undefined_flags;
        let init_eip = cpu.eip;
        step_one(&mut image, &mut ram, &mut cpu).unwrap();
        let expected_mem = case.mem.map(|(a, _)| (a, ram.load(a, MemWidth::Dword).unwrap()));

        let addr_expr = match address {
            Some(a) => format!("Some({a:?})"),
            None => "None".to_string(),
        };
        let setup_paging = if case.paged {
            "ram.store(0x10000, MemWidth::Dword, 0x11007).unwrap();\n\
             ram.store(0x11000, MemWidth::Dword, 0x00007).unwrap();\n\
             ram.store(0x11008, MemWidth::Dword, 0x02007).unwrap();\n\
             ram.store(0x1100c, MemWidth::Dword, 0x03007).unwrap();\n\
             m.controls[1] = 0x10000; m.paging = true;\n\
             ram.set_paging(m.ram_paging());\n\
             let _ = ram.load(0x3000, MemWidth::Dword).unwrap();\n\
             let _ = ram.load(0x2000, MemWidth::Dword).unwrap();\n\
             assert!(ram.tlb_contains(0x2000)); assert!(ram.tlb_contains(0x3000));\n"
        } else {
            ""
        };
        let mem_init = match case.mem {
            Some((a, v)) => format!("ram.store({a}, MemWidth::Dword, {v}).unwrap();\n"),
            None => String::new(),
        };
        let mem_check = match expected_mem {
            Some((a, v)) => format!("assert_eq!(ram.load({a}, MemWidth::Dword).unwrap(), {v}, \"{} mem\");\n", case.name),
            None => String::new(),
        };
        let tlb_check = if case.paged {
            "assert!(!ram.tlb_contains(0x2000), \"emitted INVLPG failed to evict 0x2000\");\n\
             assert!(ram.tlb_contains(0x3000), \"emitted INVLPG evicted unrelated 0x3000\");\n"
        } else {
            ""
        };
        runner.push_str(&format!(
            "{{\n\
             let mut ram = Ram::new(0x20000);\n\
             let code_access = SegmentAccess::Code {{ readable: true, conforming: false, big: true, dpl: 0 }};\n\
             let data_access = SegmentAccess::Data {{ writable: true, expand_down: false, big: true, dpl: 0 }};\n\
             let mut m = SegmentState {{ segments: [SegmentCache {{ base: 0, limit: 0xffff_ffff, access: data_access }}; 6], selectors: [8; 6], msw: 0x11, real_mode: false, ss_shadow: 0, sti_shadow: 0, tables: [(0, 0); 2], controls: [0x11, 0], flag_checkpoint: None, ldt_tr: [SystemSegment::default(); 2], paging: false, debug: [0; 6], cpu_model: CpuModel::I486Dx }};\n\
             m.segments[1] = SegmentCache {{ base: 0, limit: 0xffff_ffff, access: code_access }};\n\
             {mem_init}{setup_paging}\
             let mut regs = {init_regs:?}; let mut flags = {init_flags}u32; let mut undef = {init_undef}u32; let mut ip = {init_eip}u32;\n\
             m_{name}::execute_with_machine(&mut regs, &mut flags, &mut undef, &mut ip, ({dr},{dl}), ({sr},{sl}), {flow:?}, &mut ram, {addr_expr}, &mut m).unwrap();\n\
             assert_eq!(regs, {exp_regs:?}, \"{name} regs\");\n\
             assert_eq!(flags, {exp_flags}u32, \"{name} flags\");\n\
             assert_eq!(undef, {exp_undef}u32, \"{name} undef\");\n\
             assert_eq!(ip, {exp_eip}u32, \"{name} eip\");\n\
             {tlb_check}{mem_check}}}\n",
            name = case.name,
            dr = dst.register, dl = dst.low,
            sr = src.register, sl = src.low,
            exp_regs = cpu.regs,
            exp_flags = cpu.eflags,
            exp_undef = cpu.undefined_flags,
            exp_eip = cpu.eip,
        ));
    }
    runner.push_str("}\n");
    source.push_str(&runner);

    let dir = std::env::temp_dir().join(format!("pil-486-emit-{}-{}", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("check.rs");
    fs::write(&path, source).unwrap();
    let bin = dir.join(if cfg!(windows) { "check.exe" } else { "check" });
    let out = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--edition=2021", "-O"])
        .arg(&path)
        .arg("-o")
        .arg(&bin)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let run = Command::new(&bin).status().unwrap();
    assert!(run.success());
    fs::remove_dir_all(&dir).unwrap();
}

#[cfg(target_arch = "x86_64")]
#[test]
fn host_x86_64_oracle_486_instructions() {
    use core::arch::asm;

    // 1. BSWAP r32 against hardware BSWAP.
    for val in [0u32, 1, 0x1234_5678, 0x8000_0000, 0xffff_ff00, 0xa55a_f00f] {
        let mut hw = val;
        let mut hw_flags: u64;
        unsafe {
            asm!(
                "push 0x202",
                "popfq",
                "bswap {v:e}",
                "pushfq",
                "pop {fl}",
                v = inout(reg) hw,
                fl = out(reg) hw_flags,
            );
        }
        let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xc8], CpuModel::I486Dx, true, true);
        cpu.regs[0] = val;
        cpu.eflags = 0x202;
        step_one(&mut image, &mut ram, &mut cpu).unwrap();
        assert_eq!(cpu.regs[0], hw, "BSWAP 0x{val:08x}");
        assert_eq!(cpu.eflags & ARITH_6_FLAGS, (hw_flags as u32) & ARITH_6_FLAGS);
    }

    // 2. XADD r8/r16/r32 against hardware XADD.
    let vals32 = [0u32, 1, 0x7f, 0x80, 0xff, 0x7fff, 0x8000, 0xffff, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff, 0x1234_5678];
    for &a in &vals32 {
        for &b in &vals32 {
            // 32-bit XADD EAX, ECX (0F C1 C8)
            let mut hw_a = a;
            let mut hw_b = b;
            let mut hw_flags: u64;
            unsafe {
                asm!(
                    "push 0x202",
                    "popfq",
                    "xadd eax, ecx",
                    "pushfq",
                    "pop {fl}",
                    inout("eax") hw_a,
                    inout("ecx") hw_b,
                    fl = out(reg) hw_flags,
                );
            }
            let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xc1, 0xc8], CpuModel::I486Dx, true, true);
            cpu.regs[0] = a;
            cpu.regs[1] = b;
            step_one(&mut image, &mut ram, &mut cpu).unwrap();
            assert_eq!((cpu.regs[0], cpu.regs[1]), (hw_a, hw_b), "XADD32 a={a:#x} b={b:#x}");
            assert_eq!(cpu.eflags & ARITH_6_FLAGS, (hw_flags as u32) & ARITH_6_FLAGS);

            // 16-bit XADD AX, CX (0F C1 C8 in 16-bit mode)
            let mut hw_a16 = a as u16;
            let mut hw_b16 = b as u16;
            unsafe {
                asm!(
                    "push 0x202",
                    "popfq",
                    "xadd ax, cx",
                    "pushfq",
                    "pop {fl}",
                    inout("ax") hw_a16,
                    inout("cx") hw_b16,
                    fl = out(reg) hw_flags,
                );
            }
            let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xc1, 0xc8], CpuModel::I486Dx, false, false);
            cpu.regs[0] = 0xaaaa_0000 | u32::from(a as u16);
            cpu.regs[1] = 0x5555_0000 | u32::from(b as u16);
            step_one(&mut image, &mut ram, &mut cpu).unwrap();
            assert_eq!((cpu.regs[0], cpu.regs[1]), (0xaaaa_0000 | u32::from(hw_a16), 0x5555_0000 | u32::from(hw_b16)), "XADD16 a={a:#x} b={b:#x}");
            assert_eq!(cpu.eflags & ARITH_6_FLAGS, (hw_flags as u32) & ARITH_6_FLAGS);

            // 8-bit XADD AL, CL (0F C0 C8)
            let mut hw_a8 = a as u8;
            let mut hw_b8 = b as u8;
            unsafe {
                asm!(
                    "push 0x202",
                    "popfq",
                    "xadd al, cl",
                    "pushfq",
                    "pop {fl}",
                    inout("al") hw_a8,
                    inout("cl") hw_b8,
                    fl = out(reg) hw_flags,
                );
            }
            let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xc0, 0xc8], CpuModel::I486Dx, true, true);
            cpu.regs[0] = 0xdead_be00 | u32::from(a as u8);
            cpu.regs[1] = 0xcafe_ba00 | u32::from(b as u8);
            step_one(&mut image, &mut ram, &mut cpu).unwrap();
            assert_eq!((cpu.regs[0], cpu.regs[1]), (0xdead_be00 | u32::from(hw_a8), 0xcafe_ba00 | u32::from(hw_b8)), "XADD8 a={a:#x} b={b:#x}");
            assert_eq!(cpu.eflags & ARITH_6_FLAGS, (hw_flags as u32) & ARITH_6_FLAGS);
        }
    }

    // 3. CMPXCHG r8/r16/r32 against hardware CMPXCHG.
    let cmp_triples = [
        (0u32, 0u32, 0x1234_5678u32),
        (0x7f, 0x7f, 0x80),
        (0x80, 0x7f, 0x11),
        (0x7fff_ffff, 0x7fff_ffff, 0x8000_0000),
        (0x8000_0000, 1, 0xffff_ffff),
        (0xffff_ffff, 0xffff_ffff, 0),
        (0x1234_5678, 0x8765_4321, 0xaaaa_bbbb),
    ];
    for &(acc, dst, src) in &cmp_triples {
        // 32-bit CMPXCHG ECX, EDX (0F B1 D1)
        let mut hw_acc = acc;
        let mut hw_dst = dst;
        let mut hw_flags: u64;
        unsafe {
            asm!(
                "push 0x202",
                "popfq",
                "cmpxchg ecx, edx",
                "pushfq",
                "pop {fl}",
                inout("eax") hw_acc,
                inout("ecx") hw_dst,
                in("edx") src,
                fl = out(reg) hw_flags,
            );
        }
        let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xb1, 0xd1], CpuModel::I486Dx, true, true);
        cpu.regs[0] = acc;
        cpu.regs[1] = dst;
        cpu.regs[2] = src;
        step_one(&mut image, &mut ram, &mut cpu).unwrap();
        assert_eq!((cpu.regs[0], cpu.regs[1]), (hw_acc, hw_dst), "CMPXCHG32 acc={acc:#x} dst={dst:#x}");
        assert_eq!(cpu.eflags & ARITH_6_FLAGS, (hw_flags as u32) & ARITH_6_FLAGS);

        // 16-bit CMPXCHG CX, DX (0F B1 D1 in 16-bit mode)
        let mut hw_acc16 = acc as u16;
        let mut hw_dst16 = dst as u16;
        unsafe {
            asm!(
                "push 0x202",
                "popfq",
                "cmpxchg cx, dx",
                "pushfq",
                "pop {fl}",
                inout("ax") hw_acc16,
                inout("cx") hw_dst16,
                in("dx") src as u16,
                fl = out(reg) hw_flags,
            );
        }
        let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xb1, 0xd1], CpuModel::I486Dx, false, false);
        cpu.regs[0] = 0x1111_0000 | u32::from(acc as u16);
        cpu.regs[1] = 0x2222_0000 | u32::from(dst as u16);
        cpu.regs[2] = 0x3333_0000 | u32::from(src as u16);
        step_one(&mut image, &mut ram, &mut cpu).unwrap();
        assert_eq!((cpu.regs[0], cpu.regs[1]), (0x1111_0000 | u32::from(hw_acc16), 0x2222_0000 | u32::from(hw_dst16)), "CMPXCHG16 acc={acc:#x} dst={dst:#x}");
        assert_eq!(cpu.eflags & ARITH_6_FLAGS, (hw_flags as u32) & ARITH_6_FLAGS);

        // 8-bit CMPXCHG CL, DL (0F B0 D1)
        let mut hw_acc8 = acc as u8;
        let mut hw_dst8 = dst as u8;
        unsafe {
            asm!(
                "push 0x202",
                "popfq",
                "cmpxchg cl, dl",
                "pushfq",
                "pop {fl}",
                inout("al") hw_acc8,
                inout("cl") hw_dst8,
                in("dl") src as u8,
                fl = out(reg) hw_flags,
            );
        }
        let (mut image, mut ram, mut cpu) = setup_cpu(&[0x0f, 0xb0, 0xd1], CpuModel::I486Dx, true, true);
        cpu.regs[0] = 0x1111_2200 | u32::from(acc as u8);
        cpu.regs[1] = 0x3333_4400 | u32::from(dst as u8);
        cpu.regs[2] = 0x5555_6600 | u32::from(src as u8);
        step_one(&mut image, &mut ram, &mut cpu).unwrap();
        assert_eq!((cpu.regs[0], cpu.regs[1]), (0x1111_2200 | u32::from(hw_acc8), 0x3333_4400 | u32::from(hw_dst8)), "CMPXCHG8 acc={acc:#x} dst={dst:#x}");
        assert_eq!(cpu.eflags & ARITH_6_FLAGS, (hw_flags as u32) & ARITH_6_FLAGS);
    }
}
