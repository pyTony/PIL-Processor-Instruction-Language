# Build svgalib 1.1 (static libvga.a), vgatest and vgamand on MCC 1.0.
set -e
mkdir -p /usr/src /usr/local/lib /usr/local/bin
cd /usr/src
gunzip < /tmp/svga.tgz | cpio -imd -H ustar
cd svgalib/src
cp vga.h mouse/vgamouse.h keyboard/vgakeyboard.h /usr/include
make libvga.a
cp libvga.a /usr/lib
echo "chipset VGA" > /usr/local/lib/libvga.config
cd ../demos
gcc -O2 -o /usr/local/bin/vgatest vgatest.c -lvga
cd /tmp
gcc -O2 -o vgamand vgamand.c -lvga
ls -l /usr/lib/libvga.a /usr/local/bin/vgatest /tmp/vgamand
