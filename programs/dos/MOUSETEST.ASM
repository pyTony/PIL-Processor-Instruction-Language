; Experimental DOS INT 33h smoke test. NASM: nasm -f bin MOUSETEST.ASM -o MOUSETEST.COM
bits 16
org 100h
    xor ax, ax
    int 33h
    test ax, ax
    jz absent
    mov dx, message
    mov ah, 09h
    int 21h
    mov ax, 0001h
    int 33h
poll:
    mov ax, 0003h
    int 33h
    test bx, 1
    jz poll
    mov ax, 0002h
    int 33h
    mov ax, 4c00h
    int 21h
absent:
    mov dx, no_driver
    mov ah, 09h
    int 21h
    mov ax, 4c01h
    int 21h
message db 'Move mouse; left-click to exit.',13,10,'$'
no_driver db 'No DOS mouse driver. Start PILdos with --mouse.',13,10,'$'
