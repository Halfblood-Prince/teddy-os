BITS 32
ORG 0x10000

kernel_start:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, 0x9F000

    mov edi, 0xB8000
    mov eax, 0x1F201F20
    mov ecx, 80 * 25 / 2
    rep stosd

    mov edi, 0xB8000 + ((2 * 80) + 8) * 2
    mov esi, kernel_title
    mov ah, 0x1F
    call draw_string_pm

    mov edi, 0xB8000 + ((5 * 80) + 8) * 2
    mov esi, kernel_status
    mov ah, 0x1E
    call draw_string_pm

    mov edi, 0xB8000 + ((8 * 80) + 8) * 2
    mov esi, kernel_detail
    mov ah, 0x17
    call draw_string_pm

    mov edi, 0xB8000 + ((11 * 80) + 8) * 2
    mov esi, kernel_next
    mov ah, 0x1A
    call draw_string_pm

    mov edi, 0xB8000 + ((22 * 80) + 8) * 2
    mov esi, kernel_footer
    mov ah, 0x70
    call draw_string_pm

.halt:
    hlt
    jmp .halt

draw_string_pm:
    lodsb
    test al, al
    jz .done
    mov [edi], al
    mov [edi + 1], ah
    add edi, 2
    jmp draw_string_pm
.done:
    ret

kernel_title db "TEDDY-OS KERNEL", 0
kernel_status db "Protected mode kernel handoff succeeded", 0
kernel_detail db "Stage 2 loaded this kernel from disk and entered 32-bit mode", 0
kernel_next db "Next: replace this stub with a Rust kernel payload", 0
kernel_footer db "Kernel running - halt loop active", 0

times (16 * 512) - ($ - $$) db 0
