; FMA cancellation removes the deferred mul cost, but the fused add's elapsed
; cost should still resolve every outstanding async-load debt in parallel.
start main 0:
.entry:
r1 = aload 1 0
r2 = aload 1 1
r3 = mul 2 3 64
r4 = eadd r3 2 64
call write r2
ret 0
end main
