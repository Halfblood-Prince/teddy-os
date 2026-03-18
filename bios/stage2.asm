BITS 16
ORG 0x8000

%define INPUT_BUFFER_SIZE 64
%define STAGE2_SECTORS 96
%define BOOT_INFO_ADDR   0x18000
%define BOOT_INFO_SEG    0x1800
%define MODE_INFO_ADDR   0x19000
%define MODE_INFO_SEG    0x1900
%define KERNEL_LOAD_ADDR 0x20000
%define KERNEL_LOAD_SEG  0x2000
%define KERNEL_SECTORS   512
%define KERNEL_LBA_START (1 + STAGE2_SECTORS)
%define MODE13_FRAMEBUFFER 0x000A0000
%define MODE13_WIDTH 320
%define MODE13_HEIGHT 200
%define MODE13_PITCH 320
%define MODE13_BPP 8
%define VBE_MODE_640X480X8   0x101
%define VBE_MODE_800X600X8   0x103
%define VBE_MODE_1024X768X8  0x105

stage2_start:
    cli
    cld
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7000
    sti
    mov [boot_drive], dl

    call redraw_shell_screen
    mov byte [input_len], 0

shell_loop:
    call draw_prompt
    call read_line
    call execute_command
    jmp shell_loop

draw_string:
    lodsb
    test al, al
    jz .done
    stosw
    jmp draw_string
.done:
    ret

draw_prompt:
    mov si, prompt_text
    call print_string
    ret

print_string:
    lodsb
    test al, al
    jz .done
    call put_char
    jmp print_string
.done:
    ret

print_newline:
    mov al, 13
    call put_char
    mov al, 10
    call put_char
    ret

put_char:
    mov ah, 0x0E
    mov bh, 0x00
    mov bl, 0x0F
    int 0x10
    ret

read_line:
    mov di, input_buffer
    mov byte [input_len], 0
.read_key:
    xor ah, ah
    int 0x16
    cmp al, 13
    je .enter
    cmp al, 8
    je .backspace
    cmp al, 0
    je .read_key
    cmp al, 32
    jb .read_key

    mov bl, [input_len]
    cmp bl, INPUT_BUFFER_SIZE - 1
    jae .read_key

    mov [di], al
    inc di
    inc byte [input_len]
    call put_char
    jmp .read_key

.backspace:
    cmp byte [input_len], 0
    je .read_key
    dec di
    dec byte [input_len]
    mov al, 8
    call put_char
    mov al, ' '
    call put_char
    mov al, 8
    call put_char
    jmp .read_key

.enter:
    mov byte [di], 0
    call print_newline
    ret

execute_command:
    mov al, [input_len]
    cmp al, 0
    je .done

    mov si, input_buffer
    mov di, cmd_help
    call strings_equal
    cmp al, 1
    je .help

    mov si, input_buffer
    mov di, cmd_clear
    call strings_equal
    cmp al, 1
    je .clear

    mov si, input_buffer
    mov di, cmd_info
    call strings_equal
    cmp al, 1
    je .info

    mov si, input_buffer
    mov di, cmd_reboot
    call strings_equal
    cmp al, 1
    je .reboot

    mov si, input_buffer
    mov di, cmd_graphics
    call strings_equal
    cmp al, 1
    je .graphics

    mov si, input_buffer
    mov di, cmd_kernel
    call strings_equal
    cmp al, 1
    je .kernel

    mov si, input_buffer
    mov di, cmd_kernelgfx
    call strings_equal
    cmp al, 1
    je .kernelgfx

    mov si, input_buffer
    mov di, cmd_kernelgfx800
    call strings_equal
    cmp al, 1
    je .kernelgfx800

    mov si, input_buffer
    mov di, cmd_kernelgfx1024
    call strings_equal
    cmp al, 1
    je .kernelgfx1024

    mov si, input_buffer
    mov di, cmd_echo_prefix
    call starts_with
    cmp al, 1
    je .echo

    mov si, unknown_text
    call print_string
    call print_newline
    jmp .done

.help:
    mov si, help_text_1
    call print_string
    call print_newline
    mov si, help_text_2
    call print_string
    call print_newline
    mov si, help_text_3
    call print_string
    call print_newline
    jmp .done

.clear:
    call redraw_shell_screen
    jmp .done

.info:
    mov si, info_text_1
    call print_string
    call print_newline
    mov si, info_text_2
    call print_string
    call print_newline
    jmp .done

.echo:
    mov si, input_buffer + 5
    call print_string
    call print_newline
    jmp .done

.reboot:
    call bios_warm_reboot
    jmp $

.graphics:
    call graphics_demo
    call redraw_shell_screen
    jmp .done

