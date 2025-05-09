    device ZXSPECTRUM48
    org #2000

text
    include "modules/version.asm"
    jp start
ver db "zxmit v", VERSION_STRING, 13
    db "(c) 2022 Alex Nihirash", 13
    db "(c) 2025 Vasily Khoruzhick", 13, 13, 0

    include "modules/display.asm"
    include "modules/wifi.asm"
    include "modules/esxdos.asm"
    include "modules/dzx0_standard.asm"

    IFDEF UNO
    include "drivers/zxuno.asm"
    ENDIF

    IFDEF AY
    include "drivers/ay.asm"
    ENDIF

    IFDEF NEXT
    include "drivers/next.asm"
    ENDIF

start:
    call Display.cls
    printMsg ver
    call Uart.init
    call Wifi.init
    printMsg msg_my_ip
    printMsg Wifi.ipAddr
    printMsg new_line
    jp Wifi.recv
msg_my_ip db "Device IP: ", 0
new_line db 13, "Listening port: 6144", 13, 0

    savebin "zxmit", text, $ - text

recv_buffer = $
data_buffer = ($ + 1024 + 32)
data_end = data_buffer + 4096

    assert data_end < 16384
