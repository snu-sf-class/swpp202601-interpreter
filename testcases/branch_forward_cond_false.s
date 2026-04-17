; Conditional forward false branch: base 30 -> 45 with x1.5 scaling.
start main 0:
.entry:
r1 = xor 0 0 64
br r1 .else .later
.else:
ret 0
.later:
ret 0
end main
