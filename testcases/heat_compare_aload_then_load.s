; Difference-focused case:
; aload heats memory first, then ordinary loads revisit the same hot byte.
;
; This is useful because it combines:
; - heat created by aload issue time
; - later synchronous loads to the same location
; - async debt waiting on the final use
;
; The exact final cost depends on the cooldown rule, so compare implementations.
start main 0:
.entry:
r1 = malloc 8
r2 = aload 1 r1
r3 = load 1 r1
r4 = load 1 r1
call write r2
ret 0
end main
