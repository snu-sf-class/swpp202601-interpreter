start main 0:
.entry:
r1 = malloc 8
r2 = aload 1 r1
r3 = and r2 1 64
call write r3
ret 0
end main
