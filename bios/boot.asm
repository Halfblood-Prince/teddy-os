BITS 16
ORG 0x7C00

%define STAGE2_LOAD_ADDR 0x8000
%define STAGE2_SECTORS 64

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

    xor bx, bx
    mov bx, STAGE2_LOAD_ADDR
    mov si, 1
    mov di, STAGE2_SECTORS

.load_loop:
    cmp di, 0
    je .loaded

    mov ax, si
    call lba_to_chs

    mov ah, 0x02
    mov al, 0x01
    mov dl, [boot_drive]
    int 0x13
    jc disk_error

    add bx, 512
    inc si
    dec di
    jmp .load_loop

.loaded:
    jmp 0x0000:STAGE2_LOAD_ADDR

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

lba_to_chs:
    push ax
    push bx
    push dx

    xor dx, dx
    mov bx, 18
    div bx
    mov cl, dl
    inc cl

    xor dx, dx
    mov bx, 2
    div bx
    mov dh, dl
    mov ch, al

    pop dx
    pop bx
    pop ax
    ret

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
hint_text db "Loading x86_64 stage 2 from boot image", 0
loading_text db "Stage 2 loading...", 0
error_text db "Stage 2 load failed", 0
boot_drive db 0

times 510 - ($ - $$) db 0
dw 0xAA55
