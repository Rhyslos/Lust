# Benchmark: components every tick

A Lust game (`game.lst`) with 2,000 movers doing vector and rotation maths every tick and sending
events to each other, run the way Solis runs components: 600 ticks in Rust (the library
`lust build --library` makes) and in Luau (the Lua `lust::to_lua` makes), with Luau's native
code on and off.

```
lust build bench/game.lst --library
cargo run --release --manifest-path bench/Cargo.toml
```

It prints how long a tick takes each way, and stops with a failure if Rust and Luau end with
different values in any mover, down to the last bit. CI runs it on every pull request.

Results on the machine step 6 was made on (a tick, 2,000 movers):

| | Rust | Luau, native code | Luau, interpreted |
|---|---|---|---|
| Before step 6 (alpha 32) | 0.20 ms | 12.2 ms | 13.5 ms |
| After step 6 (alpha 33) | 0.14 ms | 5.1 ms | 6.2 ms |
| Luau vectors (alpha 34) | 0.14 ms | 3.4 ms | 3.8 ms |

The times move by about 10% from run to run.