.kernel:
    mov byte [boot_video_mode], 0
    mov dword [boot_framebuffer_addr], 0
    mov word [boot_framebuffer_width], 0
    mov word [boot_framebuffer_height], 0
    mov word [boot_framebuffer_pitch], 0
    mov byte [boot_framebuffer_bpp], 0
    call load_kernel_image
    jc .kernel_failed
    call write_boot_info
    call enter_long_mode
    jmp $

.kernelgfx:
    mov bx, VBE_MODE_640X480X8
    call set_kernel_vbe_mode
    jnc .kernelgfx_ready
    call set_kernel_graphics_mode
.kernelgfx_ready:
    mov byte [boot_video_mode], 1
    call load_kernel_image
    jc .kernel_failed
    call write_boot_info
    call enter_long_mode
    jmp $

.kernelgfx800:
    mov bx, VBE_MODE_800X600X8
    call set_kernel_vbe_mode
    jnc .kernelgfx800_ready
    call set_kernel_graphics_mode
.kernelgfx800_ready:
    mov byte [boot_video_mode], 1
    call load_kernel_image
    jc .kernel_failed
    call write_boot_info
    call enter_long_mode
    jmp $

.kernelgfx1024:
    mov bx, VBE_MODE_1024X768X8
    call set_kernel_vbe_mode
    jnc .kernelgfx1024_ready
    call set_kernel_graphics_mode
.kernelgfx1024_ready:
    mov byte [boot_video_mode], 1
    call load_kernel_image
    jc .kernel_failed
    call write_boot_info
    call enter_long_mode
    jmp $

.kernel_failed:
    mov si, kernel_fail_text
    call print_string
    call print_newline
    jmp .done

.done:
    ret

redraw_shell_screen:
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

    mov si, shell_text
    mov di, 80 * 2 * 14 + 8 * 2
    mov ah, 0x1F
    call draw_string

    mov si, footer_text
    mov di, 80 * 2 * 22 + 8 * 2
    mov ah, 0x70
    call draw_string
    ret

graphics_demo:
    mov ax, 0x0013
    int 0x10

    mov ax, 0xA000
    mov es, ax

    mov al, 1
    call clear_vga

    mov ax, 0
    mov bx, 0
    mov cx, 320
    mov dx, 200
    mov si, 1
    call fill_rect_13h

    mov ax, 0
    mov bx, 176
    mov cx, 320
    mov dx, 24
    mov si, 7
    call fill_rect_13h

    mov ax, 8
    mov bx, 180
    mov cx, 72
    mov dx, 16
    mov si, 2
    call fill_rect_13h

    mov ax, 214
    mov bx, 26
    mov cx, 88
    mov dx, 56
    mov si, 15
    call fill_rect_13h

    mov ax, 214
    mov bx, 26
    mov cx, 88
    mov dx, 10
    mov si, 9
    call fill_rect_13h

    mov ax, 18
    mov bx, 22
    mov cx, 34
    mov dx, 34
    mov si, 14
    call fill_rect_13h

    mov ax, 62
    mov bx, 22
    mov cx, 34
    mov dx, 34
    mov si, 10
    call fill_rect_13h

    mov ax, 106
    mov bx, 22
    mov cx, 34
    mov dx, 34
    mov si, 12
    call fill_rect_13h

    mov dh, 1
    mov dl, 2
    call set_cursor
    mov si, gfx_title
    call print_string

    mov dh, 5
    mov dl, 28
    call set_cursor
    mov si, gfx_panel_title
    call print_string

    mov dh, 7
    mov dl, 28
    call set_cursor
    mov si, gfx_panel_body
    call print_string

    mov dh, 23
    mov dl, 2
    call set_cursor
    mov si, gfx_footer
    call print_string

    xor ah, ah
    int 0x16
    ret

set_cursor:
    mov ah, 0x02
    mov bh, 0x00
    int 0x10
    ret

clear_vga:
    xor di, di
    mov cx, 320 * 200
    rep stosb
    ret

fill_rect_13h:
    push ax
    push bx
    push cx
    push dx
    push si
    push di
    push bp

    mov bp, ax
    mov ax, si
    mov [rect_color], al
.row_loop:
    cmp dx, 0
    je .done

    mov ax, bx
    mov di, ax
    shl ax, 8
    shl di, 6
    add di, ax
    add di, bp

    mov al, [rect_color]
    push dx
    rep stosb
    pop dx

    inc bx
    dec dx
    jmp .row_loop

.done:
    pop bp
    pop di
    pop si
    pop dx
    pop cx
    pop bx
    pop ax
    ret

