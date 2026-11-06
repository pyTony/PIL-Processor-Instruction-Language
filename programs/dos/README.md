# DOS demo programs

Original programs for trying Dine from the FreeCOM prompt. FACTOR and MANDEL
use 8086 instructions and DOS output only (AH02/AH09), and end with AH4C.

| Program | What it does |
|---|---|
| `FACTOR.COM [n]` | Prints 0! to n! (n up to 99, default 25) as exact decimal numbers. |
| `MANDCYC.COM` | Draws a 320×200 mode 13h Mandelbrot in 2×2 blocks and cycles the VGA DAC palette. Press any key to return to DOS. Requires a 386 and the mode 13h palette implementation. |
| `MANDEL.COM` | Draws the Mandelbrot set in 79x24 text characters, using 8.8 fixed point. |
| `TIMEIT.COM command [arguments]` | Runs the command through `COMMAND.COM /C` (from COMSPEC), then prints the command with its start time, end time and elapsed time from the DOS clock (INT 21h AH=2Ch, 1/18.2 s steps), with the country's separators (`14.27.05,32` for 358). Ends with the command's exit code, so `IF ERRORLEVEL` still works. |
| `MANDBBC.COM` | Draws the classic BBC BASIC benchmark Mandelbrot (79x25, 16 iterations, shades `.,'~=+:;*%&$OXB#@`) with the same characters as its floating-point version, using 32-bit integers with 20 fraction bits. Needs a 386. |

The `.COM` files are built from the `.asm` sources with NASM:

    nasm -f bin factor.asm -o FACTOR.COM
    nasm -f bin mandel.asm -o MANDEL.COM
    nasm -f bin mandbbc.asm -o MANDBBC.COM
    nasm -f bin timeit.asm -o TIMEIT.COM

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
