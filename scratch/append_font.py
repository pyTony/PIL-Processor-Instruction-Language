p = 'src/dos_font8x8.txt'
t = open(p, encoding='utf-8', newline='').read()
h = open('scratch/font_high.txt', encoding='utf-8').read()
nl = '\r\n' if '\r\n' in t else '\n'
t = t.replace('\r\n', '\n')
old = """; Dine's original 8x8 ROM font for characters 00h-7Fh, copied to
; F000:FA6E when the BIOS area is built. Each glyph is its hex code and
; eight rows, top first; '#' is a set pixel (bit 7 is the left column).
; Drawn for this project, not taken from any BIOS or font file.
"""
new = """; Dine's original 8x8 ROM font. Characters 00h-7Fh are copied to F000:FA6E
; and 80h-FFh (code page 437 layout) to the video BIOS area, where INT 1Fh
; points, when the BIOS area is built. Each glyph is its hex code and eight
; rows, top first; '#' is a set pixel (bit 7 is the left column).
; Drawn for this project, not taken from any BIOS or font file. The accented
; letters reuse this file's own 00h-7Fh letters; box and block characters
; were computed from line positions.
"""
assert t.startswith(old)
t = new + t[len(old):]
if not t.endswith('\n'):
    t += '\n'
t += h
print('line ending', repr(nl))
open(p, 'w', encoding='utf-8', newline='').write(t.replace('\n', nl))
