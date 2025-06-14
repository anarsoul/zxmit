    module Uart
UART_DATA_REG = #c6
UART_STAT_REG = #c7
UART_BYTE_RECEIVED = #80
UART_BYTE_SENDING = #40
ZXUNO_ADDR = #FC3B
ZXUNO_REG = #FD3B
init:
    ld bc, ZXUNO_ADDR : ld a, UART_STAT_REG : out (c), a
    ld bc, ZXUNO_REG : in A, (c)
    ld bc, ZXUNO_ADDR : ld a, UART_DATA_REG : out (c), a
    ld bc, ZXUNO_REG : in A, (c)

    ei 
    ld b,50
1
    push bc
    call uartRead
    pop bc
    halt
    djnz 1b

    ld bc, #ffff
.loop
    push bc
    call uartRead
    pop bc
    dec bc 
    ld a,b : or c
    jr z, .loop

    ld hl, set_speed_cmd
.speedCmd
    ld a, (hl) : and a : ret z
    push hl
    call write
    pop hl
    inc hl
    jr .speedCmd

write:
    push af
    ld bc, ZXUNO_ADDR : ld a, UART_STAT_REG : out (c), a
    ld bc, ZXUNO_REG : in A, (c) : and UART_BYTE_RECEIVED
    jr nz, .is_recvF
.checkSent
    ld bc, ZXUNO_REG : in A, (c) : and UART_BYTE_SENDING
    jr nz, .checkSent

    ld bc, ZXUNO_ADDR : ld a, UART_DATA_REG : out (c), a

    ld bc, ZXUNO_REG : pop af : out (c), a
    ret
.is_recvF
    push af : push hl
    ld hl, is_recv : ld a, 1 : ld (hl), a 
    
    pop hl : pop af
    jr .checkSent


read:
    call uartRead
    jr nc, read
    ret

; Read block from UART
; HL - destination
; DE - size
readBlock:
    ld a, (poked_byte): and 1: jr z, .noBuff
    xor a : ld (poked_byte), a: ld a, (byte_buff)
    ld (hl), a
    dec de
.noBuff:
    ld a, d
    or e
    ret z

    ; check is_recv flag, clear it and skip waiting if necessary
    ld a, (is_recv): push af
    xor a : ld (is_recv), a
    pop af: or a
    jr nz, .skipWait
    di
.loop
    ld bc, ZXUNO_ADDR : ld a, UART_STAT_REG : out (c), a
.waitByte
    ld bc, ZXUNO_REG: in a, (c) : and UART_BYTE_RECEIVED
    jr z, .waitByte

.skipWait
    ld bc, ZXUNO_ADDR : ld a, UART_DATA_REG : out (c), a
    ld bc, ZXUNO_REG: in a, (c)

    ld (hl), a
    inc hl
    dec de
    ld a, d : or e : jr nz, .loop
    ei
    ret

; Read byte from UART
; A: byte
; B:
;     1 - Was read
;     0 - Nothing to read
uartRead:
    ld a, (poked_byte) : and 1 : jr nz, .retBuff

    ld a, (is_recv) : and 1 : jr nz, recvRet

    ld bc, ZXUNO_ADDR : ld a, UART_STAT_REG : out (c), a
    ld bc, ZXUNO_REG : in a, (c) : and UART_BYTE_RECEIVED
    jr nz, retReadByte

    or a
    ret
.retBuff
    ld a, 0 : ld (poked_byte), a : ld a, (byte_buff)
    scf 
    ret

retReadByte:
    xor a : ld (poked_byte), a : ld (is_recv), a

    ld bc, ZXUNO_ADDR : ld a, UART_DATA_REG : out (c), a
    ld bc, ZXUNO_REG : in a, (c)

    scf
    ret

recvRet:
    ld bc, ZXUNO_ADDR : ld a,  UART_DATA_REG : out (c),a

    ld bc, ZXUNO_REG : in a, (c)
    ld hl, is_recv : ld (hl), 0
    ld hl, poked_byte : ld (hl), 0
    
    scf
    ret

set_speed_cmd db "AT+UART_DEF=115200,8,1,0,2", 13, 10, 0

poked_byte defb 0
byte_buff defb 0
is_recv defb 0

    endmodule
