; TIMEIT.COM - runs a command and prints its start time, end time and the
; time it took:  TIMEIT command [arguments]
; The summary repeats the command, so the four lines can be saved as a record.
; The command goes to COMMAND.COM /C (from COMSPEC), so programs on the PATH,
; batch files and internal commands such as DIR all work. Times come from
; INT 21h AH=2Ch, so they follow the DOS clock and its 1/18.2 s resolution.
; The separators follow the country (AH=38h): 14:27:05.32 for 001 and
; 14.27.05,32 for 358. TIMEIT ends with the command's exit code, so
; IF ERRORLEVEL sees the command's result. 8086 instructions only.
        org 100h
        mov sp,stack_top
        mov bx,end_of_program+15
        mov cl,4
        shr bx,cl               ; paragraphs from PSP:0 to the stack top
        mov ah,4ah              ; give the rest of memory back for the command
        int 21h
        jc no_memory

        ; The command tail, without leading blanks.
        mov si,81h
        mov cl,[80h]
        xor ch,ch
.blank: jcxz .none
        cmp byte [si],' '
        je .next
        cmp byte [si],9
        jne .tail
.next:  inc si
        dec cx
        jmp .blank
.none:  jmp usage
.tail:  cmp cx,126-4
        ja too_long
        mov word [tail+1],' /'
        mov word [tail+3],'C '
        mov di,tail+5           ; after " /C "
        mov [tail],cl
        add byte [tail],4
        rep movsb
        mov byte [di],13

        call find_comspec
        jc no_comspec

        ; Country separators; the defaults stay if AH38 fails.
        mov dx,country
        mov ax,3800h
        int 21h
        jc .seps
        mov al,[country+0dh]
        mov [time_sep],al
        mov al,[country+09h]
        mov [dec_sep],al
.seps:
        mov ah,2ch
        int 21h
        mov [start_cx],cx
        mov [start_dx],dx

        mov [fcb1+2],cs         ; parameter block segments
        mov [fcb2+2],cs
        mov [tailp+2],cs
        mov [saved_sp],sp
        mov dx,comspec
        mov bx,params
        mov ax,4b00h
        int 21h
        cli                     ; DOS 2 does not keep SS:SP over EXEC
        mov bx,cs
        mov ss,bx
        mov sp,[cs:saved_sp]
        sti
        mov ds,bx
        mov es,bx
        jc exec_failed
        mov ah,4dh
        int 21h
        mov [exit_code],al

        mov ah,2ch
        int 21h
        mov [stop_cx],cx
        mov [stop_dx],dx

        mov dx,msg_command      ; the command as typed, for saving with the times
        mov ah,9
        int 21h
        mov si,tail+5
        mov cl,[tail]
        sub cl,4
        xor ch,ch
.cmd:   lodsb
        mov dl,al
        call put
        loop .cmd
        mov dl,13
        call put
        mov dl,10
        call put
        mov dx,msg_start
        mov cx,[start_cx]
        mov bx,[start_dx]
        call print_line
        mov dx,msg_end
        mov cx,[stop_cx]
        mov bx,[stop_dx]
        call print_line

        ; Elapsed hundredths, wrapping over midnight, then back to h:m:s.hh.
        mov cx,[stop_cx]
        mov bx,[stop_dx]
        call hundredths
        push dx
        push ax
        mov cx,[start_cx]
        mov bx,[start_dx]
        call hundredths
        mov bx,ax
        mov cx,dx
        pop ax
        pop dx
        sub ax,bx
        sbb dx,cx
        jnc .positive
        add ax,0d600h           ; + 8,640,000 hundredths (one day)
        adc dx,083h
.positive:
        mov bx,6000
        div bx                  ; ax = minutes (< 1440), dx = hundredths in it
        push dx
        xor dx,dx
        mov bx,60
        div bx                  ; ax = hours, dx = minutes
        mov ch,al
        mov cl,dl
        pop ax
        mov bl,100
        div bl                  ; al = seconds, ah = hundredths
        mov bh,al
        mov bl,ah
        mov dx,msg_elapsed
        call print_line
        mov al,[exit_code]
        jmp exit

