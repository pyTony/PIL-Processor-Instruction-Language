#!/usr/bin/env python3
"""Run the development-only SingleStepTests/80386 real-mode adapter.

The suite (software MIT, data public domain) is fetched at a pinned revision
into target/external-tests/SingleStepTests-80386; it is never committed and
never run by ordinary `cargo test`. Usage:

    python tools/verify_sst80386.py [--files 00,01,F6.4] [--no-generated]
"""
from pathlib import Path
import argparse,os,subprocess,sys
REPO='https://github.com/SingleStepTests/80386'
REVISION='459d49fbe6280e9ed46fee887b58dacd9cb880ab'
root=Path(__file__).resolve().parents[1]
data=root/'target'/'external-tests'/'SingleStepTests-80386'
out=root/'target'/'external-tests'/'sst80386'/'report'

def git(*args,cwd=None):
    subprocess.run(['git',*args],cwd=cwd,check=True)

def fetch():
    if not (data/'.git').exists():
        data.parent.mkdir(parents=True,exist_ok=True)
        git('clone','--no-checkout','--filter=blob:none',REPO,str(data))
    head=subprocess.run(['git','rev-parse','HEAD'],cwd=data,text=True,capture_output=True).stdout.strip()
    if head!=REVISION or not (data/'v1_ex_real_mode').is_dir():
        git('-c','advice.detachedHead=false','checkout','-q',REVISION,cwd=data)
    head=subprocess.run(['git','rev-parse','HEAD'],cwd=data,text=True,capture_output=True,check=True).stdout.strip()
    if head!=REVISION:
        sys.exit(f'ERROR: {data} is at {head}, expected pinned {REVISION}')

def main():
    p=argparse.ArgumentParser(description=__doc__,formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument('--files',help='comma-separated MOO file stems, e.g. 00,6601,F6.4 (default: all)')
    p.add_argument('--no-generated',action='store_true',help='run the PIL interpreter only')
    a=p.parse_args()
    fetch()
    env=dict(os.environ,SST80386_DIR=str(data),SST80386_OUT=str(out))
    if a.files: env['SST80386_FILES']=a.files
    if a.no_generated: env['SST80386_GENERATED']='0'
    r=subprocess.run(['cargo','test','--release','--locked','sst80386_real_mode_suite','--','--ignored','--nocapture'],cwd=root,env=env)
    if r.returncode: sys.exit(r.returncode)
    print(f'Suite revision {REVISION}; report {out/"report.md"}; failing cases {out/"failures.tsv"}')

if __name__=='__main__':
    main()
