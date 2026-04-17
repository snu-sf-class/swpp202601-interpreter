; Two separate <val> operands are both 4 here.
; 4-phobia should still be charged only once for the instruction.
start main 0:
.entry:
r1 = eadd 4 4 64
call write r1
ret 0
end main
