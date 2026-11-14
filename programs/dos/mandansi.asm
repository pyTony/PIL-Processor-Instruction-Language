; Original 79x24 ANSI Mandelbrot, signed 8.8 fixed point.
; Based on this project's original mandel.asm arithmetic.
; Requires a 386 and an ANSI terminal (ANSI.SYS on DOS).
; Build: nasm -f bin mandansi.asm -o MANDANSI.COM
bits 16
org 100h
    mov dx,header
    mov ah,9
    int 21h
    mov word [cy],-288
    mov byte [row],24
next_row:
    mov word [cx_],-512
    mov byte [col],79
next_col:
    xor si,si
    xor di,di
    mov cx,15
iterate:
    mov ax,si
    imul ax
    call fix
    mov bx,ax
    mov ax,di
    imul ax
    call fix
    mov dx,bx
    add dx,ax
    cmp dx,1024
    jg near pixel
    sub bx,ax
    mov ax,si
    imul di
    call fix
    shl ax,1
    add ax,[cy]
    mov di,ax
    add bx,[cx_]
    mov si,bx
    loop iterate
pixel:
    mov bx,cx
    shl bx,1
    add bx,cx
    shl bx,1
    add bx,cx
    mov dx,palette
    add dx,bx
    mov ah,9
    int 21h
    add word [cx_],10
    dec byte [col]
    jnz near next_col
    mov dx,crlf
    mov ah,9
    int 21h
    add word [cy],25
    dec byte [row]
    jnz near next_row
    mov dx,reset
    mov ah,9
    int 21h
    mov ax,4c00h
    int 21h
fix:
    mov al,ah
    mov ah,dl
    ret
header:
    db 27,91,48,59,57,55,59,52,48,109,27,91,50,74,27,91,72,80,73,76,32,110,97,116,105,118,101,32,65,78,83,73,32,77,97,110,100,101,108,98,114,111,116,13,10,36
crlf:
    db 13,10,36
reset:
    db 27,91,48,109,13,10,36
palette:
    db 27,91,51,48,109,32,36,27,91,57,54,109,64,36,27,91,57,54,109,64,36,27,91,57,52,109,37,36,27,91,57,52,109,37,36,27,91,57,53,109,42,36,27,91,57,53,109,42,36,27,91,57,49,109,43,36,27,91,57,49,109,43,36,27,91,57,51,109,61,36,27,91,57,51,109,61,36,27,91,57,50,109,45,36,27,91,57,50,109,45,36,27,91,51,54,109,58,36,27,91,51,52,109,58,36,27,91,51,48,109,46,36
cy:
    db 0,0
cx_:
    db 0,0
row:
    db 0
col:
    db 0
