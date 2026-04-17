; Nested call should not permanently overwrite the caller's arg registers.
; `outer` receives arg1 = 7, calls `inner 9`, then should still see arg1 = 7.
start main 0:
.entry:
r1 = call outer 7
call write r1
ret 0
end main

start outer 1:
.entry:
r1 = call inner 9
ret arg1
end outer

start inner 1:
.entry:
ret arg1
end inner
