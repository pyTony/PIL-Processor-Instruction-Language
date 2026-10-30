# DOS demo programs

Original programs for trying Dine from the FreeCOM prompt. Both use 8086
instructions and DOS output only (AH02/AH09), and end with AH4C.

| Program | What it does |
|---|---|
| `FACTOR.COM [n]` | Prints 0! to n! (n up to 99, default 25) as exact decimal numbers. |
| `MANDEL.COM` | Draws the Mandelbrot set in 79x24 text characters, using 8.8 fixed point. |

The `.COM` files are built from the `.asm` sources with NASM:

    nasm -f bin factor.asm -o FACTOR.COM
    nasm -f bin mandel.asm -o MANDEL.COM

To run them, mount this folder as a drive:

    python tools/dos_terminal.py --mount programs/dos images/FD14LITE.img::COMMAND.COM

Then at the prompt type `D:`, `FACTOR 30` or `MANDEL`. The unit tests run both
files and check their output.
