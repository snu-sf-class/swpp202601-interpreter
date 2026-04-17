; Repeated heap loads from the same byte.
; Same pattern as heat_repeat_stack_loads.s, but heap base cost is larger.
start main 0:
.entry:
r1 = malloc 8
r2 = load 1 r1
r3 = load 1 r1
r4 = load 1 r1
call write r4
ret 0
end main
