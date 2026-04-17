; Recursive stress test for the interpreter's function-call machinery.
; This performs 5000 self-recursive calls by counting down to zero and then
; rebuilding the result while unwinding.
;
; The old recursive Rust implementation could blow the host call stack on
; sufficiently deep recursion. The current iterative runtime call stack should
; handle this safely.
start main 0:
.entry:
r1 = call recurse 5000
call write r1
ret 0
end main

start recurse 1:
.entry:
r1 = icmp eq arg1 0 64
br r1 .base .step
.base:
ret 0
.step:
r2 = esub arg1 1 64
r3 = call recurse r2
r4 = eadd r3 1 64
ret r4
end recurse
