.intel_syntax noprefix
.code16
.global _start
_start:
    mov ax, 0x0013
    int 0x10
    mov ax, 0xa000
    mov es, ax
    xor di, di
    mov word ptr [cy], -1200
    mov word ptr [rows], 100
row_loop:
    mov word ptr [cxval], -2048
    mov word ptr [cols], 160
pixel_loop:
    xor si, si
    xor bp, bp
    mov cx, 24
iterate:
    mov ax, si
    imul si
    shrd ax, dx, 10
    mov bx, ax
    mov ax, bp
    imul bp
    shrd ax, dx, 10
    mov dx, bx
    add dx, ax
    cmp dx, 4096
    jae escaped
    sub bx, ax
    mov ax, si
    imul bp
    shrd ax, dx, 10
    add ax, ax
    add ax, word ptr [cy]
    mov bp, ax
    add bx, word ptr [cxval]
    mov si, bx
    loop iterate
    xor ax, ax
    jmp draw
escaped:
    mov ax, 25
    sub ax, cx
draw:
    mov ah, al
    mov word ptr es:[di], ax
    mov word ptr es:[di+320], ax
    add di, 2
    add word ptr [cxval], 20
    dec word ptr [cols]
    jnz pixel_loop
    add di, 320
    add word ptr [cy], 24
    dec word ptr [rows]
    jnz row_loop
    mov byte ptr [phase], 0
cycle:
    mov dx, 0x3c8
    mov al, 1
    out dx, al
    inc dx
    mov bl, 1
colors:
    mov al, bl
    add al, byte ptr [phase]
    and al, 63
    out dx, al
    add al, 21
    and al, 63
    out dx, al
    add al, 21
    and al, 63
    out dx, al
    inc bl
    cmp bl, 25
    jb colors
    inc byte ptr [phase]
    mov cx, 5000
delay:
    loop delay
    mov ah, 1
    int 0x16
    jz cycle
    xor ah, ah
    int 0x16
    mov ax, 3
    int 0x10
    mov ax, 0x4c00
    int 0x21
cxval: .word 0
cy: .word 0
rows: .word 0
cols: .word 0
phase: .byte 0
