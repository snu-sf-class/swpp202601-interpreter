; The first jump is forward, but the second jumps backward and should keep base cost 30.
start main 0:
.entry:
br .setup
.target:
ret 0
.setup:
br .target
end main
