#!/usr/bin/env python3
"""Run the development-only SingleStepTests/80386 real-mode adapter.

The suite (software MIT, data public domain) is fetched at a pinned revision
into target/external-tests/SingleStepTests-80386; it is never committed and
never run by ordinary `cargo test`. Usage:

    python tools/verify_sst80386.py [--files 00,01,F6.4] [--no-generated] [--download]

By default the suite is fetched with git. --download fetches the needed files
one at a time from raw.githubusercontent.com instead, resuming interrupted
transfers and checking every file against its git blob hash at the pinned
revision (listed in tools/sst80386_manifest.txt); use it when a network drops long git transfers. A directory filled
this way remembers it, so later runs keep using per-file downloads.
"""
from pathlib import Path
import argparse,hashlib,os,subprocess,sys,time,urllib.error,urllib.request
REPO='https://github.com/SingleStepTests/80386'
REVISION='459d49fbe6280e9ed46fee887b58dacd9cb880ab'
RAW=f'https://raw.githubusercontent.com/SingleStepTests/80386/{REVISION}/'
SET='v1_ex_real_mode'
root=Path(__file__).resolve().parents[1]
data=root/'target'/'external-tests'/'SingleStepTests-80386'
out=root/'target'/'external-tests'/'sst80386'/'report'
marker=data/'.pil-downloaded-revision'
MANIFEST=Path(__file__).with_name('sst80386_manifest.txt')

def git(*args,cwd=None):
    subprocess.run(['git',*args],cwd=cwd,check=True)

def fetch_git():
    try:
        if not (data/'.git').exists():
            data.parent.mkdir(parents=True,exist_ok=True)
            git('clone','--no-checkout','--filter=blob:none',REPO,str(data))
        head=subprocess.run(['git','rev-parse','HEAD'],cwd=data,text=True,capture_output=True).stdout.strip()
        if head!=REVISION or not (data/SET).is_dir():
            git('-c','advice.detachedHead=false','checkout','-q',REVISION,cwd=data)
    except subprocess.CalledProcessError:
        sys.exit('ERROR: git could not fetch the suite. If the connection drops during long '
                 'transfers, rerun with --download to fetch the files one at a time.')
    head=subprocess.run(['git','rev-parse','HEAD'],cwd=data,text=True,capture_output=True,check=True).stdout.strip()
    if head!=REVISION:
        sys.exit(f'ERROR: {data} is at {head}, expected pinned {REVISION}')

def blob_sha1(path):
    h=hashlib.sha1(f'blob {path.stat().st_size}\0'.encode())
    with open(path,'rb') as f:
        for chunk in iter(lambda:f.read(1<<20),b''): h.update(chunk)
    return h.hexdigest()

def get(url,dest,size=None,tries=8):
    """Downloads url to dest through dest.part, resuming after a dropped connection."""
    part=dest.with_name(dest.name+'.part')
    for attempt in range(tries):
        have=part.stat().st_size if part.exists() else 0
        if size is not None and have>size: part.unlink();have=0
        req=urllib.request.Request(url,headers={'User-Agent':'pil-sst80386-fetch'})
        if have: req.add_header('Range',f'bytes={have}-')
        try:
            with urllib.request.urlopen(req,timeout=60) as r:
                if have and r.status!=206: have=0
                with open(part,'ab' if have else 'wb') as f:
                    for chunk in iter(lambda:r.read(1<<16),b''): f.write(chunk)
            if size is None or part.stat().st_size==size:
                part.replace(dest);return
        except urllib.error.HTTPError as e:
            if e.code==416: part.unlink(missing_ok=True)
            elif e.code<500 and e.code!=429: raise
        except (OSError,urllib.error.URLError) as e:
            print(f'  {dest.name}: {e}; retrying',file=sys.stderr)
        time.sleep(min(2**attempt,30))
    sys.exit(f'ERROR: could not download {url} after {tries} attempts')

def fetch_files(stems):
    blobs={}
    for line in MANIFEST.read_text().splitlines():
        if line and not line.startswith('#'):
            sha,size,path=line.split(' ',2);blobs[path]=(sha,int(size))
    wanted=['revocation_list.txt','LICENSE','README.md']
    if stems: wanted+=[f'{SET}/{s}.MOO.gz' for s in stems]
    else: wanted+=sorted(p for p in blobs if p.startswith(SET+'/'))
    missing=[p for p in wanted if p not in blobs]
    if missing: sys.exit(f'ERROR: not in the suite at {REVISION[:8]}: {", ".join(missing)}')
    data.mkdir(parents=True,exist_ok=True)
    marker.write_text(REVISION+'\n')
    for i,path in enumerate(wanted,1):
        sha,size=blobs[path];dest=data/path
        dest.parent.mkdir(parents=True,exist_ok=True)
        if dest.exists() and blob_sha1(dest)==sha: continue
        print(f'[{i}/{len(wanted)}] {path} ({size:,} bytes)',flush=True)
        dest.unlink(missing_ok=True)
        get(RAW+path,dest,size)
        if blob_sha1(dest)!=sha:
            dest.unlink();sys.exit(f'ERROR: {path} does not match blob {sha}; run again')

def main():
    p=argparse.ArgumentParser(description=__doc__,formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument('--files',help='comma-separated MOO file stems, e.g. 00,6601,F6.4 (default: all)')
    p.add_argument('--no-generated',action='store_true',help='run the PIL interpreter only')
    p.add_argument('--download',action='store_true',help='fetch files one at a time over HTTPS instead of git')
    p.add_argument('--fetch-only',action='store_true',help='fetch the suite and stop')
    a=p.parse_args()
    stems=[s.strip() for s in a.files.split(',') if s.strip()] if a.files else []
    if a.download or (marker.exists() and marker.read_text().strip()==REVISION and not (data/'.git').exists()):
        fetch_files(stems)
    else:
        fetch_git()
    if a.fetch_only:
        print(f'Suite revision {REVISION} in {data}');return
    env=dict(os.environ,SST80386_DIR=str(data),SST80386_OUT=str(out))
    if a.files: env['SST80386_FILES']=a.files
    if a.no_generated: env['SST80386_GENERATED']='0'
    r=subprocess.run(['cargo','test','--release','--locked','sst80386_real_mode_suite','--','--ignored','--nocapture'],cwd=root,env=env)
    if r.returncode: sys.exit(r.returncode)
    print(f'Suite revision {REVISION}; report {out/"report.md"}; failing cases {out/"failures.tsv"}')

if __name__=='__main__':
    main()
