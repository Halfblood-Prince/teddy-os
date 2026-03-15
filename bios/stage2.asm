BITS 16
ORG 0x0000

%define INPUT_BUFFER_SIZE 64

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

    mov si, shell_text
    mov di, 80 * 2 * 14 + 8 * 2
    mov ah, 0x1F
    call draw_string

    mov si, footer_text
    mov di, 80 * 2 * 22 + 8 * 2
    mov ah, 0x70
    call draw_string

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
    mov al, 0
    mov [di], al
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
    mov ax, 0x0003
    int 0x10
    mov si, title_text
    call print_string
    call print_newline
    mov si, status_text
    call print_string
    call print_newline
    mov si, detail_text
    call print_string
    call print_newline
    mov si, shell_text
    call print_string
    call print_newline
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
    int 0x19
    jmp $

.done:
    ret

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
next_text db "Next: graphics mode and Rust kernel handoff", 0
shell_text db "Shell ready. Commands: help, clear, info, echo, reboot", 0
footer_text db "Boot OK - Stage 2 running", 0
prompt_text db "> ", 0
unknown_text db "Unknown command. Type help.", 0
help_text_1 db "help  - list commands", 0
help_text_2 db "info  - show stage information", 0
help_text_3 db "clear - clear screen, echo X, reboot", 0
info_text_1 db "Teddy-OS BIOS Stage 2 is reading keyboard input via INT 16h.", 0
info_text_2 db "This is the first interactive reset baseline.", 0

cmd_help db "help", 0
cmd_clear db "clear", 0
cmd_info db "info", 0
cmd_reboot db "reboot", 0
cmd_echo_prefix db "echo ", 0

input_len db 0
input_buffer times INPUT_BUFFER_SIZE db 0

times (8 * 512) - ($ - $$) db 0
