# DOS demo programs

Original programs for trying Dine from the FreeCOM prompt. FACTOR and MANDEL
use 8086 instructions and DOS output only (AH02/AH09), and end with AH4C.

| Program | What it does |
|---|---|
| `FACTOR.COM [n]` | Prints 0! to n! (n up to 99, default 25) as exact decimal numbers. |
| `MANDEL.COM` | Draws the Mandelbrot set in 79x24 text characters, using 8.8 fixed point. |
| `TIMEIT.COM command [arguments]` | Runs the command through `COMMAND.COM /C` (from COMSPEC), then prints its start time, end time and elapsed time from the DOS clock (INT 21h AH=2Ch, 1/18.2 s steps), with the country's separators (`14.27.05,32` for 358). Ends with the command's exit code, so `IF ERRORLEVEL` still works. |
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
