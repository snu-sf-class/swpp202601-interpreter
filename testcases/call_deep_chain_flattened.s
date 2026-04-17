; A deeper nested call chain to exercise the interpreter's explicit runtime call stack.
; Each function forwards to the next one and returns the result.
start main 0:
.entry:
r1 = call f1
call write r1
ret 0
end main

start f1 0:
.entry:
r1 = call f2
ret r1
end f1

start f2 0:
.entry:
r1 = call f3
ret r1
end f2

start f3 0:
.entry:
r1 = call f4
ret r1
end f3

start f4 0:
.entry:
r1 = call f5
ret r1
end f4

start f5 0:
.entry:
r1 = call f6
ret r1
end f5

start f6 0:
.entry:
r1 = call f7
ret r1
end f6

start f7 0:
.entry:
r1 = call f8
ret r1
end f7

start f8 0:
.entry:
r1 = call f9
ret r1
end f8

start f9 0:
.entry:
r1 = call f10
ret r1
end f9

start f10 0:
.entry:
r1 = call f11
ret r1
end f10

start f11 0:
.entry:
r1 = call f12
ret r1
end f11

start f12 0:
.entry:
r1 = call f13
ret r1
end f12

start f13 0:
.entry:
r1 = call f14
ret r1
end f13

start f14 0:
.entry:
r1 = call f15
ret r1
end f14

start f15 0:
.entry:
r1 = call f16
ret r1
end f15

start f16 0:
.entry:
r1 = call f17
ret r1
end f16

start f17 0:
.entry:
r1 = call f18
ret r1
end f17

start f18 0:
.entry:
r1 = call f19
ret r1
end f18

start f19 0:
.entry:
r1 = call f20
ret r1
end f19

start f20 0:
.entry:
r1 = call f21
ret r1
end f20

start f21 0:
.entry:
r1 = call f22
ret r1
end f21

start f22 0:
.entry:
r1 = call f23
ret r1
end f22

start f23 0:
.entry:
r1 = call f24
ret r1
end f23

start f24 0:
.entry:
r1 = call f25
ret r1
end f24

start f25 0:
.entry:
r1 = call f26
ret r1
end f25

start f26 0:
.entry:
r1 = call f27
ret r1
end f26

start f27 0:
.entry:
r1 = call f28
ret r1
end f27

start f28 0:
.entry:
r1 = call f29
ret r1
end f28

start f29 0:
.entry:
r1 = call f30
ret r1
end f29

start f30 0:
.entry:
r1 = call f31
ret r1
end f30

start f31 0:
.entry:
r1 = call f32
ret r1
end f31

start f32 0:
.entry:
ret 123
end f32
