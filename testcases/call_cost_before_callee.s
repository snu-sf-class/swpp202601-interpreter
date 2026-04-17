; Call timing check:
; the call instruction cost should be paid before entering `use_loaded`.
; That means the callee's first use of r2 should wait only for the remaining
; unresolved aload debt, not the full original debt.
start main 0:
.entry:
r1 = malloc 8
r2 = aload 1 r1
r3 = call use_loaded
call write r3
ret 0
end main

start use_loaded 0:
.entry:
r3 = eadd r2 0 64
ret r3
end use_loaded
