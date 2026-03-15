BITS 16
ORG 0x7C00

%define STAGE2_SEGMENT 0x0800
%define STAGE2_SECTORS 16

start:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    sti
    mov [boot_drive], dl

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

    mov si, loading_text
    mov di, 80 * 2 * 22 + 10 * 2
    mov ah, 0x70
    call draw_string

    xor ax, ax
    mov es, ax
    mov bx, 0
    mov dl, [boot_drive]
    mov ah, 0x00
    int 0x13
    jc disk_error

    mov ax, STAGE2_SEGMENT
    mov es, ax
    xor bx, bx
    mov ah, 0x02
    mov al, STAGE2_SECTORS
    mov ch, 0x00
    mov cl, 0x02
    mov dh, 0x00
    mov dl, [boot_drive]
    int 0x13
    jc disk_error

    jmp STAGE2_SEGMENT:0x0000

disk_error:
    mov ax, 0xB800
    mov es, ax
    mov si, error_text
    mov di, 80 * 2 * 22 + 10 * 2
    mov ah, 0x4F
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
status_text db "Legacy BIOS stage 1 online", 0
hint_text db "Loading second stage from boot image", 0
loading_text db "Stage 2 loading...", 0
error_text db "Stage 2 load failed", 0
boot_drive db 0

times 510 - ($ - $$) db 0
dw 0xAA55
