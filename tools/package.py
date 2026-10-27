#!/usr/bin/env python3
"""Package a tested native release build, retaining documentation and provenance."""
from pathlib import Path
import hashlib,os,platform,re,subprocess,tempfile,zipfile
root=Path(__file__).resolve().parents[1]
build=Path(os.environ.get('CARGO_TARGET_DIR',root/'target'))
if not build.is_absolute():build=root/build
binary=build/'release'/('pil-proof.exe' if os.name=='nt' else 'pil-proof')
subprocess.run([str(binary),'--selftest'],check=True)
subprocess.run([str(binary),'--numeric-test'],check=True)
version=re.search(r'^version = "([^"]+)"', (root/'Cargo.toml').read_text(),re.M)[1]
name=f'pil-proof-{version}-{platform.system().lower()}-{platform.machine().lower()}'
dist=root/'dist';dist.mkdir(exist_ok=True)
with tempfile.TemporaryDirectory() as temp:
    image=Path(temp)/'selftest.bin'
    subprocess.run([str(binary),'--write-selftest',str(image)],check=True)
    hello=Path(temp)/'hello.bin'
    subprocess.run([str(binary),'--write-hello',str(hello)],check=True)
    files={binary:binary.name,image:'programs/selftest.bin',hello:'programs/hello.bin'}
    for example in ['factorials','loop']:
        path=Path(temp)/(example+'.bin')
        subprocess.run([str(binary),'--write-'+example,str(path)],check=True)
        files[path]='programs/'+example+'.bin'
    for src,dst in [('delivery/README.md','README.md'),('docs/LANGUAGE.md','PIL_REFERENCE.md'),('docs/DOS_PROBE.md','DOS_PROBE.md'),('docs/CONTAINERS.md','CONTAINERS.md'),('delivery/debug.sh','debug.sh'),('delivery/debug.ps1','debug.ps1'),('delivery/source/DEBUGGER.md','DEBUGGER.md'),('delivery/source/REFERENCES_AND_CREDITS.md','REFERENCES_AND_CREDITS.md'),('delivery/source/RELEASE_NOTES.md','RELEASE_NOTES.md'),('delivery/source/FEATURE_MATRIX.md','FEATURE_MATRIX.md'),('programs/selftest.lst','programs/selftest.lst'),('programs/hello.lst','programs/hello.lst'),('programs/factorials.lst','programs/factorials.lst'),('programs/loop.lst','programs/loop.lst')]:
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
    subprocess.run([str(exe),'--numeric-test'],cwd=delivery,check=True)
    subprocess.run([str(exe),'--compare-backends','16','2'],cwd=delivery,check=True)
    subprocess.run([str(exe),'--mandelbrot','8','8'],cwd=delivery,check=True)
    commands='serial rx 41\nrun 1000\ncheck\nserial tx\nload programs/hello.bin\nrun 1000\nserial tx\nload programs/factorials.bin\ntime 1000\ncheck\ndemo extensions\nrun 1000\ncheck\nbench 16 2\ncache stats\ncache off\nbench 16 2\ncache on\nstress 2\nquit\n'
    launch=['pwsh','-NoProfile','-File','debug.ps1'] if os.name=='nt' else ['sh','debug.sh']
    result=subprocess.run(launch,cwd=delivery,input=commands,text=True,capture_output=True,check=True)
    if 'PASS: selftest' not in result.stdout or 'serial output: 41 | A' not in result.stdout or 'TX: 41' not in result.stdout or 'Hello, world!\\n' not in result.stdout or 'PASS: factorials' not in result.stdout or 'PASS: extensions' not in result.stdout or 'PASS: counted-loop benchmark' not in result.stdout or 'PASS: factorial stress' not in result.stdout or 'cache=on (cold per sample)' not in result.stdout or 'cache=off (cold per sample)' not in result.stdout:
        raise SystemExit('unpacked debugger smoke test failed:\n'+result.stdout+result.stderr)
    com=delivery/'programs/dos-hello.com';mz=delivery/'programs/dos-hello.exe'
    for kind,path in [('com',com),('exe',mz)]:
        subprocess.run([str(exe),'--write-dos-hello',kind,str(path)],cwd=delivery,check=True)
    dos=subprocess.check_output([str(exe),'--dos-probe',str(com),str(mz)],cwd=delivery,text=True)
    if 'SUMMARY imported=2/2 exited=2' not in dos or dos.count('OUTPUT (escaped bytes): Hello, DOS!\\r\\n')!=2:
        raise SystemExit('unpacked DOS probe failed:\n'+dos)
    archive=delivery/'programs/dos-programs.zip'
    with zipfile.ZipFile(archive,'w',zipfile.ZIP_DEFLATED)as z:
        z.write(com,'BIN/HELLO.COM');z.write(mz,'BIN/HELLO.EXE')
    listed=subprocess.check_output([str(exe),'--dos-list',str(archive)],cwd=delivery,text=True)
    probed=subprocess.check_output([str(exe),'--dos-probe',str(archive)],cwd=delivery,text=True)
    extracted=delivery/'dos-extracted'
    subprocess.run([str(exe),'--dos-extract',str(archive),str(extracted)],cwd=delivery,check=True)
    if '2 executable candidates' not in listed or 'SUMMARY imported=2/2 exited=2' not in probed or (extracted/'BIN/HELLO.COM').read_bytes()!=com.read_bytes():
        raise SystemExit('unpacked ZIP list/probe/extract failed')
    print('Unpacked ZIP list/probe/extract passed')
    print('Unpacked delivery DOS COM/MZ hello passed')
    print('Unpacked delivery: binary, launcher, guest program and serial console passed')
print(dist/(name+'.zip'))
