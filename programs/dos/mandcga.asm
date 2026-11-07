; MANDCGA.COM - the Mandelbrot set in CGA mode 4 (320x200, four colours),
; using 4.12 fixed point and 8086 instructions only. Each pixel is written
; straight into the interleaved B800h framebuffer: even scanlines at 0000h,
; odd ones at 2000h, 80 bytes a line, four pixels a byte (left pixel in the
; top two bits). Points that stay inside after 32 iterations are colour 0;
; the others take colour 1 + (iterations mod 3). The caption is printed
; through DOS, so it is drawn with the BIOS graphics-mode font.
; Keys: P switches CGA palette 1/0, any other key returns to text mode.
        cpu 8086
        org 100h
MAXIT   equ 32
ONE     equ 4096                ; 1.0 in 4.12
        cld
        mov ax,0004h            ; 320x200 four colours, palette 1
        int 10h
        mov ax,0b800h
        mov es,ax
        mov word [cy],-(5*ONE/4) ; y from -1.25, step 51/4096 (2.5/200)
        mov word [y],0
.row:   mov ax,[y]
        mov bx,ax
        and bx,1
        mov cl,13
        shl bx,cl               ; odd lines in the 2000h bank
        shr ax,1
        mov dx,80
        mul dx
        add ax,bx
        mov [vptr],ax
        mov word [cx_],-(9*ONE/4); x from -2.25, step 42/4096 (3.25/320)
        mov word [xb],80
.byte:  mov byte [acc],0
        mov byte [px],4
.pixel: call point
        shl byte [acc],1
        shl byte [acc],1
        or [acc],al
        add word [cx_],42
        dec byte [px]
        jnz .pixel
        mov al,[acc]
        mov di,[vptr]
        stosb
        mov [vptr],di
        dec word [xb]
        jnz .byte
        add word [cy],51
        inc word [y]
        cmp word [y],200
        jb .row
        mov ah,2                ; cursor to row 24, column 0
        xor bh,bh
        mov dx,1800h
        int 10h
        mov dx,caption
        mov ah,9
        int 21h
.key:   xor ah,ah
        int 16h
        or al,20h
        cmp al,'p'
        jne .exit
        xor byte [pal],1
        mov ah,0bh
        mov bh,1
        mov bl,[pal]
        int 10h
        jmp .key
.exit:  mov ax,0003h
        int 10h
        mov ax,4c00h
        int 21h

; Colour of the point (cx_, cy) in al. zr in si, zi in di, count in bp.
point:  xor si,si
        xor di,di
        mov bp,MAXIT
.it:    cmp si,2*ONE            ; |zr| or |zi| >= 2 has left already, and
        jge .out                ; keeps the squares below 4.0 (16 bits)
        cmp si,-2*ONE
        jle .out
        cmp di,2*ONE
        jge .out
        cmp di,-2*ONE
        jle .out
        mov ax,si
        imul si
        call fix
        mov bx,ax               ; zr^2
        mov ax,di
        imul di
        call fix                ; zi^2
        mov cx,bx
        add cx,ax
        cmp cx,4*ONE
        jg .out
        sub bx,ax
        add bx,[cx_]            ; zr^2 - zi^2 + x
        mov ax,si
        imul di
        call fix
        shl ax,1
        add ax,[cy]             ; 2 zr zi + y
        mov di,ax
        mov si,bx
        dec bp
        jnz .it
        xor al,al
        ret
.out:   mov ax,MAXIT
        sub ax,bp               ; iterations completed
        mov bl,3
        div bl
        mov al,ah
        inc al
        ret
; dx:ax >> 12 into ax
fix:    mov cl,12
        shr ax,cl
        mov cl,4
        shl dx,cl
        or ax,dx
        ret

caption db 'CGA Mandelbrot  P=palette  other=exit$'
pal     db 1
cy      dw 0
cx_     dw 0
y       dw 0
vptr    dw 0
xb      dw 0
acc     db 0
px      db 0
