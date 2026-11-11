from verify_test286 import Session, screen_rows, engine
s=Session(engine, 'images/FD14LITE.img::COMMAND.COM', mount='driveD', args='/K', stop_log='')
def advance(n):
    for _ in range(n):
        r=s.send('run 1000000 buffered')
        if r['status']!='running': break
    r=s.send('snapshot')
    print(r['status'],r['detail'],r['steps'], 'CGA',r.get('cga_mode_control'),flush=True)
    print('\n'.join(t.rstrip() for t,_ in screen_rows(r)).rstrip(),flush=True)
    return r
try:
    advance(1)
    for command in ['D:', r'CD \TP\EXAMPLES', r'D:\TP\BIN\TURBO.EXE TEST286.PAS']:
        for byte in (command+'\r').encode('ascii'):
            s.send(f'key {byte} {28 if byte==13 else 0}')
            s.send('run 10000 buffered')
        advance(3)
    advance(10)
    print('CTRL-F9',s.send('key 0 102')['ok'],flush=True)
    advance(20)
    print('ALT-X',s.send('key 0 45')['ok'],flush=True)
    advance(10)
finally: s.close()
