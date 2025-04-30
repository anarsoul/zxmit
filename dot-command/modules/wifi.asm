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

    module Wifi
init:
    EspSend "+++"
    ld b, 50
1
    halt
    djnz 1b

    call reset

    EspCmdOkErr "ATE0"
    jr c, .err

    EspCmdOkErr "AT+CIPDINFO=0" ; Disable additional info
    jr c, .err

    EspCmdOkErr "AT+CIPMUX=1" ; Multiplexing required for listening
    jr c, .err

    EspCmdOkErr "AT+CIPSERVER=1,6144" ; Port number 
    jr c, .err

    call getMyIp

    ret
.err
    ld hl, .err_msg
    call Display.putStr
    di : halt
.err_msg db 13, "ESP error! Halted!", 0

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

; BC - chunk size
writeChunkOrStoreFN:
    ld a, (filename_received)
    or a
    jr nz, .writeChunk
    push bc ; chunk len
    ld a, (fn_buffer_avail)
    ld b, 0 : ld c, a
    ld hl, filename
    add hl, bc
    ld de, hl
    ld hl, buffer
    ld b, a
1:
    ld a, (hl)
    ld (de), a
    inc hl
    inc de
    inc b
    ld a, b
    pop bc ; chunk len
    dec bc
    push bc ; chunk len
    cp 32
    jr z, .filenameComplete
    push af ; filename bytes received
    ld a, b
    or c ; end of chuck. Don't forget to store a!
    jr z, .endOfChunk
    pop af
    ld b, a
    jr 1b

.filenameComplete
    push hl ; buffer + bytes used for filename
    ld a, 1
    ld (filename_received), a
    ld hl, filename
    call EsxDOS.prepareFile
    pop hl ; buffer + bytes used for filename
    pop bc ; chunk len
    ld a, b
    or c
    ret z ; end of chunk
    push bc ; chunk len
    ld de, buffer
    ldir ; move non-consumed data to buffer start
    pop bc

.writeChunk:
    call EsxDOS.writeChunk
    ret

.endOfChunk
    pop af ; filename bytes received
    pop bc
    ld (fn_buffer_avail), a
    ret

recvWithFilename:
    xor a
    ld (filename_received), a
    ld (fn_buffer_avail), a
    ld hl, filename
    ld bc, 32
1:
    ld (hl), 0
    inc hl
    dec bc
    ld a, b
    or c
    jr nz, 1b
.recv:
    call Uart.read
    cp 'L' : jp z, .closedBegins 
    cp 'I' : jr nz, .recv
    call Uart.read : cp 'P' : jr nz, .recv
    call Uart.read : cp 'D' : jr nz, .recv
    call Uart.read ; Comma :-) 
.waitComma
    call Uart.read ; We don't care about socket number :-)
    cp ',' :  jr nz,.waitComma
    ld hl,0			; count lenght
.cil1	
    push  hl
    call Uart.read
    pop hl 
    cp ':' : jr z, .storeAvail
    sub 0x30 : ld c,l : ld b,h : add hl,hl : add hl,hl : add hl,bc : add hl,hl : ld c,a : ld b,0 : add hl,bc
    jr .cil1
.storeAvail
    ld (data_avail), hl
    ld de, buffer
.loadPacket
    push hl
    push de
    call Uart.read 
    pop de 
    pop hl
    ld (de), a
    inc de
    dec hl
    ld a, h : or l : jr nz, .loadPacket

    ld hl, (data_avail)
    ld bc, hl
    call writeChunkOrStoreFN
    ld a, '+' : rst #10

    jp .recv
    
.closedBegins
    call Uart.read : cp 'O' : jr nz, .recv
    call Uart.read : cp 'S' : jr nz, .recv
    call Uart.read : cp 'E' : jr nz, .recv
    call Uart.read : cp 'D' : jr nz, .recv

    EspCmd "AT+RST"

    jp EsxDOS.close

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
    jr $
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

filename_received db 0
fn_buffer_avail db 32
filename ds 32
data_avail dw 0
    endmodule