load_kernel_image:
    push ax
    push bx
    push cx
    push dx
    push si
    push di
    push es

    mov ax, KERNEL_LOAD_SEG
    mov es, ax
    xor bx, bx
    mov si, KERNEL_LBA_START
    mov di, KERNEL_SECTORS

.load_loop:
    cmp di, 0
    je .success

    mov ax, si
    call lba_to_chs

    mov ah, 0x02
    mov al, 0x01
    mov dl, [boot_drive]
    int 0x13
    jc .error

    mov ax, es
    add ax, 0x20
    mov es, ax
    inc si
    dec di
    jmp .load_loop

.success:
    clc
    jmp .out

.error:
    stc

.out:
    pop es
    pop di
    pop si
    pop dx
    pop cx
    pop bx
    pop ax
    ret

write_boot_info:
    push ax
    push bx
    push cx
    push si
    push di
    push ds
    push es

    xor ax, ax
    mov ds, ax
    mov ax, BOOT_INFO_SEG
    mov es, ax
    xor di, di

    mov si, boot_info_signature
    mov cx, 8
    rep movsb

    mov byte [es:di], 2
    inc di
    mov al, [boot_drive]
    mov [es:di], al
    inc di
    mov ax, KERNEL_LOAD_SEG
    mov [es:di], ax
    add di, 2
    mov ax, KERNEL_SECTORS
    mov [es:di], ax
    add di, 2
    mov ax, STAGE2_SECTORS
    mov [es:di], ax
    add di, 2

    mov al, [boot_video_mode]
    mov [es:di], al
    inc di
    mov al, [boot_framebuffer_bpp]
    mov [es:di], al
    inc di

    mov eax, [boot_framebuffer_addr]
    mov [es:di], eax
    add di, 4
    mov ax, [boot_framebuffer_width]
    mov [es:di], ax
    add di, 2
    mov ax, [boot_framebuffer_height]
    mov [es:di], ax
    add di, 2
    mov ax, [boot_framebuffer_pitch]
    mov [es:di], ax

    pop es
    pop ds
    pop di
    pop si
    pop cx
    pop bx
    pop ax
    ret

lba_to_chs:
    push ax
    push bx

    xor dx, dx
    mov bx, 18
    div bx
    mov cl, dl
    inc cl

    xor dx, dx
    mov bx, 2
    div bx
    ; Return CHS in CH, CL, and DH for INT 13h sector reads.
    mov dh, dl
    mov ch, al

    pop bx
    pop ax
    ret

enter_long_mode:
    cli
    call enable_a20_fast
    xor eax, eax
    mov ax, ds
    shl eax, 4
    add eax, gdt_start
    mov [gdt_descriptor + 2], eax
    lgdt [gdt_descriptor]
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    jmp 0x08:protected_mode_entry

set_kernel_graphics_mode:
    mov ax, 0x0013
    int 0x10
    mov dword [boot_framebuffer_addr], MODE13_FRAMEBUFFER
    mov word [boot_framebuffer_width], MODE13_WIDTH
    mov word [boot_framebuffer_height], MODE13_HEIGHT
    mov word [boot_framebuffer_pitch], MODE13_PITCH
    mov byte [boot_framebuffer_bpp], MODE13_BPP
    clc
    ret

set_kernel_vbe_mode:
    push ax
    push bx
    push cx
    push dx
    push di
    push es

    mov cx, bx
    mov ax, MODE_INFO_SEG
    mov es, ax
    xor di, di
    mov ax, 0x4F01
    int 0x10
    cmp ax, 0x004F
    jne .error

    test byte [es:0], 0x80
    jz .error

    mov ax, 0x4F02
    mov bx, cx
    or bx, 0x4000
    int 0x10
    cmp ax, 0x004F
    jne .error

    mov eax, [es:40]
    mov [boot_framebuffer_addr], eax
    mov ax, [es:18]
    mov [boot_framebuffer_width], ax
    mov ax, [es:20]
    mov [boot_framebuffer_height], ax
    mov ax, [es:16]
    mov [boot_framebuffer_pitch], ax
    mov al, [es:25]
    mov [boot_framebuffer_bpp], al
    clc
    jmp .out

.error:
    stc

.out:
    pop es
    pop di
    pop dx
    pop cx
    pop bx
    pop ax
    ret

enable_a20_fast:
    in al, 0x92
    or al, 00000010b
    out 0x92, al
    ret

BITS 32
protected_mode_entry:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, 0x7000
    cld

    call enable_fpu_sse
    call setup_page_tables

    mov eax, pml4_table
    mov cr3, eax

    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    mov eax, cr0
    or eax, 0x80000000
    mov cr0, eax

    jmp 0x18:long_mode_entry

