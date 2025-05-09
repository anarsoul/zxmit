    MACRO EspSend Text
    ld hl, .txtB
    ld e, (.txtE - .txtB)
    call Wifi.espSend
    jr .txtE
.txtB 
    db Text
.txtE 
    ENDM

    MACRO EspCmd Text
    ld hl, .txtB
    ld e, (.txtE - .txtB)
    call Wifi.espSend
    jr .txtE
.txtB 
    db Text
    db 13, 10 
.txtE
    ENDM

    MACRO EspCmdOkErr text
    EspCmd text
    call Wifi.checkOkErr
    ENDM

    MACRO Border color
    ld a, color
    out (254), a
    ENDM

    module Wifi
init:
    EspSend "+++"
    ld b, 50
1
    halt
    djnz 1b

    call reset

    EspCmdOkErr "ATE0"
    jp c, .err

    EspCmdOkErr "AT+CIPDINFO=0" ; Disable additional info
    jp c, .err

    EspCmdOkErr "AT+CIPMUX=1" ; Multiplexing required for listening
    jp c, .err

    EspCmdOkErr "AT+CIPSERVERMAXCONN=1" ; Maximum 1 connection
    ; this command doesn't exist on older firmware, so skip checking for error

    EspCmdOkErr "AT+CIPSERVER=1,6144" ; Port number 
    jp c, .err

    call getMyIp

    ret
.err:
    ld hl, .err_msg
    call Display.putStr
    ret
.err_msg db 13, "ESP initialization error", 0

reset:
    EspCmdOkErr "AT"
    EspCmd "AT+RST"
.loop
    call Uart.read
    cp 'e' : jr nz, .loop
    call Uart.read : cp 'a' : jr nz, .loop
    call Uart.read : cp 'd' : jr nz, .loop
    call Uart.read : cp 'y' : jr nz, .loop
gotWait:
    call Uart.read : cp 'G' : jr nz, gotWait
    call Uart.read : cp 'O' : jr nz, gotWait
    call Uart.read : cp 'T' : jr nz, gotWait
    call Uart.read : cp ' ' : jr nz, gotWait
    call Uart.read : cp 'I' : jr nz, gotWait
    call Uart.read : cp 'P' : jr nz, gotWait
    ret

LONG_HEADER_SIZE  equ 17
SHORT_HEADER_SIZE equ 4
FLAGS_COMPRESSED  equ 1
FLAGS_LONG_HEADER equ 2

processPacket
    ; does EsxDOS need it preserved?
    push ix
    ld ix, recv_buffer
    ld a, (ix + 0) ; sequence
    ld (seq), a
    ld h, (ix + 2)
    ld l, (ix + 1)
    pop ix
    ld bc, LONG_HEADER_SIZE
    ld a, (recv_buffer + 3)
    and FLAGS_LONG_HEADER
    jr nz, 1f
    ld bc, SHORT_HEADER_SIZE
1:
    ld (header_size), bc
    add hl, bc
    ld bc, (data_size)
    sbc hl, bc
    ld a, h
    or l
    jr z, 1f

    ; error, packet size doesn't match the header
    ld a, '!' : rst #10
    ld a, 1
    ret

1:
    ld a, (file_opened)
    or a
    jr nz, 2f
    ld a, (recv_buffer + 3)
    and FLAGS_LONG_HEADER
    jr nz, 1f

    ; file is not opened, but we recevied a short header.
    ld a, '@' : rst #10
    ld a, 2
    ret
1:
    ld hl, recv_buffer + 4
    call EsxDOS.open
    ld a, 1
    ld (file_opened), a

2:
    ld a, (recv_buffer + 3)
    and FLAGS_COMPRESSED
    jr z, .notcompressed

    ld bc, (header_size)
    ld hl, recv_buffer
    add hl, bc
    ex hl, de
    ld hl, data_buffer
    ld bc, (data_buffered)
    add hl, bc
    ex hl, de
    call dzx0_standard

    ld bc, 1024
    jr .write

.notcompressed:
    ld hl, (data_size)
    ld bc, (header_size)
    sbc hl, bc
    ld bc, hl
    push bc
    ld bc, (header_size)
    ld hl, recv_buffer
    add hl, bc
    ex hl, de
    ld hl, data_buffer
    ld bc, (data_buffered)
    add hl, bc
    ex hl, de
    pop bc : push bc
    ldir
    pop bc

.write
    ld hl, (data_buffered)
    add hl, bc
    ld (data_buffered), hl

    ; accumulate at least 4 packets before writing
    ld a, (packets_buffered)
    inc a: and 3
    ld (packets_buffered), a
    jr nz, .skipWrite

    ld hl, data_buffer
    ld bc, (data_buffered)
    call EsxDOS.write
    ld bc, 0
    ld (data_buffered), bc
.skipWrite
    ; OK
    ld a, 0
    ret

recv:
    call Uart.read
    cp 'L' : jp z, .closedBegins 
    cp 'I' : jr nz, recv

    call Uart.read : cp 'P' : jr nz, recv
    call Uart.read : cp 'D' : jr nz, recv
    call Uart.read ; Comma
    ld hl, socket_num
