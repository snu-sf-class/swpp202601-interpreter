start main 0:
.entry:
r1 = udiv 8 2 64
r2 = eadd r1 2 64
call write r2
ret 0
end main
