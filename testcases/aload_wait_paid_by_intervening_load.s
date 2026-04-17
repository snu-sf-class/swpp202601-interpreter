start main 0:
.entry:
r1 = malloc 8
r2 = aload 1 r1
r3 = load 1 r1
call write r2
ret 0
end main
