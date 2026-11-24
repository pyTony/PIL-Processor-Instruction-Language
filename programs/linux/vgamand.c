/* Mandelbrot through svgalib 1.1 (May 1994), for Linux 1.0 and gcc 2.5.8.
   vgamand [256|16] [reps]: 320x200 with 256 colours (default) or 640x480
   with 16 colours. Each point is computed in 32-bit integers with 12
   fraction bits and drawn with vga_drawpixel. After the last picture it
   waits for Enter, returns to text mode and prints the clock ticks from
   times() and a sum of the iteration counts. */
#include <stdio.h>
#include <stdlib.h>
#include <sys/times.h>
#include <vga.h>

#define MAXIT 64

int main(int argc, char **argv)
{
    int mode = G320x200x256, reps = 1, w, h, x, y, r, n, colors;
    long sum = 0;
    struct tms t0, t1;
    clock_t c0, c1;

    if (argc > 1 && atoi(argv[1]) == 16)
        mode = G640x480x16;
    if (argc > 2)
        reps = atoi(argv[2]);
    if (reps < 1)
        reps = 1;
    vga_init();
    if (!vga_hasmode(mode)) {
        printf("vgamand: mode not available\n");
        return 1;
    }
    vga_setmode(mode);
    w = vga_getxdim();
    h = vga_getydim();
    colors = vga_getcolors();
    if (colors == 256)
        for (n = 0; n < MAXIT; n++)
            vga_setpalette(n + 1, n, (n * 2) & 63, 63 - n);
    c0 = times(&t0);
    for (r = 0; r < reps; r++) {
        sum = 0;
        for (y = 0; y < h; y++) {
            /* -1.25 .. 1.25 down, -2.25 .. 0.75 across, 12 fraction bits */
            int ci = -5120 + (y * 10240) / h;
            for (x = 0; x < w; x++) {
                int cr = -9216 + (x * 12288) / w;
                int zr = 0, zi = 0, zr2 = 0, zi2 = 0;
                for (n = 0; n < MAXIT && zr2 + zi2 < 16384; n++) {
                    zi = ((zr * zi) >> 11) + ci;
                    zr = zr2 - zi2 + cr;
                    zr2 = (zr * zr) >> 12;
                    zi2 = (zi * zi) >> 12;
                }
                sum += n;
                if (n == MAXIT)
                    vga_setcolor(0);
                else if (colors == 256)
                    vga_setcolor(n + 1);
                else
                    vga_setcolor(1 + n % 15);
                vga_drawpixel(x, y);
            }
        }
    }
    c1 = times(&t1);
    getchar();
    vga_setmode(TEXT);
    printf("%dx%dx%d x%d: %ld ticks user, %ld real, sum %ld\n", w, h, colors,
           reps, (long) (t1.tms_utime - t0.tms_utime), (long) (c1 - c0), sum);
    return 0;
}
