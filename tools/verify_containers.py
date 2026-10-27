#!/usr/bin/env python3
"""Independent ZIP/zlib and synthetic FAT16 CLI interoperability checks."""
from pathlib import Path
import io,struct,subprocess,tempfile,zipfile,hashlib
ROOT=Path(__file__).resolve().parents[1]
def cli(*args):return subprocess.check_output(['cargo','run','--quiet','--locked','--',*map(str,args)],cwd=ROOT,text=True,stderr=subprocess.STDOUT)
def w16(b,n,v):struct.pack_into('<H',b,n,v)
def w32(b,n,v):struct.pack_into('<I',b,n,v)
def de(name,attrs,c,size):
 b=bytearray(32);b[:11]=name;b[11]=attrs;w16(b,26,c);w32(b,28,size);return b
def fat(hello,package):
 assert len(package)<=512
 base=63*512;sectors=4133;b=bytearray(base+sectors*512);b[450]=4;w32(b,454,63);w32(b,458,sectors);b[510:512]=b'\x55\xaa'
 boot=bytearray(512);boot[:3]=b'\xeb\x3c\x90';w16(boot,11,512);boot[13]=1;w16(boot,14,1);boot[16]=2;w16(boot,17,32);w16(boot,19,sectors);boot[21]=0xf8;w16(boot,22,17);boot[510:]=b'\x55\xaa';b[base:base+512]=boot
 for f in range(2):
  at=base+512+f*17*512
  for c,v in [(0,0xfff8),(1,0xffff),(2,0xffff),(3,0xffff),(4,0xffff)]:w16(b,at+c*2,v)
 root=base+35*512;b[root:root+32]=de(b'HELLO   COM',0,2,len(hello));b[root+32:root+64]=de(b'PKGS       ',16,3,0)
 at=base+37*512;b[at:at+len(hello)]=hello;b[at+512:at+544]=de(b'PACKAGE ZIP',0,4,len(package));b[at+1024:at+1024+len(package)]=package
 return b
class Streaming(io.BytesIO):
 def seekable(self):return False
 def seek(self,*args):raise io.UnsupportedOperation('stream')
with tempfile.TemporaryDirectory(prefix='pil-containers-')as temp:
 d=Path(temp);com=d/'hello.com';exe=d/'hello.exe';cli('--write-dos-hello','com',com);cli('--write-dos-hello','exe',exe)
 stream=Streaming()
 with zipfile.ZipFile(stream,'w',compression=zipfile.ZIP_DEFLATED)as z:z.writestr('BIN/HELLO.COM',com.read_bytes());z.writestr('BIN/HELLO.EXE',exe.read_bytes())
 package=stream.getvalue();archive=d/'programs.zip';archive.write_bytes(package)
 image=d/'programs.img';image.write_bytes(fat(com.read_bytes(),package))
 for p in [archive,image]:
  before=hashlib.sha256(p.read_bytes()).hexdigest();listing=cli('--dos-list',p);assert 'HELLO.COM'in listing and 'HELLO.EXE'in listing,listing
  report=cli('--dos-probe',p);assert 'SUMMARY imported=2/2 exited=2'in report,report
  out=d/(p.suffix[1:]+'-out');cli('--dos-extract',p,out)
  prefix=out if p==archive else out/'PKGS/PACKAGE.ZIP.contents'
  assert(prefix/'BIN/HELLO.COM').read_bytes()==com.read_bytes();assert(prefix/'BIN/HELLO.EXE').read_bytes()==exe.read_bytes()
  assert hashlib.sha256(p.read_bytes()).hexdigest()==before
  proc=subprocess.run(['cargo','run','--quiet','--locked','--','--dos-extract',str(p),str(out)],cwd=ROOT,capture_output=True);assert proc.returncode!=0
 # Multiple independently compressed payloads cover stored/fixed/dynamic data.
 blob=(bytes(range(256))*16)+(b'original container interoperability '*1000)
 many=d/'vectors.zip'
 with zipfile.ZipFile(many,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=9)as z:z.writestr('DATA/VECTOR.DAT',blob)
 out=d/'vectors-out';cli('--dos-extract',many,out);assert(out/'DATA/VECTOR.DAT').read_bytes()==blob
 bad=d/'traversal.zip'
 with zipfile.ZipFile(bad,'w')as z:z.writestr('../escape.com',com.read_bytes())
 proc=subprocess.run(['cargo','run','--quiet','--locked','--','--dos-extract',str(bad),str(d/'bad-out')],cwd=ROOT,capture_output=True)
 assert proc.returncode!=0 and not(d/'bad-out').exists()and not(d/'escape.com').exists()
print('PASS: independent FAT16/ZIP CLI checks passed MBR image, streaming deflate/data descriptors, DOS probe deduplication, extraction byte equality, input immutability, overwrite and traversal rejection.')
