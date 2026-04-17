; Neighbor heating should stop at heap allocation boundaries.
; The second load touches the next malloc block, so it should not inherit heat
; from the 8-byte access to the first block.
start main 0:
.entry:
r1 = malloc 8
r2 = malloc 8
r3 = load 8 r1
r4 = load 1 r2
call write r4
ret 0
end main
