; Fully-resolved async-load debts should be removed without disturbing the
; iteration over the rest of the pending debts.
start main 0:
.entry:
r1 = aload 1 0
r2 = aload 1 1
store 1 7 2
store 1 7 3
call write r2
ret 0
end main
