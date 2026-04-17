; Basic function-call sanity check:
; - arguments are passed into arg registers
; - return value is assigned to the destination register
start main 0:
.entry:
r1 = call add_one 7
call write r1
ret 0
end main

start add_one 1:
.entry:
r1 = eadd arg1 1 64
ret r1
end add_one
