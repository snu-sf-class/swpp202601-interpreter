The interpreter for Swpp 2026 class assembly programs written in Rust.



## Build
First, you need to install the [rustup](https://www.rust-lang.org/tools/install). This process will install `rustc` and `Cargo`.

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Now you have several options.

- `cargo run --release -- [Assembly File path]`: run the interpreter directly.

- `cargo build --release`: build the binary at `./target/release/main`.

- `cargo run --release --features="log" -- [Assembly File path]"`: run the interpreter with making simple log file `swpp-interpreter-basic.log`.

## Test

You can test the behaviors of various examples by following below command :

```
./test.sh          # test without logs
./test.sh --logs   # test with logs
```
