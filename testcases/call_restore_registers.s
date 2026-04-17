; Callee may clobber general registers and sp internally,
; but caller-visible values must be restored after the call returns.
start main 0:
.entry:
r3 = oadd 2 3 64
call clobber
assert_eq r3 5
assert_eq sp 102400
call write r3
ret 0
end main

start clobber 0:
.entry:
r3 = oadd 8 1 64
sp = esub sp 8 64
ret 0
end clobber