.waitComma
    ; Read and store socket number
    push hl
    call Uart.read
    pop hl
    cp ',' :  jr z,1f
    ld (hl), a
    inc hl
    jr .waitComma
1:
    ld (hl), 0 ; Null-terminator

    ; Read and store data size
    ld hl, 0
1:
    push  hl
    call Uart.read
    pop hl 
    cp ':' : jr z, .storeDataSize
    sub 0x30 : ld c,l : ld b,h : add hl,hl : add hl,hl : add hl,bc : add hl,hl : ld c,a : ld b,0 : add hl,bc
    jr 1b

.storeDataSize
    ld (data_size), hl
    ex hl, de
    ld hl, recv_buffer
    call Uart.readBlock

    call processPacket
    or a
    jr nz, .packetErr
    ld a, (packets_buffered) : or a
    jr nz, .skipProgress
    ld a, '+' : rst #10

.skipProgress:
    EspSend "AT+CIPSEND="
    ld hl, socket_num
    call espSendZ
    EspSend ",4"
    ld a, 13: call Uart.write
    ld a, 10: call Uart.write
    call checkOkErr
    jr c, .ipSendErr

.wait
    call Uart.read
    cp '>'
    jr nz, .wait
    ld a, (seq)
    call Uart.write
    ld a, 0
    call Uart.write
    ld a, (data_size)
    call Uart.write
    ld a, (data_size + 1)
    call Uart.write
    ld a, 13: call Uart.write
    ld a, 10: call Uart.write
    jp recv
.packetErr:
    ld hl, .errPacket : call Display.putStr
    jr .exit
.ipSendErr:
    ld hl, .errIpSendStr1 : call Display.putStr
    jr .exit

.closedBegins
    call Uart.read : cp 'O' : jp nz, recv
    call Uart.read : cp 'S' : jp nz, recv
    call Uart.read : cp 'E' : jp nz, recv
    call Uart.read : cp 'D' : jp nz, recv

.exit:
    EspCmd "AT+CIPSERVER=0,1"
    ld a, (file_opened)
    or a
    ret z
    ld hl, data_buffer
    ld bc, (data_buffered)
    ld a, b: or c
    call nz, EsxDOS.write
    call EsxDOS.close
    ret

.errPacket:
    db 13, "Protocol error", 0
.errIpSendStr1:
    db 13, "Error on AT+CIPSEND", 0

getMyIp:
    EspCmd "AT+CIFSR"
.loop
    call Uart.read
    cp 'P' : jr z, .infoStart
    jr .loop
.infoStart
    call Uart.read : cp ',' : jr nz, .loop
    call Uart.read : cp '"' : jr nz, .loop
    ld hl, ipAddr
.copyIpLoop
    push hl
    call Uart.read : pop hl : cp '"' : jr z, .finish
    ld (hl), a
    inc hl
    jr .copyIpLoop
.finish
    xor a : ld (hl), a
    call checkOkErr

    ld hl, ipAddr
    ld de, justZeros
.checkZero    
    ld a, (hl)
    and a : jr z, .err
    ld b, a : ld a, (de)
    cp b
    ret nz
    inc hl : inc de
    jr .checkZero
.err
    ld hl, .err_connect : call Display.putStr
    ret
.err_connect db "Use Network Manager and connect to Wifi", 13, "System halted", 0

ipAddr db "000.000.000.000", 0
justZeros db "0.0.0.0", 0

; Send buffer to UART
; HL - buff
; E - count
espSend:
    ld a, (hl) 
    push hl, de
    call Uart.write
    pop de, hl
    inc hl 
    dec e
    jr nz, espSend
    ret

espSendZ:
    ld a, (hl) : and a : ret z
    push hl
    call Uart.write
    pop hl
    inc hl
    jr espSendZ

checkOkErr:
    call Uart.read
    cp 'O' : jr z, .okStart ; OK
    cp 'E' : jr z, .errStart ; ERROR
    cp 'F' : jr z, .failStart ; FAIL
    jr checkOkErr
.okStart
    call Uart.read : cp 'K' : jr nz, checkOkErr
    call Uart.read : cp 13  : jr nz, checkOkErr
    call .flushToLF
    or a
    ret
.errStart
    call Uart.read : cp 'R' : jr nz, checkOkErr
    call Uart.read : cp 'R' : jr nz, checkOkErr
    call Uart.read : cp 'O' : jr nz, checkOkErr
    call Uart.read : cp 'R' : jr nz, checkOkErr
    call .flushToLF
    scf 
    ret 
.failStart
    call Uart.read : cp 'A' : jr nz, checkOkErr
    call Uart.read : cp 'I' : jr nz, checkOkErr
    call Uart.read : cp 'L' : jr nz, checkOkErr
    call .flushToLF
    scf
    ret
.flushToLF
    call Uart.read
    cp 10 : jr nz, .flushToLF
    ret

seq db 0
counter db 0
file_opened db 0
data_size dw 0
header_size dw 0
data_buffered dw 0
packets_buffered db 0
socket_num db "00000000", 0
    endmodule
