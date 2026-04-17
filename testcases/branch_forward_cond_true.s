; Conditional forward true branch: base 90 -> 135 with x1.5 scaling.
start main 0:
.entry:
r1 = oadd 0 1 64
br r1 .later .else
.else:
ret 0
.later:
ret 0
end main
