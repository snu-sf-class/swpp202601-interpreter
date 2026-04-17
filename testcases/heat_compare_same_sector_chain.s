; Three consecutive loads to the same stack byte.
; Stack heat is disabled, so this stays a flat baseline instead of showing any
; heat-model divergence.
start main 0:
.entry:
r1 = esub sp 8 64
r2 = load 1 r1
r3 = load 1 r1
r4 = load 1 r1
call write r4
ret 0
end main
