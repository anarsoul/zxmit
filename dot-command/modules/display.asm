    module Display
putStr:
    ld a, (hl) : and a : ret z
    push hl
    rst #10
    pop hl
    inc hl
    jr putStr

cls:
    ld hl, 16384
    ld bc, 6144
1:
    ld (hl), 0
    inc hl
    dec bc
    ld a, b
    or c
    jr nz, 1b

    ld a, 22: rst #10
    ld a, 0: rst #10
    ld a, 0: rst #10
    ret

    endmodule

    macro printMsg ptr
    ld hl, ptr : call Display.putStr
    endm

