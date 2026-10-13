# Measurements

Measured rather than recalled, so later changes can be compared with them.

| What | Command | Result | When |
| --- | --- | --- | --- |
| Toolchain | `rustc --version; wasm-bindgen --version` | rustc 1.99.0; wasm-bindgen 0.2.127 | 12 Oct 2026 |
| Core tests | `cargo test --workspace` | 48 unit tests, 12 boundary tests, 1 campaign probe: all pass | 12 Oct 2026 |
| Campaign pacing | `cargo test --release --test campaign -- --nocapture` | fights per stage for five simulated players keeping the starters: 48 to 56 in total; most stages 1 to 3, the Kennel (stage 14) 12 to 19 | 12 Oct 2026 |
| Release WebAssembly | `./scripts/build.sh` | about 400 KB | 12 Oct 2026 |
