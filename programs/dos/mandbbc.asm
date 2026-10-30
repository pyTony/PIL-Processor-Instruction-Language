; MANDBBC.COM - the Mandelbrot set as the classic BBC BASIC benchmark draws
; it: X from -49 to 29 (C = X*2.29/50), Y from -12 to 12 (D = Y*4.16/50),
; z starting at c, escape when |z|^2 >= 5, at most 16 iterations, and the
; shade ".,'~=+:;*%&$OXB#@ " by iteration count. It uses 32-bit integers
; with 20 fraction bits (386 instructions in real mode), which gives the
; same characters as the floating-point program.
        cpu 386
        org 100h
FRAC    equ 20
        mov word [y],-12
.row:   mov word [x],-49
.col:   movsx eax,word [x]
        imul eax,eax,229
        call scale              ; c = X*229/5000 in 12.20
        mov [c],eax
        movsx eax,word [y]
        imul eax,eax,416
        call scale
        mov [d],eax
        mov esi,[c]             ; a = c
        mov edi,eax             ; b = d
        xor cx,cx               ; iteration count
.iter:  mov eax,esi
        imul esi
        shrd eax,edx,FRAC       ; a^2
        mov ebx,eax
        mov eax,edi
        imul edi
        shrd eax,edx,FRAC       ; b^2
        sub ebx,eax
        add ebx,[c]             ; t = a^2 - b^2 + c
        mov eax,esi
        imul edi
        shrd eax,edx,FRAC
        add eax,eax
        add eax,[d]
        mov edi,eax             ; b = 2ab + d
        mov esi,ebx             ; a = t
        mov eax,esi
        imul esi
        shrd eax,edx,FRAC
        mov ebx,eax
        mov eax,edi
        imul edi
        shrd eax,edx,FRAC
        add eax,ebx
        cmp eax,5<<FRAC
        jge .out
        inc cx
        cmp cx,16
        jb .iter
.out:   mov bx,cx
        cmp bx,16               ; never left: the space after '@'
        jb .shade
        inc bx
.shade: mov dl,[shade+bx]
        mov ah,2
        int 21h
        inc word [x]
        cmp word [x],29
        jle .col
        mov dx,crlf
        mov ah,9
        int 21h
        inc word [y]
        cmp word [y],12
        jle .row
        mov ax,4c00h
        int 21h
; eax * 2^20 / 5000, truncated toward zero
scale:  cdq
        shld edx,eax,FRAC
        shl eax,FRAC
        mov ebx,5000
        idiv ebx
        ret
shade   db ".,'~=+:;*%&$OXB#@ "
crlf    db 13,10,'$'
x       dw 0
y       dw 0
c       dd 0
d       dd 0
