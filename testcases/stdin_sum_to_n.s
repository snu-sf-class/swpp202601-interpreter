;; Program to compute the sum from 1 to n
start main 0:
.entry:
r1 = call read
r5 = malloc 8
r9 = esub sp 8 64
r7 = aload 8 r9
r8 = eadd r7 r7 64
r2 = eadd 4 4 64
r2 = xor r2 r2 64
r3 = oadd 0 1 64
r6 = oadd 0 1 64
br .loop
.loop:
r4 = icmp ugt r3 r1 64
br r4 .end .body
.body:
r2 = eadd r2 r3 64
r3 = eadd r3 r6 64
br .loop
.end:
call write r2
free r5
ret 0
end main