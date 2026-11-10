import sys
from pathlib import Path
sys.path.insert(0, 'tools')
from dos_terminal import Session
base=Path('target/release/pil.exe').resolve()
native=next(Path('shadow/windows-x86_64/818e01a4cee4df09abe34562629bcfa99e63656c02bb46214bb63c8eae6ffb75').glob('pil-turbo.exe')).resolve()
sessions=[Session(e,'driveD/TP/BIN/TURBO.EXE',stop_log='') for e in (base,native)]
try:
 for i in range(800):
  budget=(int(sys.argv[1]) if i>=46 and len(sys.argv)>1 else [1,2,3,16,33,127,255,256,257,1000][i%10])
  before=[(s.last['cs'],s.last['eip'],s.last['instruction_bytes']) for s in sessions]
  replies=[s.send(f'run {budget}') for s in sessions]
  keys=['registers','selectors','eflags','eip','status','steps','screen']
  diff=[k for k in keys if replies[0][k]!=replies[1][k]]
  print(i,budget,before,[(r['eip'],r['registers'],r['eflags'],r['native_block_instructions']) for r in replies],diff,flush=True)
  if diff:
   print([{k:r.get(k) for k in ['status','detail','steps','cs','instruction_bytes','selectors']} for r in replies],flush=True)
   break
finally:
 for s in sessions:s.close()
