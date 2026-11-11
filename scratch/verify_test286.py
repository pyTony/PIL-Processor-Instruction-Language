import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from dos_terminal import Session, screen_rows

engine = Path('target/landmark-fix/release/pil.exe').resolve()
def execute(args, limit=200):
    session = Session(engine, 'images/FD14LITE.img::COMMAND.COM', mount='driveD', args='/C ' + args, stop_log='')
    try:
        for _ in range(limit):
            reply = session.send('run 1000000 buffered')
            if reply['status'] != 'running': break
        reply = session.send('snapshot')
        print(reply['status'], reply['detail'], 'steps', reply['steps'], flush=True)
        print('\n'.join(t.rstrip() for t, _ in screen_rows(reply)).rstrip(), flush=True)
        return reply
    finally:
        session.close()

if __name__ == '__main__':
    execute(r'D:\TP\BIN\TPC.EXE /UD:\TP\BIN D:\TP\EXAMPLES\TEST286.PAS')
    exe = Path('driveD/TP/EXAMPLES/TEST286.EXE')
    if exe.exists(): execute(r'D:\TP\EXAMPLES\TEST286.EXE')
