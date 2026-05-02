; When a pending mul cannot fuse, the flushed mul cost and the following
; instruction cost should both resolve every outstanding async-load debt.
start main 0:
.entry:
r1 = aload 1 0
r2 = aload 1 1
r3 = mul 2 3 64
r4 = xor 0 0 64
call write r2
ret 0
end main
