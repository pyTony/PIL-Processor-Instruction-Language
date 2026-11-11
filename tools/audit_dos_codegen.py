#!/usr/bin/env python3
"""Inspect optimized LLVM for actual standalone blocks and flag-liveness probes."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import statistics
import hashlib

ROOT = Path(__file__).resolve().parents[1]


def functions(ir):
    result = {}
    for match in re.finditer(r'^define\b([^\n]*)\n(.*?)^}', ir, re.M | re.S):
        name = re.search(r'@("[^"]+"|[^ (]+)\(', match[1]).group(1).strip('"')
        result[name] = match[2]
    return result


def metrics(body):
    return dict(ir_instructions=sum(bool(re.match(r'\s+(?:%[^=]+ = |(?:ret|br|store|call|switch|unreachable)\b)', line)) for line in body.splitlines()),
                parity_popcount_calls=len(re.findall(r'\bcall\b[^\n]*@llvm\.ctpop', body)),
                integer_comparisons=len(re.findall(r'\bicmp\b', body)),
                calls=len(re.findall(r'\b(?:call|invoke)\b', body)))


def inspect(source, out, name):
    subprocess.run(['rustc', '--edition=2021', '-Awarnings', '-O', '-Ccodegen-units=1',
                    '--emit=llvm-ir,asm,link', '--crate-name', name, str(source), '--out-dir', str(out)], check=True)
    ir_path = out / f'{name}.ll'
    funcs = functions(ir_path.read_text())
    selected = {k: v for k, v in funcs.items() if 'ins_' in k or 'block_' in k or k.startswith('audit_')}
    executable=out/(name+('.exe' if os.name=='nt' else ''))
    return dict(source_bytes=source.stat().st_size,source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),
                llvm_bytes=ir_path.stat().st_size,executable_bytes=executable.stat().st_size,
                instruction_functions=sum('ins_' in k for k in funcs),
                block_functions=sum('block_' in k for k in funcs),
                generated_execution=metrics('\n'.join(selected.values())),
                probes={k: metrics(v) for k, v in funcs.items() if k.startswith('audit_')})


def probe_source(engine):
    modules = []
    for op in ('add', 'adc', 'inc', 'sbb', 'dec', 'je', 'jg'):
        text = subprocess.check_output([str(engine), '--emit-rust', '32', op], text=True)
        # The same forced inlining used by dos_codegen::instruction_module.
        for function in ('fn execute_state(', 'pub fn execute_with_flow(', 'pub fn execute_with_metadata('):
            text = text.replace(function, '#[inline(always)] ' + function)
        modules.append(f'mod op_{op} {{\n{text}\n}}\n')
    for name, ops, branch in [
        ('add1', ['add'], None), ('add8', ['add']*8, None),
        ('add_adc', ['add', 'adc'], None), ('add_inc', ['add', 'inc'], None),
        ('add_je', ['add'], 'je'), ('add_jg', ['add'], 'jg'),
        ('adc_inc_adc', ['adc', 'inc', 'adc'], None),
        ('sbb_dec_sbb', ['sbb', 'dec', 'sbb'], None),
        ('adc8', ['adc']*8, None), ('sbb8', ['sbb']*8, None),
        ('adc8_je', ['adc']*8, 'je'), ('sbb8_je', ['sbb']*8, 'je'),
    ]:
        body = ''.join(f'op_{op}::execute_with_metadata(&mut regs,&mut flags,&mut undefined,(0,0),(1,0)).unwrap();' for op in ops)
        if branch:
            body += f'let mut ip=0;op_{branch}::execute_with_flow(&mut regs,&mut flags,&mut undefined,&mut ip,(0,0),(1,0),op_{branch}::FlowContext{{next_eip:4,displacement:4,displacement_bits:32,code_limit:u32::MAX}}).unwrap();'
            result = '((regs[0] as u128)<<64)|u128::from(ip)'
        else:
            result = '((regs[0] as u128)<<64)|((undefined as u128)<<32)|u128::from(flags)'
        modules.append(f'#[no_mangle] pub fn audit_{name}(a:u32,b:u32,mut flags:u32,mut undefined:u32)->u128{{let mut regs=[a,b,0,0,0,0,0,0];{body}{result}}}\n')
    return ''.join(modules) + 'fn main(){}\n'


def benchmark(sources,out,frames,samples,arguments=()):
    binaries={}
    outputs={}
    for label,path in sources.items():
        # Validate complete output outside the timed loop, including same-size
        # differences that the timed byte-count checksum cannot detect.
        original=out/(f'check_{label}'+('.exe' if os.name=='nt' else ''))
        subprocess.run(['rustc','--edition=2021','-Awarnings','-O','-Ccodegen-units=1',str(path),'-o',str(original)],check=True)
        checked=subprocess.run([str(original),*arguments],capture_output=True,check=True)
        assert not checked.stderr,checked.stderr
        outputs[label]=checked.stdout
        lines=path.read_text().splitlines();changed=0
        for n,line in enumerate(lines):
            if line.startswith('fn main(){use std::io::Write;match run()'):
                lines[n]=f'fn main(){{let run_fn=std::hint::black_box(run as fn()->Result<(u8,Vec<u8>),String>);let start=std::time::Instant::now();let mut checksum=0usize;for _ in 0..{frames}{{let(code,bytes)=run_fn().unwrap();assert_eq!(code,0);checksum=checksum.wrapping_add(bytes.len());std::hint::black_box(bytes);}}eprintln!("BENCH {{}} {{}}",start.elapsed().as_nanos(),checksum);}}'
                changed+=1
        assert changed==1,'expected generated executable main'
        source=out/f'bench_{label}.rs';source.write_text('\n'.join(lines)+'\n')
        binary=out/(f'bench_{label}'+('.exe' if os.name=='nt' else ''))
        subprocess.run(['rustc','--edition=2021','-Awarnings','-O','-Ccodegen-units=1',str(source),'-o',str(binary)],check=True)
        binaries[label]=binary
    timings={k:[] for k in binaries};checksums=set()
    for n in range(samples):
        for label in list(binaries)[::1 if n%2==0 else -1]:
            result=subprocess.run([str(binaries[label]),*arguments],capture_output=True,check=True)
            match=re.fullmatch(rb'BENCH (\d+) (\d+)\r?\n',result.stderr);assert match,result.stderr
            timings[label].append(int(match[1])/1e6/frames);checksums.add(int(match[2]))
    assert len(checksums)==1,'variants produced different output sizes'
    assert len(set(outputs.values()))==1,'variants produced different complete output'
    return dict(unit='milliseconds per complete program run within process; RAM initialization and runtime setup included; no terminal output',
                arguments=list(arguments),output_bytes=len(next(iter(outputs.values()))),
                output_sha256=hashlib.sha256(next(iter(outputs.values()))).hexdigest(),
                frames_per_sample=frames,samples=timings,median_ms={k:statistics.median(v[1:]) for k,v in timings.items()},
                warmup='first sample per variant excluded; alternating order; whole generated builds, not an isolated flag-only comparison')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', type=Path, default=ROOT / 'target/debug' / ('pil.exe' if os.name == 'nt' else 'pil'))
    parser.add_argument('--baseline', type=Path, help='saved standalone source before shared blocks')
    parser.add_argument('--program',type=Path,default=ROOT/'programs/dos/MANDANSI.COM')
    parser.add_argument('--argument',action='append',default=[],help='guest argument (repeat for multiple arguments)')
    parser.add_argument('--out', type=Path, default=ROOT / 'target/compiler-optimization-audit')
    parser.add_argument('--benchmark',action='store_true')
    parser.add_argument('--frames',type=int,default=100)
    parser.add_argument('--samples',type=int,default=6)
    args = parser.parse_args()
    if args.frames<1 or args.samples<2:parser.error('frames must be positive and samples at least two')
    engine, out = args.engine.resolve(), args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    source = out / 'program.rs'
    # The emitter intentionally refuses overwrites; only replace this audit's own source.
    if source.exists(): source.unlink()
    program=args.program.resolve()
    subprocess.run([str(engine), '--emit-dos-standalone', str(program), str(source)], check=True)
    probes = out / 'probes.rs'; probes.write_text(probe_source(engine))
    report = dict(rustc=subprocess.check_output(['rustc', '--version'], text=True).strip(),
                  program=str(program),program_sha256=hashlib.sha256(program.read_bytes()).hexdigest(),arguments=args.argument,
                  flags='-O -Ccodegen-units=1 --emit=llvm-ir,asm,link',
                  note='Static LLVM instruction/call counts, not dynamic instruction counts or cycle measurements. Parity counts are ctpop call sites; other flag costs require inspecting dataflow.',
                  blocks=inspect(source, out, 'program_blocks'), probes=inspect(probes, out, 'flag_probes'))
    if args.baseline:
        report['baseline'] = inspect(args.baseline.resolve(), out, 'program_baseline')
    if args.benchmark:
        sources={'baseline':args.baseline.resolve(),'blocks':source} if args.baseline else {'blocks':source}
        report['benchmark']=benchmark(sources,out,args.frames,args.samples,args.argument)
    (out / 'audit.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
