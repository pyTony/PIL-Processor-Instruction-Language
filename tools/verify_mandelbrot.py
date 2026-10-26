"""Independent integer/floor-division Mandelbrot raster oracle; no PIL imports."""
from pathlib import Path
import sys,hashlib

def reference(size,limit):
    pixels=bytearray()
    for row in range(size):
        for col in range(size):
            cr=-8192+col*12288//size;ci=-6144+row*12288//size
            real=imag=0
            for count in range(limit):
                if real*real+imag*imag>67108864:
                    pixels.append(count*4)
                    break
                real,imag=(real*real-imag*imag)//4096+cr,real*imag//2048+ci
            else:pixels.append(0)
    return bytes(pixels)

if __name__=='__main__':
    if len(sys.argv)!=4:raise SystemExit('usage: verify_mandelbrot.py FILE.pgm SIZE ITERATIONS')
    path=Path(sys.argv[1]);size=int(sys.argv[2]);limit=int(sys.argv[3]);data=path.read_bytes()
    assert data.startswith(b'P5\n')
    parts=data.split(b'\n',4);assert parts[1].startswith(b'#') and parts[2]==f'{size} {size}'.encode() and parts[3]==b'255'
    actual=parts[4];want=reference(size,limit);assert actual==want,'Mandelbrot raster differs from independent integer oracle'
    bmp=path.with_suffix('.bmp').read_bytes();stride=(size*3+3)&~3
    assert bmp[:2]==b'BM' and int.from_bytes(bmp[2:6],'little')==len(bmp)==54+stride*size
    assert int.from_bytes(bmp[18:22],'little')==size and int.from_bytes(bmp[22:26],'little')==size
    assert bmp[26:30]==b'\x01\x00\x18\x00' and int.from_bytes(bmp[10:14],'little')==54
    for row in range(size):
        for col in range(size):
            at=54+(size-1-row)*stride+col*3;raw=want[row*size+col];value=255-raw if raw else 0;assert bmp[at:at+3]==bytes([value]*3)
    print(f'PASS: independent Python verified {size}x{size} PGM and BMP; SHA256={hashlib.sha256(actual).hexdigest()}')
