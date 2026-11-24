# Linux programs

Sources to compile inside a booted Linux under PIL (docs/BOOT.md).

- mandel.c: the text Mandelbrot of programs/dos/MANDEL.COM (79x24, 8.8
  fixed point, 15 iterations, the same shade characters, ANSI colours),
  in C for gcc 2.5.8 on MCC Interim 1.0+. `mandel i N` draws it N times in
  fixed point (16-bit arithmetic, as the DOS program), `mandel f N` in
  double precision; both print the clock ticks from times() and a sum of
  the iteration counts (fixed 15564, float 15584), so a run can be checked
  against the host's own build of the same file.
- vgamand.c: a Mandelbrot through svgalib 1.1 (`vgamand [256|16]
  [reps]`): 320x200 in 256 colours or 640x480 in 16, 32-bit integers with
  12 fraction bits, 64 iterations, each point drawn with vga_drawpixel.
  After Enter it returns to text and prints the times() ticks and the sum
  of the iteration counts (1104757 for 320x200, 5294314 for 640x480).
