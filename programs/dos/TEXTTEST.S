/* DOS VGA text diagnostic, assembled as a real-mode DOS .COM program.
   as --32 TEXTTEST.S -o TEXTTEST.o
   objcopy -O binary -j .text TEXTTEST.o padded.bin
   Python: Path("TEXTTEST.COM").write_bytes(Path("padded.bin").read_bytes()[256:])
*/
.code16
.intel_syntax noprefix
.section .text
.org 0x100
start:
    push cs
    pop ds
    mov byte ptr [video_mode], 2
    mov byte ptr [committed], 0
    call change_mode
main_loop:
    xor ah, ah
    int 0x16
    cmp al, 27
    je leave
    cmp al, ' '
    je swap_mode
    cmp al, 13
    jne main_loop
    xor byte ptr [committed], 1
    call draw
    jmp main_loop
swap_mode:
    xor byte ptr [video_mode], 1
    call change_mode
    jmp main_loop
leave:
    mov ax, 3
    int 0x10
    mov ax, 0x4c00
    int 0x21
change_mode:
    xor ah, ah
    mov al, [video_mode]
    int 0x10
    call draw
    ret

/* ASCII string DS:SI -> text memory ES:DI, attribute AH. */
print:
    lodsb
    test al, al
    jz print_done
    stosw
    jmp print
print_done:
    ret

draw:
    mov ax, 0xb800
    mov es, ax
    xor di, di
    mov ax, 0x0720
    mov cx, 2000
    rep stosw
    mov ah, 0x1f
    mov si, offset header
    mov di, 0
    call print
    mov ah, 0x07
    mov si, offset instructions
    mov di, 160
    call print
    mov si, offset fg_label
    mov di, 3*160
    call print
    mov si, offset bg_label
    mov di, 6*160
    call print
    mov si, offset attrs_label
    mov di, 9*160
    call print
    mov si, offset invisible_label
    mov di, 12*160
    call print
    mov si, offset after_label
    mov di, 15*160
    call print

    /* 16 foreground colours on grey, two glyphs per sample. */
    mov di, 4*160+8*2
    xor bx, bx
fg_loop:
    mov ah, bl
    or ah, 0x70
    mov al, '#'
    stosw
    stosw
    mov ax, 0x0720
    stosw
    inc bl
    cmp bl, 16
    jb fg_loop
    /* Backgrounds 0..7 with white foreground. */
    mov di, 7*160+8*2
    xor bx, bx
bg_loop:
    mov ah, bl
    shl ah, 4
    or ah, 0x0f
    mov al, 'X'
    stosw
    mov al, ' '
    stosw
    inc bl
    cmp bl, 8
    jb bg_loop

    /* Character+attribute pairs 00,07,0F,70,87. */
    mov di, 10*160+8*2
    mov ax, 0x0041
    stosw
    mov ax, 0x0742
    stosw
    mov ax, 0x0f43
    stosw
    mov ax, 0x7044
    stosw
    mov ax, 0x8745
    stosw
    mov ah, 0x07
    mov si, offset legend
    mov di, 11*160
    call print

    /* Black-on-black text is intentionally invisible. */
    mov di, 13*160+8*2
    mov cx, 5
    mov si, offset value
invisible_loop:
    lodsb
    xor ah, ah
    stosw
    loop invisible_loop

    /* Spreadsheet A1 analogue: attr 70h before Enter, 07h afterwards. */
    mov di, 16*160+8*2
    mov si, offset value
    mov ah, 0x70
    cmp byte ptr [committed], 0
    je draw_value
    mov ah, 0x07
draw_value:
    call print
    mov ah, 0x07
    mov di, 18*160
    cmp byte ptr [committed], 0
    je pre_enter
    mov si, offset state_after
    jmp print_state
pre_enter:
    mov si, offset state_before
print_state:
    call print
    mov ah, 0x07
    mov di, 20*160
    mov si, offset mode_label
    call print
    mov al, [video_mode]
    add al, '0'
    mov ah, 0x0f
    stosw
    ret

video_mode: .byte 2
committed: .byte 0
header: .asciz "PIL DOS TEXT ATTRIBUTE DIAGNOSTIC"
instructions: .asciz "ENTER: commit A1 / toggle highlight     SPACE: mode 02h / 03h     ESC: exit"
fg_label: .asciz "Foreground 0..15 on gray:"
bg_label: .asciz "Background 0..7 (white X):"
attrs_label: .asciz "Attrs: 00 07 0F 70 87"
legend: .asciz "       A  B  C  D  E  (A deliberately invisible)"
invisible_label: .asciz "Hidden test 12345 (attr 00; should stay invisible):"
after_label: .asciz "Cell A1 test (12345 should remain visible after Enter):"
value: .asciz "12345"
state_before: .asciz "BEFORE Enter: A1 uses reverse-video attribute 70h"
state_after: .asciz "AFTER Enter:  A1 uses normal attribute 07h"
mode_label: .asciz "BIOS text mode: 0"
