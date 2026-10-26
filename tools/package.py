#!/usr/bin/env python3
"""Package a tested native release build, retaining documentation and provenance."""
from pathlib import Path
import hashlib,os,platform,re,subprocess,tempfile,zipfile
root=Path(__file__).resolve().parents[1]
build=Path(os.environ.get('CARGO_TARGET_DIR',root/'target'))
if not build.is_absolute():build=root/build
binary=build/'release'/('pil-proof.exe' if os.name=='nt' else 'pil-proof')
subprocess.run([str(binary),'--selftest'],check=True)
version=re.search(r'^version = "([^"]+)"', (root/'Cargo.toml').read_text(),re.M)[1]
name=f'pil-proof-{version}-{platform.system().lower()}-{platform.machine().lower()}'
dist=root/'dist';dist.mkdir(exist_ok=True)
with tempfile.TemporaryDirectory() as temp:
    image=Path(temp)/'selftest.bin'
    subprocess.run([str(binary),'--write-selftest',str(image)],check=True)
    files={binary:binary.name,image:'programs/selftest.bin'}
    for src,dst in [('delivery/README.md','README.md'),('delivery/debug.sh','debug.sh'),('delivery/debug.ps1','debug.ps1'),('docs/DEBUGGER.md','DEBUGGER.md'),('docs/REFERENCES_AND_CREDITS.md','REFERENCES_AND_CREDITS.md'),('docs/REVISION_HISTORY.md','REVISION_HISTORY.md'),('docs/SHA256SUMS','SOURCE_SHA256SUMS'),('programs/selftest.lst','programs/selftest.lst')]:
        files[root/src]=dst
    with zipfile.ZipFile(dist/(name+'.zip'),'w',zipfile.ZIP_DEFLATED) as archive:
        manifest=[]
        for source,target in files.items():
            archive.write(source,'pil-proof/'+target)
            manifest.append(hashlib.sha256(source.read_bytes()).hexdigest()+'  '+target)
        archive.writestr('pil-proof/SHA256SUMS','\n'.join(sorted(manifest))+'\n')
    unpacked=Path(temp)/'unpacked'
    with zipfile.ZipFile(dist/(name+'.zip')) as archive:archive.extractall(unpacked)
    delivery=unpacked/'pil-proof';exe=delivery/binary.name
    exe.chmod(0o755) # Python extractall does not restore ZIP Unix permissions.
    subprocess.run([str(exe),'--selftest'],cwd=delivery,check=True)
    commands='serial rx 41\nrun 1000\ncheck\nserial tx\nquit\n'
    launch=['pwsh','-NoProfile','-File','debug.ps1'] if os.name=='nt' else ['sh','debug.sh']
    result=subprocess.run(launch,cwd=delivery,input=commands,text=True,capture_output=True,check=True)
    if 'PASS: selftest' not in result.stdout or 'serial output: 41 | A' not in result.stdout or 'TX: 41' not in result.stdout:
        raise SystemExit('unpacked debugger smoke test failed:\n'+result.stdout+result.stderr)
    print('Unpacked delivery: binary, launcher, guest program and serial console passed')
print(dist/(name+'.zip'))
