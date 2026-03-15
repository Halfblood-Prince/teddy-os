BITS 16
ORG 0x7C00

start:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    sti

    mov ax, 0x0003
    int 0x10

    mov ax, 0xB800
    mov es, ax
    xor di, di
    mov ah, 0x1F
    mov al, ' '
    mov cx, 80 * 25
    rep stosw

    mov si, title_text
    mov di, 80 * 2 * 2 + 10 * 2
    mov ah, 0x1F
    call draw_string

    mov si, status_text
    mov di, 80 * 2 * 5 + 10 * 2
    mov ah, 0x1E
    call draw_string

    mov si, hint_text
    mov di, 80 * 2 * 8 + 10 * 2
    mov ah, 0x17
    call draw_string

    mov si, footer_text
    mov di, 80 * 2 * 22 + 10 * 2
    mov ah, 0x70
    call draw_string

.halt:
    hlt
    jmp .halt

draw_string:
    lodsb
    test al, al
    jz .done
    stosw
    jmp draw_string
.done:
    ret

title_text db "TEDDY-OS", 0
status_text db "Legacy BIOS boot path online", 0
hint_text db "Minimal reset baseline for VMware legacy boot", 0
footer_text db "Boot OK - BIOS mode", 0

times 510 - ($ - $$) db 0
dw 0xAA55

