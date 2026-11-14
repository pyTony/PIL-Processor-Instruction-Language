; Original recursive Fibonacci: two recursive calls, saved BX/CX.
; 8086, DOS AH02/AH09, 16-bit stack. F(20)=6765.
bits 16
org 100h
    mov si,81h
    xor bx,bx
    xor cx,cx
skip:
    lodsb
    cmp al,32
    je short skip
number:
    sub al,'0'
    cmp al,9
    ja short parsed
    xchg ax,bx
    mov ah,10
    mul ah
    add bx,ax
    inc cx
    cmp cx,2
    ja short usage
    lodsb
    jmp short number
parsed:
    add al,'0'
    cmp al,13
    jne short usage
    or cx,cx
    jnz short have
    mov bx,12
have:
    cmp bx,20
    ja short usage
    mov [limit],bx
    xor bp,bp
line:
    mov ax,bp
    call fib
    mov di,ax
    mov ax,bp
    call decimal
    mov dx,equals
    mov ah,9
    int 21h
    mov ax,di
    call decimal
    mov dx,crlf
    mov ah,9
    int 21h
    cmp bp,[limit]
    je short done
    inc bp
    jmp short line
done:
    mov ax,4c00h
    int 21h
usage:
    mov dx,help
    mov ah,9
    int 21h
    mov ax,4c01h
    int 21h
fib:
    cmp ax,1
    jbe short base
    push bx
    push cx
    mov bx,ax
    dec ax
    call fib
    mov cx,ax
    mov ax,bx
    sub ax,2
    call fib
    add ax,cx
    pop cx
    pop bx
base:
    ret
decimal:
    push ax
    push bx
    push cx
    push dx
    xor cx,cx
    mov bx,10
digit:
    xor dx,dx
    div bx
    push dx
    inc cx
    or ax,ax
    jnz short digit
print:
    pop dx
    add dl,'0'
    mov ah,2
    int 21h
    loop print
    pop dx
    pop cx
    pop bx
    pop ax
    ret
equals:
    db 32,61,32,36
crlf:
    db 13,10,36
help:
    db 85,115,97,103,101,58,32,82,69,67,85,82,70,73,66,32,91,110,93,32,32,32,112,114,105,110,116,115,32,70,40,48,41,46,46,70,40,110,41,44,32,110,32,60,61,32,50,48,32,40,100,101,102,97,117,108,116,32,49,50,41,13,10,36
limit:
    db 0,0
