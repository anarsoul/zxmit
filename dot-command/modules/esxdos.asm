    module EsxDOS

ESX_GETSETDRV = #89
ESX_EXEC = #8F
ESX_FOPEN = #9A
ESX_FCLOSE = #9B
ESX_FSYNC = #9C
ESX_FWRITE = #9E


FMODE_CREATE = #0E

CMD_BUFF = 23512

prepareFile:
    push hl
    xor a
    rst #8
    db ESX_GETSETDRV

    pop hl
    ld b, FMODE_CREATE
    rst #8 
    db ESX_FOPEN
    jp c, .err

    ld (fhandle), a
    ret
.err
    pop ix
    ret

;; BC - chunk size
writeChunk:
    ld hl, buffer
    ld a, (fhandle)
    rst #8 : db ESX_FWRITE
    ret

close:
    ld a, (fhandle)
    rst #8 : db ESX_FSYNC

    ld a, (fhandle)
    rst #8 : db ESX_FCLOSE

    ei
    ret

fhandle db 0
    endmodule
