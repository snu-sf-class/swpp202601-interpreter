; After returning from a call, the next branch should still use the caller block
; when deciding whether the jump is forward.
start main 0:
.entry:
call noop
br .later
.later:
ret 0
end main

start noop 0:
.entry:
ret 0
end noop
