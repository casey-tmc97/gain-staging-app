# Gain Stage App

Perceptual audio analysis engine that generates a Gain Recommendation Map (GRM).

## Architecture

- `rust-core/` — Rust workspace: all audio analysis, segmentation, gain decisions
- `ara-plugin/` — C++ ARA plugin: DAW integration only, no DSP
- FFI bridge: `rust-core/crates/ffi/` exposes C ABI; `ara-plugin/src/GainMapBridge` consumes it

## Building

### Rust core

```powershell
cd rust-core
cargo build --workspace
```

### C++ ARA plugin

```powershell
cd ara-plugin
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build
```

## Status

Scaffold phase — stub implementations only.
