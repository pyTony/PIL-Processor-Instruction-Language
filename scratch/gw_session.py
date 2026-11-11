"""Drive a --dos-session: run COMMAND.COM /C GWBASIC prog, report modes and stops."""
import json, subprocess, sys, time
prog = sys.argv[1]
budget_runs = int(sys.argv[2]) if len(sys.argv) > 2 else 200
break_after = int(sys.argv[3]) if len(sys.argv) > 3 else 0
proc = subprocess.Popen(["target/release/pil", "--dos-session", "--mount", "driveD", "--args", f"/C D:\\GWBASIC D:\\{prog}",
                         "images/FD14LITE.img::COMMAND.COM"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, bufsize=1)
def read():
    return json.loads(proc.stdout.readline())
def send(cmd):
    proc.stdin.write(cmd + "\n"); proc.stdin.flush(); return read()
r = read()
send("clock 7200")
def screen_text(r):
    b = bytes.fromhex(r['screen']); cols = r['columns']
    return '\n'.join(bytes(b[i*cols*2:(i+1)*cols*2:2]).decode('cp437').rstrip() for i in range(25)).rstrip()
last = None; t0 = time.time(); keyed = False
for n in range(budget_runs):
    r = send("run 1000000")
    mode = ('planar', r.get('planar_mode')) if 'graphics_planar' in r else ('cga', r.get('cga_mode')) if 'graphics_cga' in r else ('vga13',) if 'graphics13' in r else ('text',)
    if mode != last:
        print(f"[{n}] steps={r['steps']} mode={mode} status={r['status']}"); last = mode
    if break_after and n == break_after and not keyed:
        print("sending key"); print(send("key 32 57")['text']); keyed = True
    if r['status'] not in ('running', 'waiting_input'):
        print("STOP", r['status'], r['detail'], hex(r['cs']), hex(r['eip']), r.get('instruction_bytes'), 'regs', [hex(x) for x in r.get('registers', [])]); break
print(f"elapsed {time.time()-t0:.1f}s steps={r['steps']}")
if 'graphics_planar' not in r:
    print(screen_text(r))
else:
    open('scratch/last_planar.json', 'w').write(json.dumps({k: r[k] for k in r if k.startswith('planar') or k == 'graphics_planar'}))
send("quit")
