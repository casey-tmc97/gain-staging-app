# Architecture

## Layers

| Layer | Technology | Responsibility |
|-------|-----------|----------------|
| Core Engine | Rust | Audio analysis, segmentation, gain decisions, GRM generation |
| Standalone App | Tauri + Rust | File I/O, waveform display, batch processing (future phase) |
| ARA Plugin | C++ | DAW host integration, timeline display (future phase) |
| FFI Bridge | C ABI | Rust ↔ C++ boundary — POD types and opaque handles only |

## Rust Crate Dependency Graph

```
ffi
 └── gain_decision
      ├── analysis
      │    └── (audio primitives)
      ├── classification
      │    └── segmentation
      │         └── audio_ingestion
      └── gain_map  ← shared data types (no deps)
```

## FFI Contract

- Rust exposes opaque `GainStageMap*` handle
- C++ acquires handle via `gain_stage_analyze()`
- C++ reads regions via `gain_stage_map_get_region()`
- C++ releases handle via `gain_stage_free_map()`
- No exceptions cross the boundary
- No Rust objects are shared directly
