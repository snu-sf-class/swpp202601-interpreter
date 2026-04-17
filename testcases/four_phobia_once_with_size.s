; Both the address and the memory access size are 4 here.
; 4-phobia should still be charged only once for the whole instruction.
start main 0:
.entry:
r1 = load 4 4
call write r1
ret 0
end main
