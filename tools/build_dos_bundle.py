#!/usr/bin/env python3
"""Build a linked shadow engine for one or more DOS executables or directories.

Profiles run without host-folder mounts. Use --train PROGRAM=KEYS.txt to type
a representative workload into an interactive program. Unobserved code falls
back to Dine. Original executables and data stay in their original locations.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def candidates(inputs, recursive):
    found = {}
    for value in inputs:
        path = value.resolve()
        paths = (path.rglob('*') if recursive else path.iterdir()) if path.is_dir() else [path]
        for item in paths:
            if item.is_file() and not item.is_symlink() and item.suffix.lower() in ('.com', '.exe'):
                found[str(item)] = item
    return sorted(found.values(), key=lambda p: str(p).lower())


def bundle_source(sources):
    modules = []
    for index, source in enumerate(sources):
        # Keep each small basic block optimized, but avoid asking LLVM to
        # inline hundreds of overlapping blocks into one enormous dispatcher.
        source = source.replace('#[inline(always)] fn block_', '#[inline(never)] fn block_')
        # Inline sources so Cargo's build script only needs to track this file.
        modules.append(f'mod program_{index} {{\n' + source + '\npub fn accepts(bytes:&[u8])->bool{bytes==ORIGINAL}\n}\n')
    select = '\n'.join(f'if program_{i}::accepts(&active.bytes){{return Some({i});}}' for i in range(len(sources)))
    init = ','.join(f'Box::new(program_{i}::Backend::default())' for i in range(len(sources)))
    return ''.join(modules) + '''
use crate::{dos,guest,Cpu,Ram,address,ExecError};
pub(crate) struct Backend { backends:Vec<Box<dyn dos::InstructionBackend>>, active:Option<(std::sync::Arc<Vec<u8>>,Option<usize>)> }
impl Default for Backend {fn default()->Self{Self{backends:vec![INIT],active:None}}}
impl Backend {
 fn select(&mut self,active:&dos::ProgramIdentity)->Option<usize>{
  if let Some((old,index))=&self.active{if std::sync::Arc::ptr_eq(old,&active.bytes){return *index;}}
  let index=(||{SELECT None})();self.active=Some((active.bytes.clone(),index));index
 }
}
impl dos::InstructionBackend for Backend {
 fn may_block(&mut self,active:&dos::ProgramIdentity,image:&guest::Image,ram:&Ram,ip:u32)->bool{self.select(active).is_some_and(|i|self.backends[i].may_block(active,image,ram,ip))}
 fn may_execute(&mut self,active:&dos::ProgramIdentity,image:&guest::Image,ram:&Ram,ip:u32)->bool{self.select(active).is_some_and(|i|self.backends[i].may_execute(active,image,ram,ip))}
 fn block_stats(&self)->(u64,u64){self.backends.iter().map(|b|b.block_stats()).fold((0,0),|(a,b),(c,d)|(a+c,b+d))}
 fn block(&mut self,active:&dos::ProgramIdentity,image:&guest::Image,cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState,budget:usize)->Option<(usize,Result<(),ExecError>)>{let i=self.select(active)?;self.backends[i].block(active,image,cpu,ram,machine,budget)}
 fn execute(&mut self,active:&dos::ProgramIdentity,image:&guest::Image,decoded:&std::rc::Rc<guest::Decoded>,cpu:&mut Cpu,ram:&mut Ram,machine:&mut address::SegmentState)->Option<Result<(),ExecError>>{let i=self.select(active)?;self.backends[i].execute(active,image,decoded,cpu,ram,machine)}
}
'''.replace('INIT', init).replace('SELECT', select)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('inputs', nargs='+', type=Path)
    parser.add_argument('--recursive', action='store_true')
    parser.add_argument('--engine', type=Path, required=True, help='rebuilt profiling engine')
    parser.add_argument('--train', action='append', default=[], metavar='PROGRAM=KEYS.txt')
    parser.add_argument('--steps', type=int, default=2_000_000, help='profile budget per program (default 2000000)')
    parser.add_argument('--sites', type=int, default=512, help='maximum compiled sites per program (default 512 hottest sites)')
    parser.add_argument('--limit', type=int, help='limit number of distinct executables to profile')
    parser.add_argument('--output', type=Path, default=ROOT / 'shadow/bundle')
    args = parser.parse_args()
    if not 1 <= args.steps <= 120_000_000 or not 1 <= args.sites <= 65536 or args.limit is not None and args.limit < 1:
        parser.error('invalid steps, sites or limit')
    training = {}
    for item in args.train:
        name, sep, value = item.partition('=')
        if not sep or not Path(value).is_file():
            parser.error('--train needs PROGRAM=existing_KEYS.txt')
        training[name.lower()] = Path(value).resolve()
    paths = candidates(args.inputs, args.recursive)
    for name in training:
        matches = [p for p in paths if p.name.lower() == name or str(p).lower() == name]
        if len(matches) != 1:
            parser.error(f'training name {name!r} must match exactly one executable')
    directory = args.output.resolve(); directory.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy(); env.pop('PIL_SHADOW_SOURCE', None)
    host = next(s.split(': ', 1)[1] for s in subprocess.check_output(['rustc', '-vV'], text=True).splitlines() if s.startswith('host: '))
    sources, records, seen = [], [], set()
    for path in paths:
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest in seen: continue
        if args.limit is not None and len(seen) >= args.limit: break
        seen.add(digest)
        source = directory / f'{digest}.rs'
        # Regenerate into a fresh path; the emitter refuses overwrite.
        pending = directory / f'{digest}.pending.rs'
        if pending.exists(): pending.unlink()
        keys = training.get(str(path).lower(), training.get(path.name.lower()))
        command = [str(args.engine.resolve()), '--emit-dos-backend', str(path), str(pending), '--steps', str(args.steps), '--sites', str(args.sites)]
        if keys: command += ['--keys', str(keys)]
        result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True)
        record = dict(path=str(path), sha256=digest, training=str(keys) if keys else None,
                      training_sha256=hashlib.sha256(keys.read_bytes()).hexdigest() if keys else None,
                      profile=result.stdout.strip(), error=result.stderr.strip() if result.returncode else None)
        records.append(record)
        if result.returncode:
            print(f'SKIP {path}: {record["error"]}', flush=True); continue
        pending.replace(source); sources.append(source.read_text(encoding='utf-8'))
        print(f'{path.name}: {record["profile"]}', flush=True)
    manifest = dict(format=1, target=host, steps=args.steps, max_sites=args.sites, programs=records, data='original mounted drives; no data copied', coverage='observed sites only, interpreter fallback')
    (directory / 'manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
    if not sources: parser.error('no compilable instruction sites found; see manifest.json')
    backend = directory / 'bundle.rs'; backend.write_text(bundle_source(sources), encoding='utf-8')
    env['PIL_SHADOW_SOURCE'] = str(backend)
    env['CARGO_TARGET_DIR'] = str(ROOT / 'target/dos-bundle-build')
    subprocess.run(['cargo', 'build', '--release', '--locked', '--target', host], cwd=ROOT, env=env, check=True)
    suffix = '.exe' if os.name == 'nt' else ''
    built = Path(env['CARGO_TARGET_DIR']) / host / 'release' / f'pil{suffix}'
    dest = directory / f'pil-shadow{suffix}'
    import shutil
    pending = directory / f'engine.pending{suffix}'; shutil.copyfile(built, pending); pending.replace(dest)
    manifest['engine'] = str(dest)
    manifest['engine_sha256'] = hashlib.sha256(dest.read_bytes()).hexdigest()
    manifest['source_sha256'] = hashlib.sha256(backend.read_bytes()).hexdigest()
    (directory / 'manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
    print(f'Shadow bundle: {dest}', flush=True)


if __name__ == '__main__':
    main()