enable_fpu_sse:
    mov eax, cr0
    and eax, 0xFFFFFFFB
    or eax, 0x00000002
    mov cr0, eax

    clts

    mov eax, cr4
    or eax, (1 << 9) | (1 << 10)
    mov cr4, eax

    fninit
    ret

setup_page_tables:
    mov edi, pml4_table
    xor eax, eax
    mov ecx, (4096 * 3) / 4
    rep stosd

    mov eax, pdpt_table
    or eax, 0x003
    mov [pml4_table], eax
    mov dword [pml4_table + 4], 0

    mov eax, pd_table
    or eax, 0x003
    mov [pdpt_table], eax
    mov dword [pdpt_table + 4], 0

    mov edi, pd_table
    xor ebx, ebx
    mov ecx, 16

.map_low_pages:
    mov eax, ebx
    shl eax, 21
    or eax, 0x83
    mov [edi], eax
    mov dword [edi + 4], 0
    add edi, 8
    inc ebx
    loop .map_low_pages
    ret

BITS 64
long_mode_entry:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax
    mov rsp, 0x80000

    mov rax, 0xB8002
    mov word [rax], 0x2F4C

    mov rdi, BOOT_INFO_ADDR
    mov rax, KERNEL_LOAD_ADDR
    jmp rax

BITS 16
bios_warm_reboot:
    cli
    xor ax, ax
    mov ds, ax
    mov word [0x0472], 0x1234
    jmp 0xFFFF:0x0000

strings_equal:
    push si
    push di
.loop:
    mov al, [si]
    mov ah, [di]
    cmp al, ah
    jne .no
    test al, al
    je .yes
    inc si
    inc di
    jmp .loop
.yes:
    mov al, 1
    jmp .out
.no:
    mov al, 0
.out:
    pop di
    pop si
    ret

starts_with:
    push si
    push di
.loop:
    mov ah, [di]
    test ah, ah
    je .yes
    mov al, [si]
    cmp al, ah
    jne .no
    inc si
    inc di
    jmp .loop
.yes:
    mov al, 1
    jmp .out
.no:
    mov al, 0
.out:
    pop di
    pop si
    ret

title_text db "TEDDY-OS", 0
status_text db "Legacy BIOS stage 2 online", 0
detail_text db "Stage 1 loaded this program from disk sectors", 0
next_text db "Next: graphics mode and x86_64 kernel handoff", 0
shell_text db "Shell ready. Commands: help, clear, info, echo, graphics, kernel, kernelgfx, kernelgfx800, kernelgfx1024, reboot", 0
footer_text db "Boot OK - Stage 2 running", 0
prompt_text db "> ", 0
unknown_text db "Unknown command. Type help.", 0
help_text_1 db "help  - list commands", 0
help_text_2 db "info  - show stage information", 0
help_text_3 db "clear - clear, echo X, graphics, kernel, kernelgfx, kernelgfx800, kernelgfx1024, reboot", 0
info_text_1 db "Teddy-OS BIOS Stage 2 is using INT 16h for keyboard input.", 0
info_text_2 db "Kernelgfx boots the kernel in VGA mode 13h for graphics work.", 0
gfx_title db "TEDDY-OS GRAPHICS", 0
gfx_panel_title db "RESET GUI", 0
gfx_panel_body db "Mode 13h online", 0
gfx_footer db "Press any key to return", 0
kernel_fail_text db "Rust kernel load failed.", 0
boot_info_signature db "TEDDYOS", 0

cmd_help db "help", 0
cmd_clear db "clear", 0
cmd_info db "info", 0
cmd_graphics db "graphics", 0
cmd_kernel db "kernel", 0
cmd_kernelgfx db "kernelgfx", 0
cmd_kernelgfx800 db "kernelgfx800", 0
cmd_kernelgfx1024 db "kernelgfx1024", 0
cmd_reboot db "reboot", 0
cmd_echo_prefix db "echo ", 0

rect_color db 0
boot_drive db 0
boot_video_mode db 0
boot_framebuffer_bpp db 0
boot_framebuffer_addr dd 0
boot_framebuffer_width dw 0
boot_framebuffer_height dw 0
boot_framebuffer_pitch dw 0
input_len db 0
input_buffer times INPUT_BUFFER_SIZE db 0

align 8
gdt_start:
    dq 0x0000000000000000
    dq 0x00CF9A000000FFFF
    dq 0x00CF92000000FFFF
    dq 0x00AF9A000000FFFF
gdt_end:

gdt_descriptor:
    dw gdt_end - gdt_start - 1
    dd 0

align 4096
pml4_table:
    times 512 dq 0

align 4096
pdpt_table:
    times 512 dq 0

align 4096
pd_table:
    times 512 dq 0

times (STAGE2_SECTORS * 512) - ($ - $$) db 0
