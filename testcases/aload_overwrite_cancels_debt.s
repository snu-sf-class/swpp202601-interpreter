start main 0:
.entry:
r1 = malloc 8
r2 = aload 1 r1
r2 = eadd 41 1 64
call write r2
ret 0
end main
