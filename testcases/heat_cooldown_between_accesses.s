; Compare this with heat_repeat_stack_loads.s.
; With stack heat disabled, the inserted eadd instructions should not change the cost.
start main 0:
.entry:
r1 = esub sp 8 64
r2 = load 1 r1
r3 = eadd 1 1 64
r4 = eadd 1 1 64
r5 = eadd 1 1 64
r6 = load 1 r1
call write r6
ret 0
end main
