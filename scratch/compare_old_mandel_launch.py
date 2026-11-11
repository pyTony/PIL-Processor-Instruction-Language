import sys,time
from pathlib import Path
sys.path.insert(0,'tools')
from dos_terminal import Session,run_slice,refresh_frame
engine=Path('shadow/windows-x86_64/aa6c44afc7e6917cbaf789fea10de125b8aaae03b8966bc2f0efd8e3ce318ca8/pil-mandvga.exe').resolve()
if len(sys.argv)>1: engine=Path(sys.argv[1]).resolve()
frames=[]
for label,program,options in [
 ('direct','programs/dos/MANDVGA.COM',{}),
 ('DOS EXEC','images/FD14LITE.img::COMMAND.COM',dict(mount='driveD',args=r'/C D:\MANDVGA.COM'))]:
 s=Session(engine,program,stop_log='',**options)
 try:
  s.send('clock 0');start=time.perf_counter()
  for _ in range(180):
   r=run_slice(s,1_000_000)
   if r['status']!='running':break
  seconds=time.perf_counter()-start;r=refresh_frame(s,r)
  frames.append(r)
  print(label,round(seconds,4),'status',r['status'],r['detail'],'native',r.get('native_block_instructions'),'blocks',r.get('native_blocks'),'steps',r['steps'],'mode',r.get('planar_mode'),flush=True)
  if r['status']=='waiting_input':
   s.send('key 13 28');r=s.send('run 1000000');print('exit',r['status'],r['detail'],flush=True)
 finally:s.close()

for key in ['graphics_planar','planar_rgb','planar_width','planar_height']:
 assert frames[0][key]==frames[1][key],key
print('PASS direct and DOS EXEC VGA snapshot match')
