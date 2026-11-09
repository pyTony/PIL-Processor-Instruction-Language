; MANDVGA.COM - the Mandelbrot set in VGA mode 12h (640x480, 16 colours),
; using 4.12 fixed point and 8086 instructions only. It computes 320x240
; points and draws each as a 2x2 block through the graphics controller:
; write mode 2 (the CPU byte is the colour), the bit mask selecting the two
; pixels, and a read before each write to load the latches so the other six
; pixels of the byte keep their colours. Inside points are colour 0; the
; others take 1 + (iterations mod 15). The caption is printed through DOS
; in the 8x16 text cells of mode 12h. Any key returns to text mode.
        cpu 8086
        org 100h
MAXIT   equ 30
ONE     equ 4096
STEP    equ 42                  ; 3.25/320 in 4.12, the same on both axes
        cld
        mov ax,0012h
        int 10h
        mov ax,0a000h
        mov es,ax
        mov dx,3ceh
        mov ax,0205h            ; graphics mode register: write mode 2
        out dx,ax
        mov word [cy],-(120*STEP)
        mov word [line],0       ; offset of the first of the block's two lines
.row:   mov word [cx_],-(9*ONE/4)
        mov word [xp],0
.pt:    call point
        mov [col],al
        mov bx,[xp]
        mov cl,bl
        and cl,3
        shl cl,1
        mov ah,0c0h
        shr ah,cl               ; the block's two pixels in this byte
        mov al,8                ; bit mask register
        mov dx,3ceh
        out dx,ax
        mov di,[line]
        shr bx,1
        shr bx,1
        add di,bx
        mov al,[es:di]          ; load the latches
        mov al,[col]
        mov [es:di],al
        mov ah,[es:di+80]
        mov [es:di+80],al
        add word [cx_],STEP
        inc word [xp]
        cmp word [xp],320
        jb .pt
        add word [cy],STEP
        add word [line],160
        cmp word [line],160*240
        jb .row
        mov dx,3ceh
        mov ax,0ff08h           ; bit mask FFh
        out dx,ax
        mov ax,0005h            ; write mode 0
        out dx,ax
        mov ah,2                ; cursor to row 29, column 0
        xor bh,bh
        mov dx,1d00h
        int 10h
        mov dx,caption
        mov ah,9
        int 21h
        xor ah,ah
        int 16h
        mov ax,0003h
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
        mov bl,15
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

caption db 'VGA 640x480 Mandelbrot  any key=exit$'
cy      dw 0
cx_     dw 0
xp      dw 0
line    dw 0
col     db 0
