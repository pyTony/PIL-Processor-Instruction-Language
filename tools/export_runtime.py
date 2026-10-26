#!/usr/bin/env python3
"""Export a strict user tree from dev. main is never a merge of the dev tree."""
from pathlib import Path
import argparse,hashlib,json,os,re,subprocess,sys
ROOT=Path(__file__).resolve().parents[1]
# Explicit run/build inputs only. Tests and development documents cannot leak
# through a broad repository copy. These patterns are intentionally narrow.
PATTERNS=('src/*.rs','isa/i386/*.pil','programs/selftest.hex','programs/selftest.lst','programs/hello.hex','programs/hello.lst',
          'Cargo.toml','Cargo.lock','.gitattributes','tools/build.sh','tools/build.ps1',
          'tools/debug.sh','tools/debug.ps1')
DOCUMENTS={'README.md':'README.md','DEBUGGER.md':'docs/DEBUGGER.md',
           'REFERENCES_AND_CREDITS.md':'docs/REFERENCES_AND_CREDITS.md',
           'RELEASE_NOTES.md':'docs/RELEASE_NOTES.md'}
TEST_MODULE=re.compile(r'(?m)^#\[cfg\(test\)\]\n#\[path[^\n]*\]\nmod \w+;\n?')

def export(destination,source_ref):
    if destination.exists() and any(destination.iterdir()):
        raise SystemExit('destination must be empty; refusing to overwrite existing work')
    destination.mkdir(parents=True,exist_ok=True)
    files={}
    for pattern in PATTERNS:
        matched=sorted(ROOT.glob(pattern))
        if not matched:raise SystemExit(f'missing runtime input: {pattern}')
        for path in matched:
            relative=path.relative_to(ROOT).as_posix();data=path.read_bytes()
            if path.suffix=='.rs':data=TEST_MODULE.sub('',data.decode()).encode()
            files[relative]=data
    for source,target in DOCUMENTS.items():files[target]=(ROOT/'delivery/source'/source).read_bytes()
    files['.gitignore']=b'/target/\n'
    version=re.search(r'^version = "([^"]+)"',(ROOT/'Cargo.toml').read_text(),re.M)[1]
    files['DELIVERY.json']=(json.dumps({'version':version,'source_commit':source_ref,
        'source_branch':'dev','layout':'runtime-source','build':'sh tools/build.sh',
        'debug':'sh tools/debug.sh'},indent=2)+'\n').encode()
    files['SHA256SUMS']=''.join(hashlib.sha256(data).hexdigest()+'  '+path+'\n' for path,data in sorted(files.items())).encode()
    for relative,data in files.items():
        path=destination/relative;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(data)
    # Validate local doc links and that stripped test modules cannot require dev.
    for relative,data in files.items():
        if relative.endswith('.md'):
            for link in re.findall(r'\]\(([^)]+)\)',data.decode()):
                if '://' not in link and not link.startswith('#'):
                    target=(Path(relative).parent/link.split('#')[0]).as_posix()
                    if target not in files:raise SystemExit(f'broken user link: {relative}: {link}')
        if relative.endswith('.rs') and TEST_MODULE.search(data.decode()):raise SystemExit(f'test module leaked: {relative}')
    print(f'Exported {len(files)} user files from {source_ref} to {destination}')

def verify(destination):
    env=os.environ.copy();env['CARGO_TARGET_DIR']=str(destination/'target')
    # Runtime builds must stand alone, including a harmless cargo test invocation:
    # dev-only modules have been removed instead of leaving dangling test paths.
    for command in [['cargo','build','--locked'],['cargo','test','--locked']]:
        subprocess.run(command,cwd=destination,env=env,check=True)
    windows=os.name=='nt'
    script=['pwsh','-NoProfile','-File','tools/debug.ps1'] if windows else ['sh','tools/debug.sh']
    subprocess.run(script+(['-Check'] if windows else ['--check']),cwd=destination,env=env,check=True)
    result=subprocess.run(script,cwd=destination,env=env,input='serial rx 41\nrun 1000\ncheck\nserial tx\ndemo hello\nrun 1000\nserial tx\nquit\n',text=True,capture_output=True,check=True)
    if not all(text in result.stdout for text in ['PASS: selftest','serial output: 41 | A','TX: 41','Hello, world!\\n','TX: 48 65 6c 6c 6f 2c 20 77 6f 72 6c 64 21 0a']):
        raise SystemExit('runtime debugger transcript failed:\n'+result.stdout+result.stderr)
    exe=destination/'target/debug'/('pil-proof.exe' if windows else 'pil-proof')
    generated=subprocess.check_output([str(exe),'--emit-rust','32','add_ram'],cwd=destination,env=env)
    source=destination/'target/generated.rs';source.write_bytes(generated)
    subprocess.run([os.environ.get('RUSTC','rustc'),'--edition=2021','--crate-type','lib',str(source),'--out-dir',str(destination/'target')],cwd=destination,env=env,check=True,capture_output=True)
    print('Standalone runtime build, launchers, serial transcript and generated Rust passed')

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('destination',type=Path)
    parser.add_argument('--source-ref');parser.add_argument('--verify',action='store_true');args=parser.parse_args()
    source_ref=args.source_ref or subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
    destination=args.destination.resolve()
    # Protect both the dev checkout and any ancestor of it from accidental export.
    if destination==ROOT or destination in ROOT.parents:raise SystemExit('destination must not contain the dev checkout')
    export(destination,source_ref)
    if args.verify:verify(destination)
