; PILdos experimental INT 33h FAR callback smoke test.
; NASM: nasm -f bin programs/dos/MOUSECALL.ASM -o programs/dos/MOUSECALL.COM
; Run:  python tools/dos_terminal.py --mouse --engine target/release/pil programs/dos/MOUSECALL.COM
bits 16
org 100h

start:
    push cs
    pop ds
    push cs
    pop es
    xor ax,ax
    int 33h
    cmp ax,0ffffh
    jne no_mouse

    mov ax,000ch
    mov cx,007fh              ; movement + all button transitions
    mov dx,callback
    int 33h
    mov ax,1
    int 33h

    mov ah,09h
    mov dx,message
    int 21h

wait_for_click:
    cmp word [clicks],0
    je wait_for_click

    ; Disable callback before returning the memory to DOS.
    mov ax,000ch
    xor cx,cx
    int 33h
    mov ah,09h
    mov dx,done
    int 21h
    mov ax,4c00h
    int 21h

no_mouse:
    mov ah,09h
    mov dx,missing
    int 21h
    mov ax,4c01h
    int 21h

; Called by the mouse driver via FAR CALL with DS pointing at driver data.
; Access variables through CS, preserve only our own code's invariants,
; and return with RETF rather than IRET.
callback:
    test ax,0002h             ; left-button down event
    jz .return
    inc word [cs:clicks]
.return:
    retf

clicks dw 0
message db 'Mouse callback registered. Left-click to exit.',13,10,'$'
done db 'Callback returned; program exited cleanly.',13,10,'$'
missing db 'No DOS mouse driver installed.',13,10,'$'
