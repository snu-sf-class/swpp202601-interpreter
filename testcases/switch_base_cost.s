; Switch should keep base cost 60 and should not use forward-jump scaling.
start main 0:
.entry:
switch 0 1 .case .default
.case:
ret 0
.default:
ret 0
end main
