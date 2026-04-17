; Difference-focused case:
; a 2-byte load heats the direct and neighboring sectors,
; then we repeatedly hit the right neighbor.
;
; Under "no cooldown on touched sectors", the neighbor keeps the full newly added heat.
; Under "cool everything every turn", the neighbor loses instruction cost immediately.
start main 0:
.entry:
r1 = malloc 16
r5 = eadd r1 8 64
r2 = load 2 r1
r3 = load 1 r5
r4 = load 1 r5
call write r4
ret 0
end main
