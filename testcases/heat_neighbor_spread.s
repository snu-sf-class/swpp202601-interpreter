; A 2-byte load should heat the direct sector and its neighboring sectors.
; The following 1-byte load at r1+8 should therefore pay extra heat cost.
start main 0:
.entry:
r1 = malloc 16
r5 = eadd r1 8 64
r2 = load 2 r1
r3 = load 1 r5
call write r3
ret 0
end main
