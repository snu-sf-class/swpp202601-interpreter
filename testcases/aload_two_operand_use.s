; Two different async-load results used by the same instruction should wait
; only for the remaining parallel debt, not for each debt independently.
start main 0:
.entry:
r1 = aload 1 0
r2 = aload 1 1
r3 = eadd r1 r2 64
call write r3
ret 0
end main
