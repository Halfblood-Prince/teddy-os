BITS 16
ORG 0x0000

stage2_start:
    cli
    mov ax, cs
    mov ds, ax
    mov ss, ax
    mov sp, 0xFFFE
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
    mov di, 80 * 2 * 2 + 8 * 2
    mov ah, 0x1F
    call draw_string

    mov si, status_text
    mov di, 80 * 2 * 5 + 8 * 2
    mov ah, 0x1E
    call draw_string

    mov si, detail_text
    mov di, 80 * 2 * 8 + 8 * 2
    mov ah, 0x17
    call draw_string

    mov si, next_text
    mov di, 80 * 2 * 11 + 8 * 2
    mov ah, 0x1A
    call draw_string

    mov si, footer_text
    mov di, 80 * 2 * 22 + 8 * 2
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
status_text db "Legacy BIOS stage 2 online", 0
detail_text db "Stage 1 loaded this program from disk sectors", 0
next_text db "Next: keyboard input, graphics mode, Rust kernel handoff", 0
footer_text db "Boot OK - Stage 2 running", 0

times (8 * 512) - ($ - $$) db 0
