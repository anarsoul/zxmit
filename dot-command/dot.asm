    device ZXSPECTRUM48
    org #2000

text
    include "modules/version.asm"
    jp start
ver db "zxmit v. ", VERSION_STRING, 13
    db "(c) 2022 Alex Nihirash", 13
    db "(c) 2025 Vasily Khoruzhick", 13, 13, 0

    include "modules/display.asm"
    include "modules/wifi.asm"
    include "modules/esxdos.asm"

    IFDEF UNO
    include "drivers/zxuno.asm"
    ENDIF

    IFDEF AY
    include "drivers/ay.asm"
    ENDIF

start:
    printMsg ver
    call Uart.init
    call Wifi.init
    printMsg msg_my_ip
    printMsg Wifi.ipAddr
    printMsg new_line
    di
    jp Wifi.recvWithFilename
msg_my_ip db "Device IP: ", 0
new_line db 13, "Listening port: 6144", 13, 0

buffer = #8000
    savebin "zxmit", text, $ - text
