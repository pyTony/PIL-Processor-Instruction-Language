import sys, time
from pathlib import Path
sys.path.insert(0,'tools')
from dos_terminal import Session, screen_rows
engines=[Path('scratch/pil-before-miss.exe'),Path('shadow/windows-x86_64/e2dc0a332efa599441b8e3880c9912aed514dd4fefae9cd65637f023cb6d2f17/pil-lm60.exe')]
if len(sys.argv)>1: engines[1]=Path(sys.argv[1])
if len(sys.argv)>2: engines[0]=Path(sys.argv[2])
frames=[]
for sample in range(3):
 for index in ([0,1] if sample%2==0 else [1,0]):
  s=Session(engines[index].resolve(),'driveD/dosbench/MARKS/LM60/LM60.EXE',args='/b /q /3',stop_log='')
  try:
   start=time.perf_counter()
   for _ in range(5):r=s.send('run 1000000 buffered')
   duration=time.perf_counter()-start
   r=s.send('snapshot')
   print(sample,index,round(duration,4),r['status'],r['detail'],'steps',r['steps'],'native',r['native_block_instructions'],'location',r['cs'],r['eip'],flush=True)
   print('\n'.join(t.rstrip() for t,_ in screen_rows(r))[:180],flush=True)
   frames.append(r)
  finally:s.close()
for key in ['registers','selectors','eflags','eip','steps','status']:
 print(key,all(f[key]==frames[0][key] for f in frames),flush=True)
