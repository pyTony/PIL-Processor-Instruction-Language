import sys, time, re
sys.path.insert(0, "tools")
from dos_terminal import Session, screen_rows, paste_keys, key_command, local_offset

def get_screen_text(reply):
    rows = screen_rows(reply)
    return "\n".join(chars.rstrip() for chars, attrs in rows if chars.strip())

print("=" * 60)
print("BBC BASIC MANDELBROT BENCHMARK (PILdos headless)")
print("Reference (1.11.2026): 763.96 secs (Basic) / 764.02 s (Total)")
print("=" * 60)

engine = "target/release/pil.exe"
image = "images/FD14LITE.img::COMMAND.COM"
mount = "driveD"
args = r"/C D:\dosbasic\BBCBASIC\BBCBASIC.EXE"

print(f"Starting session: {engine} {image} --mount {mount} --args \"{args}\"")
s = Session(engine, image, mount=mount, args=args)
s.send(f"clock {local_offset()}")
s.send("country 358")

# Wait for BBCBASIC banner
for _ in range(5):
    reply = s.send("run 50000")

# Send CHAIN command
cmd_str = 'CHAIN "D:\\DOSBASIC\\BBCBASIC\\MAND.BBC"\r'
for cmd in map(key_command, paste_keys(cmd_str)):
    s.send(cmd)
    s.send("run 1000")

# Wait for "Start" to appear
t_wall_start = time.time()
total_steps = 0
started = False

for _ in range(20):
    reply = s.send("run 50000")
    total_steps += 50000
    text = get_screen_text(reply)
    if "Start" in text:
        started = True
        break

if not started:
    print("Warning: 'Start' text not yet observed, proceeding into main loop.")
else:
    print("Mandelbrot calculation started.")

t_bench_start = time.time()
slice_size = 1000000  # max allowed budget per run command
last_log_time = time.time()
found_time_line = None

try:
    while True:
        reply = s.send(f"run {slice_size}")
        total_steps += slice_size
        now = time.time()
        elapsed_bench = now - t_bench_start

        # Check screen for completion
        text = get_screen_text(reply)
        if "Time:" in text:
            t_bench_end = now
            for line in text.split("\n"):
                if "Time:" in line:
                    found_time_line = line.strip()
            break

        # Check if process died
        if reply.get("status") not in ("running", "waiting_input"):
            print(f"Unexpected status: {reply.get('status')} - detail: {reply.get('detail')}")
            break

        # Periodic progress log every ~15 seconds
        if now - last_log_time >= 15.0:
            rate = total_steps / (now - t_wall_start)
            print(f"[{elapsed_bench:6.1f}s] Steps: {total_steps/1e6:5.1f}M | Rate: {rate/1e3:4.0f} ksteps/s")
            last_log_time = now

except KeyboardInterrupt:
    print("\nBenchmark cancelled by user.")

t_wall_end = time.time()
wall_total = t_wall_end - t_wall_start
bench_total = t_bench_end - t_bench_start if found_time_line else None

print("\n" + "=" * 60)
print("BENCHMARK COMPLETED")
print("=" * 60)
print("Screen Output:")
print(get_screen_text(reply))
print("-" * 60)
if found_time_line:
    print(f"BBC BASIC Reported: {found_time_line}")
if bench_total:
    print(f"Bench Wall-clock : {bench_total:.2f} s")
print(f"Total Wall-clock : {wall_total:.2f} s")
print(f"Total Steps      : {total_steps} instructions ({total_steps / wall_total / 1e3:.1f} ksteps/s)")

if found_time_line:
    m = re.search(r"Time:\s*([\d\.]+)\s*secs", found_time_line)
    if m:
        current_secs = float(m.group(1))
        ref_secs = 763.96
        diff = current_secs - ref_secs
        pct = (diff / ref_secs) * 100
        print("-" * 60)
        print(f"Comparison vs 1.11.2026 Reference (763.96 s):")
        print(f"  Current   : {current_secs:.2f} s")
        print(f"  Reference : {ref_secs:.2f} s")
        print(f"  Difference: {diff:+.2f} s ({pct:+.1f}%)")

s.close()
