; MANDEL.COM - the Mandelbrot set in text characters, 79x24, using 8.8 fixed
; point and 8086 instructions only. Each cell shows how many iterations
; (of at most 15) the point takes to leave the radius-2 circle.
        org 100h
SCALE   equ 256
        mov word [cy],-288      ; y from -1.125 upward
        mov byte [row],24
.row:   mov word [cx_],-512     ; x from -2.0
        mov byte [col],79
.col:   xor si,si               ; zr
        xor di,di               ; zi
        mov cx,15
.iter:  mov ax,si
        imul ax                 ; zr^2 in dx:ax
        call fix
        mov bx,ax               ; bx = zr^2
        mov ax,di
        imul ax
        call fix                ; ax = zi^2
        mov dx,bx
        add dx,ax
        cmp dx,4*SCALE
        jg .out
        sub bx,ax               ; zr^2 - zi^2
        mov ax,si
        imul di
        call fix
        shl ax,1                ; 2 zr zi
        add ax,[cy]
        mov di,ax
        add bx,[cx_]
        mov si,bx
        loop .iter
.out:   mov bx,cx               ; 0 = never left
        mov dl,[shade+bx]
        mov ah,2
        int 21h
        add word [cx_],10       ; 3.0 / 79 steps in 8.8
        dec byte [col]
        jnz .col
        mov dx,crlf
        mov ah,9
        int 21h
        add word [cy],25        ; 2.34 / 24 steps
        dec byte [row]
        jnz .row
        mov ax,4c00h
        int 21h
; dx:ax >> 8 into ax
fix:    mov al,ah
        mov ah,dl
        ret
shade   db '#@@%%**++==--::. '
crlf    db 13,10,'$'
cy      dw 0
cx_     dw 0
row     db 0
col     db 0
