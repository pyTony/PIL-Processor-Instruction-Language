use std::env;
use std::fs;
use std::io::{self, Write};
use std::time::Instant;

use pil_cpu::address::{SegmentAccess, SegmentCache, SegmentRegister, SegmentState};
use pil_cpu::block_compiler::engine::Blocks;
use pil_cpu::cache::DecodeCache;
use pil_cpu::fpu::Fpu;
use pil_cpu::guest::{Image, Operation};
use pil_cpu::interrupt::{self, Event};
use pil_cpu::memory::{AccessFault, MemWidth, Ram};
use pil_cpu::{self, Cpu, FlowContext, LazyFlags};

const EAX: usize = 0;
const EBX: usize = 3;
const ECX: usize = 1;
const EDX: usize = 2;
const ESP: usize = 4;

const SYS_EXIT: u32 = 1;
const SYS_WRITE: u32 = 4;

fn configure_flat_32bit_image(image: &mut Image, start: u32, len: u32) {
    let flat_data = SegmentCache {
        base: 0,
        limit: u32::MAX,
        access: SegmentAccess::Data {
            writable: true,
            expand_down: false,
            big: true,
            dpl: 0,
        },
    };
    let flat_code = SegmentCache {
        base: 0,
        limit: u32::MAX,
        access: SegmentAccess::Code {
            readable: true,
            conforming: false,
            big: true,
            dpl: 0,
        },
    };

    image.segments = [flat_data; 6];
    image.segments[SegmentRegister::Cs.index()] = flat_code;
    image.real_mode = false;
    image.msw = 1;
    image.paging = false;
    image.start = start;
    image.end = start.saturating_add(len);
}

fn read_guest_bytes(ram: &Ram, addr: u32, len: usize) -> Result<Vec<u8>, i32> {
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let va = addr.wrapping_add(i as u32);
        let b = ram.load(va, MemWidth::Byte).map_err(|_: AccessFault| -14)?;
        out.push(b as u8);
    }
    Ok(out)
}

fn handle_syscall_0x80(
    cpu: &mut Cpu,
    ram: &Ram,
    write_calls: &mut u64,
    exit_code: &mut Option<i32>,
) -> Result<(), String> {
    let nr = cpu.regs[EAX];
    let a1 = cpu.regs[EBX];
    let a2 = cpu.regs[ECX];
    let a3 = cpu.regs[EDX];

    let ret: i32 = match nr {
        SYS_WRITE => {
            *write_calls += 1;
            let fd = a1 as i32;
            let len = a3 as usize;
            let buf = read_guest_bytes(ram, a2, len)
                .map_err(|errno| format!("SYS_write memory fault at {a2:#x}: errno {errno}"))?;
            match fd {
                1 => {
                    let mut out = io::stdout().lock();
                    out.write_all(&buf).map_err(|e| format!("stdout write: {e}"))?;
                    out.flush().map_err(|e| format!("stdout flush: {e}"))?;
                    buf.len() as i32
                }
                2 => {
                    let mut err = io::stderr().lock();
                    err.write_all(&buf).map_err(|e| format!("stderr write: {e}"))?;
                    err.flush().map_err(|e| format!("stderr flush: {e}"))?;
                    buf.len() as i32
                }
                _ => -9,
            }
        }
        SYS_EXIT => {
            *exit_code = Some(a1 as i32);
            a1 as i32
        }
        _ => -38,
    };

    cpu.regs[EAX] = ret as u32;
    Ok(())
}

