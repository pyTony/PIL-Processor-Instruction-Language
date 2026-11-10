import json
import subprocess
import time

proc = subprocess.Popen(
    ["target/debug/pil", "--dos-session", "--mount", "driveD", "images/FD14LITE.img::COMMAND.COM"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True
)

def send(cmd):
    proc.stdin.write(cmd + "\n")
    proc.stdin.flush()
    line = proc.stdout.readline()
    return json.loads(line)

print("init:", send("step 0")["status"])

# Wait for prompt
while True:
    r = send("step 100000")
    if r["status"] == "waiting_input":
        print("Prompt reached at step", r["steps"])
        break

# type "D:\GWBASIC CUBE2.BAS\r"
cmd = "d:\\gwbasic cube2.bas\r"
for ch in cmd:
    code = ord(ch)
    r = send(f"key {code} 1")
    r = send(f"key {code} 0")

in_graphics = False
for step in range(100):
    r = send("step 200000")
    if r.get("cga_mode") == 6:
        print(f"Reached CGA mode 6 at step {r['steps']}")
        in_graphics = True
        break
    if r["status"] == "stopped":
        print("Stopped early:", r.get("detail"))
        break

if in_graphics:
    # Let it run a few frames
    for _ in range(5):
        r = send("step 200000")
    print("Sending break...")
    r = send("break")
    print("Break sent:", r["status"], r.get("detail"))

    for step in range(30):
        r = send("step 100000")
        print(f"Step {r['steps']}: {r['status']}, detail: {r.get('detail')}")
        if r["status"] == "stopped":
            print("STOPPED:", r.get("detail"))
            break
        if r["status"] == "waiting_input":
            print("Waiting input reached at", r["steps"])
            break

proc.terminate()
