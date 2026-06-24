# Gain Stage App

Perceptual audio analysis engine that generates a Gain Recommendation Map (GRM).

## Architecture

- `gain-core/` — Rust workspace: all audio analysis, segmentation, gain decisions
- `gain-ara/` — C++ ARA plugin: DAW integration only, no DSP
- FFI bridge: `gain-core/crates/ffi/` exposes C ABI; `gain-ara/src/GainMapBridge` consumes it

## Building

### Rust core

```powershell
cd gain-core
cargo build --workspace
```

### C++ ARA plugin

```powershell
cd gain-ara
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build
```

## Status

Scaffold phase — stub implementations only.
