# DOS demo programs

Original programs for trying Dine from the FreeCOM prompt. FACTOR and MANDEL
use 8086 instructions and DOS output only (AH02/AH09), and end with AH4C.

| Program | What it does |
|---|---|
| `FACTOR.COM [n]` | Prints 0! to n! (n up to 99, default 25) as exact decimal numbers. |
| `RECURFIB.COM [n]` | Recursively computes F(0) through F(n), up to 20 (default 12). Two recursive CALLs per non-base case, with BX/CX saved on the guest stack. Also compiles to standalone Rust/native code. |
| `MZARGS.EXE [text]` | Original relocated MZ argument demo with distinct CS, DS and SS, nonzero entry IP and a near call. Built by `tools/build_mz_demo.py`; also compiles to a standalone native executable. |
| `MANDCYC.COM` | Draws a 320×200 mode 13h Mandelbrot in 2×2 blocks and cycles the VGA DAC palette. Press any key to return to DOS. Requires a 386 and the mode 13h palette implementation. |
| `MANDCGA.COM` | Draws a 320×200 four-colour Mandelbrot in CGA mode 4 (4.12 fixed point, 8086 only) and prints its caption in graphics mode with the BIOS font. `P` switches CGA palette 1/0; any other key returns to text mode and DOS. |
| `MANDVGA.COM` | Draws a 640×480 16-colour Mandelbrot in VGA mode 12h as 320×240 points in 2×2 blocks, through the graphics controller (write mode 2, bit mask, latch reads), 8086 only, and prints its caption in the mode's 8×16 text cells. Any key returns to text mode and DOS. |
| `MANDEL.COM` | Draws the Mandelbrot set in 79x24 text characters, using 8.8 fixed point. |
| `MANDANSI.COM` | Draws the original 79x24 fixed-point Mandelbrot with ANSI colours. Requires a 386 and ANSI terminal (or ANSI.SYS in DOS). Also compiles to a standalone native executable through PIL. |
| `TIMEIT.COM command [arguments]` | Runs the command through `COMMAND.COM /C` (from COMSPEC), then prints the command with its start time, end time and elapsed time from the DOS clock (INT 21h AH=2Ch, 1/18.2 s steps), with the country's separators (`14.27.05,32` for 358). Ends with the command's exit code, so `IF ERRORLEVEL` still works. |
| `MANDBBC.COM` | Draws the classic BBC BASIC benchmark Mandelbrot (79x25, 16 iterations, shades `.,'~=+:;*%&$OXB#@`) with the same characters as its floating-point version, using 32-bit integers with 20 fraction bits. Needs a 386. |

The `.COM` files are built from the `.asm` sources with NASM:

    nasm -f bin factor.asm -o FACTOR.COM
    nasm -f bin recurfib.asm -o RECURFIB.COM
    nasm -f bin mandel.asm -o MANDEL.COM
    nasm -f bin mandansi.asm -o MANDANSI.COM
    nasm -f bin mandbbc.asm -o MANDBBC.COM
    nasm -f bin mandcga.asm -o MANDCGA.COM
    nasm -f bin mandvga.asm -o MANDVGA.COM
    nasm -f bin timeit.asm -o TIMEIT.COM

`python tools/build_ansi_mandel.py` rebuilds MANDANSI.COM and its NASM source
without an assembler dependency. `--check` verifies both assets.
`python tools/verify_ansi_mandel.py` compares all 1896 coloured cells against
an independent Python reference and Dine, after building standalone Rust.

To run them, mount this folder as a drive:

    python tools/dos_terminal.py --mount programs/dos images/FD14LITE.img::COMMAND.COM

Then at the prompt type `D:`, `FACTOR 30`, `MANDEL` or `TIMEIT MANDEL`. The unit tests run all four
files and check their output.

Run the graphics demo directly with:

    python tools/dos_terminal.py programs/dos/MANDCYC.COM

The first frame takes time to calculate. A key exits after drawing and restores
text mode. The COM file is built from GNU assembler source:

    as --32 programs/dos/MANDCYC.s -o MANDCYC.o
    ld -m elf_i386 -Ttext 0x100 --oformat binary -o programs/dos/MANDCYC.COM MANDCYC.o

Check the frame, palette cycling and keyboard exit with a release build:

    python tools/verify_mandel_cycle.py


`CUBE2.BAS` adapts the user-supplied BASIC cube for CGA SCREEN 2 (640x200,
one-bit pixels). It omits the SCREEN 9 COLOR statement, uses LINE colour 1,
and adjusts the vertical centre and field of view. The BASIC interpreter is
not included. GW-BASIC drew part of its wireframe in both deterministic- and
host-clock tests. Graphics-mode text and scrolling are not fully implemented,
and this is not yet a complete GW-BASIC compatibility claim.

`CUBE2.BAS` is stored with DOS CRLF line endings because GW-BASIC cannot
load the LF-only file. Copy it without converting line endings, then run
`GWBASIC CUBE2.BAS` from the directory containing it.

Run the CGA demo from the prompt (`D:` then `MANDCGA`) or directly with:

    python tools/dos_terminal.py programs/dos/MANDCGA.COM

Check every pixel, the caption, the palette key and the exit with a release
build:

    python tools/verify_mandel_cga.py

The VGA demo runs the same way (`MANDVGA` at the prompt, or
`python tools/dos_terminal.py programs/dos/MANDVGA.COM`), and
`python tools/verify_mandel_vga.py` checks its four planes pixel by pixel.