; CX:BX in the AH2C layout (CH hour, CL minute, BH second, BL hundredths) to
; hundredths of the day in DX:AX.
hundredths:
        mov al,ch
        mov ah,60
        mul ah                  ; hours*60
        xor ch,ch
        add ax,cx               ; minutes of the day (< 1440)
        mov cx,60
        mul cx                  ; dx:ax = minutes*60
        mov cl,bh
        add ax,cx
        adc dx,0                ; seconds of the day
        mov cx,ax
        mov ax,dx
        mov si,100
        mul si
        xchg ax,cx              ; cx = high*100
        mul si                  ; dx:ax = low*100
        add dx,cx
        mov cl,bl
        xor ch,ch
        add ax,cx
        adc dx,0
        ret

; Prints the label at DX, then CH hour, CL minute, BH second, BL hundredths.
print_line:
        push bx
        push cx
        mov ah,9
        int 21h
        pop cx
        pop bx
        mov al,ch
        call print_number
        mov dl,[time_sep]
        call put
        mov al,cl
        call print_two
        mov dl,[time_sep]
        call put
        mov al,bh
        call print_two
        mov dl,[dec_sep]
        call put
        mov al,bl
        call print_two
        mov dl,13
        call put
        mov dl,10
        jmp put

; AL (0..99) with no leading zero, or as two digits.
print_number:
        cmp al,10
        jae print_two
        add al,'0'
        mov dl,al
        jmp put
print_two:
        xor ah,ah
        mov dl,10
        div dl                  ; al = tens, ah = units
        xchg al,ah
        push ax
        mov dl,ah
        add dl,'0'
        call put
        pop ax
        mov dl,al
        add dl,'0'
put:    push ax
        mov ah,2
        int 21h
        pop ax
        ret

; Copies the COMSPEC value from the environment to `comspec`; CF=1 if absent.
find_comspec:
        push ds
        mov ds,[2ch]
        xor si,si
.entry: cmp byte [si],0
        je .none
        mov di,name
        mov cx,8
        repe cmpsb
        je .found
        dec si                  ; back to the byte that differed
.skip:  lodsb                   ; to the end of this entry
        or al,al
        jnz .skip
        jmp .entry
.found: mov di,comspec
        mov cx,127
.copy:  lodsb
        stosb
        or al,al
        loopnz .copy
        pop ds
        jnz .bad                ; no terminator within 127 bytes
        clc
        ret
.none:  pop ds
.bad:   stc
        ret

usage:  mov dx,msg_usage
        jmp fail
too_long:
        mov dx,msg_long
        jmp fail
no_memory:
        mov dx,msg_memory
        jmp fail
no_comspec:
        mov dx,msg_comspec
        jmp fail
exec_failed:
        push ax
        mov dx,msg_exec
        mov ah,9
        int 21h
        pop ax
        call print_number
        mov dl,13
        call put
        mov dl,10
        call put
        mov al,1
        jmp exit
fail:   mov ah,9
        int 21h
        mov al,1
exit:   mov ah,4ch
        int 21h

name        db 'COMSPEC='
time_sep    db ':'
dec_sep     db '.'
msg_command db 'Command  $'
msg_start   db 'Start    $'
msg_end     db 'End      $'
msg_elapsed db 'Elapsed  $'
msg_usage   db 'Runs a command and shows how long it took.',13,10
            db 'TIMEIT command [arguments]',13,10,'$'
msg_long    db 'TIMEIT: the command line is too long',13,10,'$'
msg_memory  db 'TIMEIT: cannot shrink its memory block',13,10,'$'
msg_comspec db 'TIMEIT: COMSPEC is not set',13,10,'$'
msg_exec    db 'TIMEIT: cannot run COMSPEC, DOS error $'
params      dw 0                ; inherit the environment
tailp       dw tail,0
fcb1        dw 5ch,0
fcb2        dw 6ch,0

absolute $
tail        resb 128
comspec     resb 128
country     resb 34
start_cx    resw 1
start_dx    resw 1
stop_cx     resw 1
stop_dx     resw 1
saved_sp    resw 1
exit_code   resb 1
            resb 512
stack_top:
end_of_program:
