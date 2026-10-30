; FACTOR.COM - prints n! for n = 0..N in exact decimal (N from the command
; line, 0..99, default 25). The number is kept as decimal digits, least
; significant first, and multiplied digit by digit.
        org 100h
        mov si,81h
        xor bx,bx               ; N
        xor cx,cx               ; digits seen
.skip:  lodsb
        cmp al,' '
        je .skip
.num:   sub al,'0'
        cmp al,9
        ja .parsed
        xchg ax,bx              ; bx = new digit, ax = N so far
        mov ah,10
        mul ah                  ; ax = N*10 (N <= 99 keeps it in a byte)
        add bx,ax
        inc cx
        cmp cx,2
        ja usage
        lodsb
        jmp .num
.parsed:
        add al,'0'
        cmp al,13
        jne usage
        or cx,cx
        jnz .have
        mov bx,25
.have:  mov [limit],bx
        mov byte [digits],1     ; 0! = 1
        mov word [length],1
        xor bp,bp               ; n
.line:  mov ax,bp
        call print_n
        mov dx,eq
        mov ah,9
        int 21h
        call print_digits
        cmp bp,[limit]
        je .end
        inc bp
        ; digits *= n
        mov cx,[length]
        mov si,digits
        xor bx,bx               ; carry
.mul:   mov al,[si]
        xor ah,ah
        mul bp                  ; dx:ax = digit*n (< 1000)
        add ax,bx
        mov bl,10
        div bl                  ; al = carry, ah = digit
        mov [si],ah
        mov bl,al
        xor bh,bh
        inc si
        loop .mul
.carry: or bx,bx
        jz .line
        mov ax,bx
        mov bl,10
        div bl
        mov [si],ah
        inc si
        inc word [length]
        mov bl,al
        xor bh,bh
        jmp .carry
.end:   mov ax,4c00h
        int 21h
usage:  mov dx,help
        mov ah,9
        int 21h
        mov ax,4c01h
        int 21h

; prints AX (0..99) right-aligned in two columns
print_n:
        mov bl,10
        div bl
        mov bh,ah
        mov dl,' '
        or al,al
        jz .one
        mov dl,al
        add dl,'0'
.one:   mov ah,2
        int 21h
        mov dl,bh
        add dl,'0'
        mov ah,2
        int 21h
        ret

print_digits:
        mov si,[length]
.next:  dec si
        mov dl,[digits+si]
        add dl,'0'
        mov ah,2
        int 21h
        or si,si
        jnz .next
        mov dx,crlf
        mov ah,9
        int 21h
        ret

eq      db '! = $'
crlf    db 13,10,'$'
help    db 'Usage: FACTOR [n]   prints 0! .. n! for n up to 99 (default 25)',13,10,'$'
limit   dw 0
length  dw 0
digits:
