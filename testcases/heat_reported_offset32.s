; An 8-byte heap load should heat sectors up to four 8-byte sectors away.
; The later 1-byte load at r1+32 should therefore observe heat from the first load.
start main 0:
.entry:
r1 = malloc 40
r5 = eadd r1 32 64
r2 = load 8 r1
r3 = load 1 r5
ret 0
end main
