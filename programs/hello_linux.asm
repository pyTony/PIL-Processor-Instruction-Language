; hello32.asm
BITS 32
GLOBAL _start

SECTION .rodata
msg: db "hi from line", 10
msg_len: equ $ - msg

SECTION .text
_start:
    mov eax, 4          ; __NR_write
    mov ebx, 1          ; fd = stdout
    mov ecx, msg        ; buf
    mov edx, msg_len    ; count
    int 0x80

    mov eax, 1          ; __NR_exit
    xor ebx, ebx        ; status = 0
    int 0x80