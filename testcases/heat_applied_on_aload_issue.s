; Heat should be applied when aload is issued, even before the value is used.
; The second aload and the later load both observe the heated address.
start main 0:
.entry:
r1 = malloc 8
r2 = aload 1 r1
r3 = aload 1 r1
r4 = load 1 r1
call write r4
ret 0
end main