fn main() -> Result<(), String> {
    let bin_path = env::args().nth(1).unwrap_or_else(|| "hello32.bin".to_string());
    let org: u32 = 0x0010_0000;

    let code = fs::read(&bin_path).map_err(|e| format!("failed to read {bin_path}: {e}"))?;
    if code.is_empty() {
        return Err(format!("{bin_path} is empty"));
    }

    let mut ram = Ram::new(0x0020_0000);
    ram.write_bytes(org, &code)
        .map_err(|f| format!("failed to load code at {org:#x}: {f:?}"))?;

    let (mut image, _) = Image::load(&[0x90]).map_err(|e| format!("Image::load failed: {e}"))?;
    configure_flat_32bit_image(&mut image, org, code.len() as u32);

    let mut cpu = Cpu {
        regs: [0; 8],
        eflags: 0x202,
        undefined_flags: 0,
        eip: org,
    };
    cpu.regs[ESP] = 0x001F_F000;

    let mut fpu = Fpu::default();
    let mut machine: SegmentState = image.segment_state();
    let mut blocks = Blocks::default();
    let mut cache = DecodeCache::default();

    let started = Instant::now();
    let mut total_steps: u64 = 0;
    let mut pil_steps: u64 = 0;
    let mut single_pil_steps: u64 = 0;
    let mut native_decodes: u64 = 0;
    let mut fallback_decodes: u64 = 0;
    let mut write_calls: u64 = 0;
    let mut exit_code: Option<i32> = None;

    while exit_code.is_none() {
        if let Some(run) = blocks.run(
            &mut cache,
            &image,
            &mut cpu,
            &mut ram,
            &mut fpu,
            &mut machine,
            4096,
            true,
        ) {
            total_steps += run.steps as u64;
            pil_steps += run.pil_steps as u64;
            if let Err(exec_err) = run.result {
                if let Some(ev) = Event::fault(&exec_err, cpu.eip) {
                    interrupt::deliver(&mut cpu, &mut image, &mut ram, ev)?;
                    machine = image.segment_state();
                    continue;
                }
                return Err(format!("block execution error at {:#x}: {exec_err:?}", cpu.eip));
            }
        }

        if exit_code.is_some() {
            break;
        }

        let decoded = cache
            .fetch(&image, &ram, cpu.eip)
            .map_err(|e| format!("decode error at {:#x}: {e:?}", cpu.eip))?;

        if decoded.native().is_some() {
            native_decodes += 1;
        } else {
            fallback_decodes += 1;
        }

        match &decoded.operation {
            Operation::Interrupt {
                vector: 0x80,
                overflow: _,
                next,
            } => {
                total_steps += 1;
                let next_eip = *next;
                handle_syscall_0x80(&mut cpu, &ram, &mut write_calls, &mut exit_code)?;
                if exit_code.is_none() {
                    cpu.eip = next_eip;
                }
            }
            Operation::Interrupt {
                vector,
                overflow: _,
                next,
            } => {
                total_steps += 1;
                let ev = Event::software(*vector, *next);
                interrupt::deliver(&mut cpu, &mut image, &mut ram, ev)?;
                machine = image.segment_state();
            }
            Operation::Halt => {
                total_steps += 1;
                exit_code = Some(0);
            }
            Operation::Input { next } | Operation::Output { next } => {
                total_steps += 1;
                cpu.eip = *next;
            }
            Operation::Pil {
                program,
                dst,
                src,
                flow,
                address,
            } => {
                let prev_eip = cpu.eip;
                let mut ctx: FlowContext = *flow;
                if ctx.next_eip == 0 {
                    ctx.next_eip = prev_eip.wrapping_add(decoded.length);
                }
                let mut lazy = LazyFlags::None;
                let res = pil_cpu::execute_machine_lazy(
                    program,
                    &mut cpu,
                    *dst,
                    *src,
                    ctx,
                    &mut ram,
                    *address,
                    &mut machine,
                    &mut lazy,
                );
                lazy.flush(&mut cpu.eflags, &mut cpu.undefined_flags);
                if let Err(exec_err) = res {
                    if let Some(ev) = Event::fault(&exec_err, cpu.eip) {
                        interrupt::deliver(&mut cpu, &mut image, &mut ram, ev)?;
                        machine = image.segment_state();
                        continue;
                    }
                    return Err(format!(
                        "PIL execution error at {prev_eip:#x} ({}): {exec_err:?}",
                        decoded.label
                    ));
                }
                if !program.uses_flow && cpu.eip == prev_eip {
                    cpu.eip = prev_eip.wrapping_add(decoded.length);
                }
                total_steps += 1;
                pil_steps += 1;
                single_pil_steps += 1;
            }
        }
    }

    let elapsed = started.elapsed();
    eprintln!(
        "line summary: exit={}, elapsed={:.3}s, steps={}, pil_steps={}, single_pil={}, outer_native={}, outer_fallback={}, sys_write={}, cache={}",
        exit_code.unwrap_or(0),
        elapsed.as_secs_f64(),
        total_steps,
        pil_steps,
        single_pil_steps,
        native_decodes,
        fallback_decodes,
        write_calls,
        cache.summary()
    );
    Ok(())
}
