; A 1-byte heap load only heats its own sector.
; A later 8-byte load from the neighboring sector may heat r1+8, but it should
; not pay heat from r1+8 because its directly accessed sector is r1.
start main 0:
.entry:
r1 = malloc 16
r5 = eadd r1 8 64
r2 = load 1 r5
r3 = load 8 r1
ret 0
end main
