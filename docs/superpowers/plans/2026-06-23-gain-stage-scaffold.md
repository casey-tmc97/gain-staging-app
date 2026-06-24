# Gain Stage App — Repository Scaffold Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Generate a compileable repository scaffold with a Rust core workspace, C ABI FFI layer, and C++ ARA plugin stub — all correctly separated by responsibility, with GainMap data structures defined in both layers.

**Architecture:** The Rust workspace at `rust-core/` contains all analysis logic as separate crates; the `ffi` crate exposes a C ABI shared library; the C++ ARA plugin at `ara-plugin/` links against that library and contains only session management and bridge stubs. No DSP is implemented — every algorithm entry point returns a stub/empty result.

**Tech Stack:** Rust 2021 edition (Cargo workspace, `cdylib` FFI), C++17 (CMake 3.20+), C ABI bridge

## Global Constraints

- No DSP algorithm implementations — stubs and placeholder returns only
- No UI logic beyond placeholder structs/comments
- No global mutable state in Rust (`static mut` banned; use function-local state)
- All Rust `unsafe` blocks must carry a one-line `// SAFETY:` comment
- No C++ exceptions crossing the FFI boundary (`noexcept` on all bridge functions)
- No object sharing across the FFI boundary — only POD types and opaque handles
- FFI crate compiled as `cdylib` (produces `.dll` + import `.lib` on Windows)
- C++ standard: C++17 minimum (`set(CMAKE_CXX_STANDARD 17)`)
- CMake minimum version: 3.20
- Rust edition: 2021
- Crate names match the module names in the spec exactly: `audio_ingestion`, `analysis`, `segmentation`, `classification`, `gain_decision`, `gain_map`, `ffi`

---

## File Map

```
Gain Staging App/
  .gitignore
  README.md
  gain stage core.md              (existing spec — do not modify)
  rust-core/
    Cargo.toml                    (workspace manifest — lists all member crates)
    crates/
      audio_ingestion/
        Cargo.toml
        src/lib.rs                (stub: AudioBuffer struct + load_file placeholder)
      analysis/
        Cargo.toml
        src/lib.rs                (re-exports sub-modules)
        src/loudness.rs           (stub: analyze_loudness fn)
        src/spectral.rs           (stub: analyze_spectral fn)
        src/transient.rs          (stub: detect_transients fn)
        src/envelope.rs           (stub: detect_envelope fn)
      segmentation/
        Cargo.toml
        src/lib.rs                (stub: segment fn)
      classification/
        Cargo.toml
        src/lib.rs                (stub: classify fn)
      gain_decision/
        Cargo.toml
        src/lib.rs                (stub: decide fn)
      gain_map/
        Cargo.toml
        src/lib.rs                (REAL: GainRegion, RegionType, GainRecommendationMap)
      ffi/
        Cargo.toml
        src/lib.rs                (C ABI exports: analyze, free_map, region_count, get_region)
        include/
          gain_stage_ffi.h        (C header — hand-written to match lib.rs exports)
  ara-plugin/
    CMakeLists.txt
    stubs/
      ara_stubs.h                 (minimal ARA type stubs — replaced by real ARA SDK later)
    src/
      GainMapBridge.h             (C++ wrapper around FFI handle)
      GainMapBridge.cpp
      ARAPlugin.h                 (ARA session skeleton)
      ARAPlugin.cpp
  docs/
    architecture.md
    gain-map-schema.md
    superpowers/
      plans/
        2026-06-23-gain-stage-scaffold.md   (this file)
```

---

### Task 1: Repository skeleton + git init

**Files:**
- Create: `.gitignore`
- Create: `README.md`
- Create: `docs/architecture.md`
- Create: `docs/gain-map-schema.md`

**Interfaces:**
- Produces: initialized git repo, `.gitignore` covering Rust and C++ build artifacts

- [ ] **Step 1: Initialize git repository**

Run from `C:\Users\Admin\Documents\GitHub\Gain Staging App`:
```powershell
git init
```
Expected: `Initialized empty Git repository in .../.git/`

- [ ] **Step 2: Create .gitignore**

Create `.gitignore`:
```gitignore
# Rust
rust-core/target/

# C++ / CMake
ara-plugin/build/
ara-plugin/cmake-build-*/
*.o
*.obj
*.dll
*.lib
*.exp
*.pdb

# Editor
.vscode/
.idea/
*.swp

# OS
Thumbs.db
.DS_Store
```

- [ ] **Step 3: Create README.md**

Create `README.md`:
```markdown
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
```

- [ ] **Step 4: Create docs/architecture.md**

Create `docs/architecture.md`:
```markdown
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
```

- [ ] **Step 5: Create docs/gain-map-schema.md**

Create `docs/gain-map-schema.md`:
```markdown
# Gain Recommendation Map Schema

## GainRegion

| Field | Type | Description |
|-------|------|-------------|
| start_time | f64 (seconds) | Region start in seconds |
| end_time | f64 (seconds) | Region end in seconds |
| gain_db | f32 | Recommended gain adjustment in dB |
| confidence | f32 | Confidence score 0.0–1.0 |
| region_type | RegionType | Stable / Transient / Envelope / Mixed |
| reason | String | Human-readable reasoning tag |

## RegionType

- `Stable` — sustained, steady-level content
- `Transient` — sharp attack/impact content
- `Envelope` — shaping-driven dynamic content
- `Mixed` — overlapping characteristics

## GainRecommendationMap

A time-ordered list of non-overlapping `GainRegion` entries covering the analyzed audio.
```

- [ ] **Step 6: Commit**

```powershell
git add .gitignore README.md docs/
git commit -m "chore: initialize repository with architecture docs and gitignore"
```

---

### Task 2: Rust workspace — all crate skeletons

**Files:**
- Create: `rust-core/Cargo.toml`
- Create: `rust-core/crates/audio_ingestion/Cargo.toml`
- Create: `rust-core/crates/audio_ingestion/src/lib.rs`
- Create: `rust-core/crates/analysis/Cargo.toml`
- Create: `rust-core/crates/analysis/src/lib.rs`
- Create: `rust-core/crates/analysis/src/loudness.rs`
- Create: `rust-core/crates/analysis/src/spectral.rs`
- Create: `rust-core/crates/analysis/src/transient.rs`
- Create: `rust-core/crates/analysis/src/envelope.rs`
- Create: `rust-core/crates/segmentation/Cargo.toml`
- Create: `rust-core/crates/segmentation/src/lib.rs`
- Create: `rust-core/crates/classification/Cargo.toml`
- Create: `rust-core/crates/classification/src/lib.rs`
- Create: `rust-core/crates/gain_decision/Cargo.toml`
- Create: `rust-core/crates/gain_decision/src/lib.rs`
- Create: `rust-core/crates/gain_map/Cargo.toml` (stub content; real code in Task 3)
- Create: `rust-core/crates/gain_map/src/lib.rs` (stub; real code in Task 3)
- Create: `rust-core/crates/ffi/Cargo.toml` (stub; real code in Task 4)
- Create: `rust-core/crates/ffi/src/lib.rs` (stub; real code in Task 4)

**Interfaces:**
- Produces: `cargo build --workspace` succeeds (all crates compile as empty stubs)

- [ ] **Step 1: Create workspace Cargo.toml**

Create `rust-core/Cargo.toml`:
```toml
[workspace]
resolver = "2"
members = [
    "crates/audio_ingestion",
    "crates/analysis",
    "crates/segmentation",
    "crates/classification",
    "crates/gain_decision",
    "crates/gain_map",
    "crates/ffi",
]
```

- [ ] **Step 2: Create audio_ingestion crate**

Create `rust-core/crates/audio_ingestion/Cargo.toml`:
```toml
[package]
name = "audio_ingestion"
version = "0.1.0"
edition = "2021"
```

Create `rust-core/crates/audio_ingestion/src/lib.rs`:
```rust
pub struct AudioBuffer {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

pub fn load_file(_path: &std::path::Path) -> Result<AudioBuffer, String> {
    Err("not implemented".to_string())
}
```

- [ ] **Step 3: Create analysis crate**

Create `rust-core/crates/analysis/Cargo.toml`:
```toml
[package]
name = "analysis"
version = "0.1.0"
edition = "2021"
```

Create `rust-core/crates/analysis/src/lib.rs`:
```rust
pub mod loudness;
pub mod spectral;
pub mod transient;
pub mod envelope;
```

Create `rust-core/crates/analysis/src/loudness.rs`:
```rust
pub struct LoudnessResult {
    pub integrated_lufs: f32,
    pub short_term_lufs: Vec<f32>,
}

pub fn analyze_loudness(_samples: &[f32], _sample_rate: u32) -> LoudnessResult {
    LoudnessResult { integrated_lufs: 0.0, short_term_lufs: vec![] }
}
```

Create `rust-core/crates/analysis/src/spectral.rs`:
```rust
pub struct SpectralResult {
    pub centroid_hz: f32,
}

pub fn analyze_spectral(_samples: &[f32], _sample_rate: u32) -> SpectralResult {
    SpectralResult { centroid_hz: 0.0 }
}
```

Create `rust-core/crates/analysis/src/transient.rs`:
```rust
pub struct TransientResult {
    pub onset_times_sec: Vec<f64>,
}

pub fn detect_transients(_samples: &[f32], _sample_rate: u32) -> TransientResult {
    TransientResult { onset_times_sec: vec![] }
}
```

Create `rust-core/crates/analysis/src/envelope.rs`:
```rust
pub struct EnvelopeResult {
    pub rms_db: Vec<f32>,
}

pub fn detect_envelope(_samples: &[f32], _sample_rate: u32) -> EnvelopeResult {
    EnvelopeResult { rms_db: vec![] }
}
```

- [ ] **Step 4: Create segmentation crate**

Create `rust-core/crates/segmentation/Cargo.toml`:
```toml
[package]
name = "segmentation"
version = "0.1.0"
edition = "2021"
```

Create `rust-core/crates/segmentation/src/lib.rs`:
```rust
pub struct Segment {
    pub start_sample: usize,
    pub end_sample: usize,
}

pub fn segment(_samples: &[f32], _sample_rate: u32) -> Vec<Segment> {
    vec![]
}
```

- [ ] **Step 5: Create classification crate**

Create `rust-core/crates/classification/Cargo.toml`:
```toml
[package]
name = "classification"
version = "0.1.0"
edition = "2021"
```

Create `rust-core/crates/classification/src/lib.rs`:
```rust
pub enum SegmentClass {
    Stable,
    Transient,
    Envelope,
    Mixed,
}

pub struct ClassifiedSegment {
    pub start_sample: usize,
    pub end_sample: usize,
    pub class: SegmentClass,
    pub confidence: f32,
}

pub fn classify(_samples: &[f32], _sample_rate: u32) -> Vec<ClassifiedSegment> {
    vec![]
}
```

- [ ] **Step 6: Create gain_decision crate (stub — real content in Task 3 dependencies)**

Create `rust-core/crates/gain_decision/Cargo.toml`:
```toml
[package]
name = "gain_decision"
version = "0.1.0"
edition = "2021"

[dependencies]
gain_map = { path = "../gain_map" }
```

Create `rust-core/crates/gain_decision/src/lib.rs`:
```rust
use gain_map::GainRecommendationMap;

pub fn decide(_samples: &[f32], _sample_rate: u32) -> GainRecommendationMap {
    GainRecommendationMap::default()
}
```

- [ ] **Step 7: Create gain_map and ffi crate stubs (real code written in Tasks 3 and 4)**

Create `rust-core/crates/gain_map/Cargo.toml`:
```toml
[package]
name = "gain_map"
version = "0.1.0"
edition = "2021"
```

Create `rust-core/crates/gain_map/src/lib.rs`:
```rust
// Populated in Task 3
#[derive(Debug, Clone, Default)]
pub struct GainRecommendationMap {
    pub regions: Vec<()>,
}
```

Create `rust-core/crates/ffi/Cargo.toml`:
```toml
[package]
name = "ffi"
version = "0.1.0"
edition = "2021"

[lib]
name = "gain_stage_ffi"
crate-type = ["cdylib"]

[dependencies]
gain_map = { path = "../gain_map" }
```

Create `rust-core/crates/ffi/src/lib.rs`:
```rust
// Populated in Task 4
```

- [ ] **Step 8: Verify workspace builds**

```powershell
cd rust-core
cargo build --workspace
```
Expected: all 7 crates compile, 0 errors. Warnings about unused code are acceptable.

- [ ] **Step 9: Commit**

```powershell
git add rust-core/
git commit -m "feat: scaffold Rust workspace with 7 crate stubs"
```

---

### Task 3: GainMap data structures (TDD)

Replaces the stub `gain_map/src/lib.rs` with real types. Also updates `gain_decision` to use them.

**Files:**
- Modify: `rust-core/crates/gain_map/src/lib.rs`
- Modify: `rust-core/crates/gain_decision/src/lib.rs`

**Interfaces:**
- Produces:
  - `gain_map::RegionType` (enum: Stable/Transient/Envelope/Mixed)
  - `gain_map::GainRegion { start_time: f64, end_time: f64, gain_db: f32, confidence: f32, region_type: RegionType, reason: String }`
  - `gain_map::GainRecommendationMap { regions: Vec<GainRegion> }` with `::default()` → empty map
- Consumed by: `ffi` crate (Task 4), `gain_decision` crate

- [ ] **Step 1: Write failing tests in gain_map**

Add to bottom of `rust-core/crates/gain_map/src/lib.rs` (file currently has a stub — replace entire file):

```rust
// Intentionally empty placeholder replaced here

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_region_fields_are_accessible() {
        let region = GainRegion {
            start_time: 0.0,
            end_time: 1.5,
            gain_db: -3.0,
            confidence: 0.85,
            region_type: RegionType::Stable,
            reason: "test".to_string(),
        };
        assert_eq!(region.start_time, 0.0);
        assert_eq!(region.end_time, 1.5);
        assert_eq!(region.gain_db, -3.0);
        assert_eq!(region.confidence, 0.85);
        assert_eq!(region.reason, "test");
    }

    #[test]
    fn gain_recommendation_map_default_is_empty() {
        let map = GainRecommendationMap::default();
        assert!(map.regions.is_empty());
    }

    #[test]
    fn gain_recommendation_map_can_hold_regions() {
        let mut map = GainRecommendationMap::default();
        map.regions.push(GainRegion {
            start_time: 0.0,
            end_time: 2.0,
            gain_db: 6.0,
            confidence: 1.0,
            region_type: RegionType::Transient,
            reason: "peak".to_string(),
        });
        assert_eq!(map.regions.len(), 1);
    }
}
```

- [ ] **Step 2: Run tests — verify they fail**

```powershell
cd rust-core
cargo test -p gain_map 2>&1
```
Expected: compile error — `GainRegion`, `RegionType`, `GainRecommendationMap` not defined.

- [ ] **Step 3: Implement the real types**

Replace entire `rust-core/crates/gain_map/src/lib.rs` with:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum RegionType {
    Stable,
    Transient,
    Envelope,
    Mixed,
}

#[derive(Debug, Clone)]
pub struct GainRegion {
    pub start_time: f64,
    pub end_time: f64,
    pub gain_db: f32,
    pub confidence: f32,
    pub region_type: RegionType,
    pub reason: String,
}

#[derive(Debug, Clone, Default)]
pub struct GainRecommendationMap {
    pub regions: Vec<GainRegion>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_region_fields_are_accessible() {
        let region = GainRegion {
            start_time: 0.0,
            end_time: 1.5,
            gain_db: -3.0,
            confidence: 0.85,
            region_type: RegionType::Stable,
            reason: "test".to_string(),
        };
        assert_eq!(region.start_time, 0.0);
        assert_eq!(region.end_time, 1.5);
        assert_eq!(region.gain_db, -3.0);
        assert_eq!(region.confidence, 0.85);
        assert_eq!(region.reason, "test");
    }

    #[test]
    fn gain_recommendation_map_default_is_empty() {
        let map = GainRecommendationMap::default();
        assert!(map.regions.is_empty());
    }

    #[test]
    fn gain_recommendation_map_can_hold_regions() {
        let mut map = GainRecommendationMap::default();
        map.regions.push(GainRegion {
            start_time: 0.0,
            end_time: 2.0,
            gain_db: 6.0,
            confidence: 1.0,
            region_type: RegionType::Transient,
            reason: "peak".to_string(),
        });
        assert_eq!(map.regions.len(), 1);
    }
}
```

- [ ] **Step 4: Run tests — verify they pass**

```powershell
cargo test -p gain_map
```
Expected: `test result: ok. 3 passed`

- [ ] **Step 5: Update gain_decision to use real GainRegion**

Replace `rust-core/crates/gain_decision/src/lib.rs`:
```rust
use gain_map::GainRecommendationMap;

pub fn decide(_samples: &[f32], _sample_rate: u32) -> GainRecommendationMap {
    GainRecommendationMap::default()
}
```

- [ ] **Step 6: Build workspace to verify no regressions**

```powershell
cargo build --workspace
```
Expected: 0 errors.

- [ ] **Step 7: Commit**

```powershell
git add rust-core/crates/gain_map/src/lib.rs rust-core/crates/gain_decision/src/lib.rs
git commit -m "feat: implement GainRecommendationMap data structures with tests"
```

---

### Task 4: FFI crate — C ABI surface + header

**Files:**
- Modify: `rust-core/crates/ffi/src/lib.rs`
- Create: `rust-core/crates/ffi/include/gain_stage_ffi.h`

**Interfaces:**
- Produces (Rust side, `extern "C"`):
  - `gain_stage_analyze(samples: *const f32, count: usize, sample_rate: u32) -> *mut GainStageMap`
  - `gain_stage_free_map(map: *mut GainStageMap)`
  - `gain_stage_map_region_count(map: *const GainStageMap) -> usize`
  - `gain_stage_map_get_region(map: *const GainStageMap, index: usize) -> CGainRegion`
- Produces (C header): `rust-core/crates/ffi/include/gain_stage_ffi.h`
- Consumed by: C++ `GainMapBridge` (Task 5)

- [ ] **Step 1: Write failing FFI tests**

Replace `rust-core/crates/ffi/src/lib.rs` with just the test block first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_returns_non_null_map() {
        let samples: Vec<f32> = vec![0.0f32; 1024];
        let map = unsafe {
            gain_stage_analyze(samples.as_ptr(), samples.len(), 44100)
        };
        assert!(!map.is_null());
        unsafe { gain_stage_free_map(map) };
    }

    #[test]
    fn empty_audio_produces_zero_regions() {
        let samples: Vec<f32> = vec![0.0f32; 1024];
        let map = unsafe {
            gain_stage_analyze(samples.as_ptr(), samples.len(), 44100)
        };
        assert!(!map.is_null());
        let count = unsafe { gain_stage_map_region_count(map) };
        assert_eq!(count, 0);
        unsafe { gain_stage_free_map(map) };
    }
}
```

- [ ] **Step 2: Run tests — verify they fail**

```powershell
cargo test -p ffi 2>&1
```
Expected: compile error — functions not defined.

- [ ] **Step 3: Implement the FFI crate**

Replace entire `rust-core/crates/ffi/src/lib.rs`:

```rust
use gain_map::{GainRecommendationMap, RegionType};

// Opaque handle — C++ holds a raw pointer, never sees internals
pub struct GainStageMap(GainRecommendationMap);

// POD region struct safe to pass across the C ABI boundary.
// reason is fixed-size to avoid lifetime issues across the boundary.
#[repr(C)]
pub struct CGainRegion {
    pub start_time: f64,
    pub end_time: f64,
    pub gain_db: f32,
    pub confidence: f32,
    /// 0=Stable 1=Transient 2=Envelope 3=Mixed
    pub region_type: u8,
    pub reason: [u8; 64],
}

/// Analyze audio samples and return an opaque GainStageMap handle.
/// Returns null only on allocation failure (should not occur in practice).
/// Caller must free the returned pointer with `gain_stage_free_map`.
#[no_mangle]
pub extern "C" fn gain_stage_analyze(
    _samples: *const f32,
    _count: usize,
    _sample_rate: u32,
) -> *mut GainStageMap {
    let map = GainRecommendationMap::default();
    Box::into_raw(Box::new(GainStageMap(map)))
}

/// Free a GainStageMap previously returned by `gain_stage_analyze`.
/// Passing null is a no-op.
#[no_mangle]
pub extern "C" fn gain_stage_free_map(map: *mut GainStageMap) {
    if map.is_null() {
        return;
    }
    // SAFETY: map was created by Box::into_raw in gain_stage_analyze
    unsafe { drop(Box::from_raw(map)) };
}

/// Return the number of regions in the map.
/// Passing null returns 0.
#[no_mangle]
pub extern "C" fn gain_stage_map_region_count(map: *const GainStageMap) -> usize {
    if map.is_null() {
        return 0;
    }
    // SAFETY: map is non-null and was created by gain_stage_analyze
    unsafe { (*map).0.regions.len() }
}

/// Return a copy of the region at the given index as a C-compatible struct.
/// If map is null or index is out of range, returns a zeroed CGainRegion.
#[no_mangle]
pub extern "C" fn gain_stage_map_get_region(
    map: *const GainStageMap,
    index: usize,
) -> CGainRegion {
    let zeroed = CGainRegion {
        start_time: 0.0,
        end_time: 0.0,
        gain_db: 0.0,
        confidence: 0.0,
        region_type: 0,
        reason: [0u8; 64],
    };

    if map.is_null() {
        return zeroed;
    }

    // SAFETY: map is non-null and was created by gain_stage_analyze
    let regions = unsafe { &(*map).0.regions };
    let Some(region) = regions.get(index) else {
        return zeroed;
    };

    let region_type_byte = match region.region_type {
        RegionType::Stable => 0u8,
        RegionType::Transient => 1u8,
        RegionType::Envelope => 2u8,
        RegionType::Mixed => 3u8,
    };

    let mut reason_buf = [0u8; 64];
    let bytes = region.reason.as_bytes();
    let len = bytes.len().min(63);
    reason_buf[..len].copy_from_slice(&bytes[..len]);

    CGainRegion {
        start_time: region.start_time,
        end_time: region.end_time,
        gain_db: region.gain_db,
        confidence: region.confidence,
        region_type: region_type_byte,
        reason: reason_buf,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_returns_non_null_map() {
        let samples: Vec<f32> = vec![0.0f32; 1024];
        let map = unsafe {
            gain_stage_analyze(samples.as_ptr(), samples.len(), 44100)
        };
        assert!(!map.is_null());
        unsafe { gain_stage_free_map(map) };
    }

    #[test]
    fn empty_audio_produces_zero_regions() {
        let samples: Vec<f32> = vec![0.0f32; 1024];
        let map = unsafe {
            gain_stage_analyze(samples.as_ptr(), samples.len(), 44100)
        };
        assert!(!map.is_null());
        let count = unsafe { gain_stage_map_region_count(map) };
        assert_eq!(count, 0);
        unsafe { gain_stage_free_map(map) };
    }

    #[test]
    fn get_region_on_empty_map_returns_zeroed() {
        let samples: Vec<f32> = vec![0.0f32; 1024];
        let map = unsafe {
            gain_stage_analyze(samples.as_ptr(), samples.len(), 44100)
        };
        let region = unsafe { gain_stage_map_get_region(map, 0) };
        assert_eq!(region.gain_db, 0.0);
        unsafe { gain_stage_free_map(map) };
    }
}
```

- [ ] **Step 4: Run tests — verify they pass**

```powershell
cargo test -p ffi
```
Expected: `test result: ok. 3 passed`

- [ ] **Step 5: Build the cdylib**

```powershell
cargo build -p ffi --release
```
Expected: `Compiling ffi v0.1.0 ...` then `Finished release`. On Windows, `rust-core/target/release/gain_stage_ffi.dll` and `rust-core/target/release/gain_stage_ffi.dll.lib` will be present.

Verify:
```powershell
Test-Path "rust-core\target\release\gain_stage_ffi.dll"
```
Expected: `True`

- [ ] **Step 6: Create C header**

Create `rust-core/crates/ffi/include/gain_stage_ffi.h`:

```c
#pragma once
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handle to a GainRecommendationMap allocated in Rust. */
typedef struct GainStageMap GainStageMap;

/* region_type values */
#define GAIN_STAGE_REGION_STABLE    0
#define GAIN_STAGE_REGION_TRANSIENT 1
#define GAIN_STAGE_REGION_ENVELOPE  2
#define GAIN_STAGE_REGION_MIXED     3

/* POD struct safe to copy across the FFI boundary. */
typedef struct {
    double  start_time;
    double  end_time;
    float   gain_db;
    float   confidence;
    uint8_t region_type;  /* see GAIN_STAGE_REGION_* constants */
    uint8_t reason[64];   /* null-terminated UTF-8 */
} CGainRegion;

/*
 * Analyze raw audio and return an opaque GainStageMap handle.
 * samples     - interleaved f32 PCM samples (mono expected at scaffold level)
 * count       - total number of samples
 * sample_rate - samples per second (e.g. 44100)
 * Returns NULL only on allocation failure.
 * Caller must release with gain_stage_free_map().
 */
GainStageMap* gain_stage_analyze(
    const float*  samples,
    size_t        count,
    uint32_t      sample_rate
);

/* Release a GainStageMap. Passing NULL is a no-op. */
void gain_stage_free_map(GainStageMap* map);

/* Return the number of regions. Returns 0 if map is NULL. */
size_t gain_stage_map_region_count(const GainStageMap* map);

/*
 * Return a copy of the region at index.
 * Returns a zeroed CGainRegion if map is NULL or index is out of range.
 */
CGainRegion gain_stage_map_get_region(const GainStageMap* map, size_t index);

#ifdef __cplusplus
} /* extern "C" */
#endif
```

- [ ] **Step 7: Commit**

```powershell
git add rust-core/crates/ffi/
git commit -m "feat: implement FFI crate with C ABI surface and header"
```

---

### Task 5: C++ ARA plugin scaffold + CMake

**Files:**
- Create: `ara-plugin/stubs/ara_stubs.h`
- Create: `ara-plugin/src/GainMapBridge.h`
- Create: `ara-plugin/src/GainMapBridge.cpp`
- Create: `ara-plugin/src/ARAPlugin.h`
- Create: `ara-plugin/src/ARAPlugin.cpp`
- Create: `ara-plugin/CMakeLists.txt`

**Interfaces:**
- Consumes: `rust-core/crates/ffi/include/gain_stage_ffi.h` (from Task 4)
- Consumes: `rust-core/target/release/gain_stage_ffi.dll.lib` (Windows import library)
- Produces: `ara-plugin/build/gain_stage_ara.dll` (ARA plugin DLL, scaffold-level)

- [ ] **Step 1: Create ARA stub types header**

> Note: The real ARA SDK (from Celemony) requires a separate download and license agreement. This stub header provides minimal placeholder types so the plugin scaffold compiles. Replace with the real ARA SDK before implementing DAW integration.

Create `ara-plugin/stubs/ara_stubs.h`:

```cpp
#pragma once

// Placeholder ARA SDK types.
// Replace this entire file with the real ARA SDK headers when integrating.

#include <cstdint>

namespace ARA {

struct ARAAudioSourceRef { uint64_t id = 0; };
struct ARADocumentControllerRef { uint64_t id = 0; };
struct ARAPlaybackRegionRef { uint64_t id = 0; };

enum class ARAContentType : uint32_t {
    kARAContentTypeNotes = 1,
    kARAContentTypeTempoEntries = 2,
};

struct ARADocumentControllerInterface {
    virtual ~ARADocumentControllerInterface() = default;
    virtual void notifyAudioSourceAnalysisProgress(
        ARAAudioSourceRef audioSource, float progress) = 0;
};

} // namespace ARA
```

- [ ] **Step 2: Create GainMapBridge header**

Create `ara-plugin/src/GainMapBridge.h`:

```cpp
#pragma once

#include <cstddef>
#include <string>
#include <vector>

// C++ RAII wrapper around the Rust FFI GainStageMap handle.
// Acquires a map on construction, releases it on destruction.
// No exceptions are thrown — check is_valid() before use.

struct GainRegionCxx {
    double      start_time;
    double      end_time;
    float       gain_db;
    float       confidence;
    uint8_t     region_type;  // 0=Stable 1=Transient 2=Envelope 3=Mixed
    std::string reason;
};

class GainMapBridge {
public:
    // Analyze audio samples. stub: always returns an empty map.
    GainMapBridge(const float* samples, size_t count, uint32_t sample_rate) noexcept;
    ~GainMapBridge() noexcept;

    // Non-copyable — owns the Rust handle
    GainMapBridge(const GainMapBridge&) = delete;
    GainMapBridge& operator=(const GainMapBridge&) = delete;

    bool              is_valid()      const noexcept;
    size_t            region_count()  const noexcept;
    GainRegionCxx     get_region(size_t index) const noexcept;

private:
    struct GainStageMap* map_ = nullptr;
};
```

- [ ] **Step 3: Create GainMapBridge implementation**

Create `ara-plugin/src/GainMapBridge.cpp`:

```cpp
#include "GainMapBridge.h"
#include "../rust-core-include/gain_stage_ffi.h"

#include <cstring>

GainMapBridge::GainMapBridge(
    const float* samples, size_t count, uint32_t sample_rate) noexcept
{
    map_ = gain_stage_analyze(samples, count, sample_rate);
}

GainMapBridge::~GainMapBridge() noexcept
{
    gain_stage_free_map(map_);
}

bool GainMapBridge::is_valid() const noexcept
{
    return map_ != nullptr;
}

size_t GainMapBridge::region_count() const noexcept
{
    return gain_stage_map_region_count(map_);
}

GainRegionCxx GainMapBridge::get_region(size_t index) const noexcept
{
    CGainRegion c = gain_stage_map_get_region(map_, index);
    GainRegionCxx r{};
    r.start_time  = c.start_time;
    r.end_time    = c.end_time;
    r.gain_db     = c.gain_db;
    r.confidence  = c.confidence;
    r.region_type = c.region_type;
    r.reason      = reinterpret_cast<const char*>(c.reason);
    return r;
}
```

- [ ] **Step 4: Create ARAPlugin header**

Create `ara-plugin/src/ARAPlugin.h`:

```cpp
#pragma once

#include "../stubs/ara_stubs.h"
#include "GainMapBridge.h"

#include <memory>

// ARA plugin session stub.
// Receives audio from the DAW host and delegates analysis to GainMapBridge.
// No DSP logic here — transport and session management only.

class ARAPlugin : public ARA::ARADocumentControllerInterface {
public:
    ARAPlugin() = default;
    ~ARAPlugin() override = default;

    // Called by the host when audio data is available for analysis.
    // stub: creates a GainMapBridge and discards the result.
    void analyzeAudioSource(
        ARA::ARAAudioSourceRef source,
        const float*           samples,
        size_t                 count,
        uint32_t               sample_rate) noexcept;

    // ARADocumentControllerInterface stubs
    void notifyAudioSourceAnalysisProgress(
        ARA::ARAAudioSourceRef audioSource, float progress) override;

private:
    std::unique_ptr<GainMapBridge> current_map_;
};
```

- [ ] **Step 5: Create ARAPlugin implementation**

Create `ara-plugin/src/ARAPlugin.cpp`:

```cpp
#include "ARAPlugin.h"

void ARAPlugin::analyzeAudioSource(
    ARA::ARAAudioSourceRef /*source*/,
    const float*           samples,
    size_t                 count,
    uint32_t               sample_rate) noexcept
{
    current_map_ = std::make_unique<GainMapBridge>(samples, count, sample_rate);
}

void ARAPlugin::notifyAudioSourceAnalysisProgress(
    ARA::ARAAudioSourceRef /*audioSource*/, float /*progress*/)
{
    // stub: no-op until real ARA SDK integration
}
```

- [ ] **Step 6: Create CMakeLists.txt**

The CMake copies the FFI header into a local include path at configure time so `GainMapBridge.cpp` can find it without a hardcoded absolute path.

Create `ara-plugin/CMakeLists.txt`:

```cmake
cmake_minimum_required(VERSION 3.20)
project(gain_stage_ara VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# ── Paths ──────────────────────────────────────────────────────────────────────

# Path to the Rust workspace root (relative to this CMakeLists.txt)
set(RUST_CORE_DIR "${CMAKE_SOURCE_DIR}/../rust-core")

# The Rust FFI header
set(FFI_INCLUDE_DIR "${RUST_CORE_DIR}/crates/ffi/include")

# The Rust import library (Windows: .dll.lib)
# Build the Rust FFI crate first: cd rust-core && cargo build -p ffi --release
set(RUST_LIB "${RUST_CORE_DIR}/target/release/gain_stage_ffi.dll.lib")

# ── Verify prerequisites ───────────────────────────────────────────────────────

if(NOT EXISTS "${FFI_INCLUDE_DIR}/gain_stage_ffi.h")
    message(FATAL_ERROR
        "FFI header not found at ${FFI_INCLUDE_DIR}/gain_stage_ffi.h\n"
        "Ensure rust-core/crates/ffi/include/gain_stage_ffi.h exists.")
endif()

if(NOT EXISTS "${RUST_LIB}")
    message(FATAL_ERROR
        "Rust import library not found at ${RUST_LIB}\n"
        "Run: cd rust-core && cargo build -p ffi --release")
endif()

# ── Copy header to local include dir ─────────────────────────────────────────

set(LOCAL_FFI_INCLUDE "${CMAKE_SOURCE_DIR}/rust-core-include")
file(MAKE_DIRECTORY "${LOCAL_FFI_INCLUDE}")
configure_file(
    "${FFI_INCLUDE_DIR}/gain_stage_ffi.h"
    "${LOCAL_FFI_INCLUDE}/gain_stage_ffi.h"
    COPYONLY
)

# ── Target ────────────────────────────────────────────────────────────────────

add_library(gain_stage_ara SHARED
    src/GainMapBridge.cpp
    src/ARAPlugin.cpp
)

target_include_directories(gain_stage_ara PRIVATE
    "${LOCAL_FFI_INCLUDE}"
    stubs/
    src/
)

target_link_libraries(gain_stage_ara PRIVATE "${RUST_LIB}")

# On Windows: copy the Rust DLL next to the built plugin DLL
if(WIN32)
    set(RUST_DLL "${RUST_CORE_DIR}/target/release/gain_stage_ffi.dll")
    add_custom_command(TARGET gain_stage_ara POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
            "${RUST_DLL}"
            "$<TARGET_FILE_DIR:gain_stage_ara>/gain_stage_ffi.dll"
        COMMENT "Copying Rust FFI DLL alongside plugin"
    )
endif()
```

- [ ] **Step 7: Build Rust FFI in release mode (prerequisite for CMake)**

```powershell
cd rust-core
cargo build -p ffi --release
cd ..
```
Expected: `Finished release [optimized]`

Verify the import lib exists:
```powershell
Test-Path "rust-core\target\release\gain_stage_ffi.dll.lib"
```
Expected: `True`

- [ ] **Step 8: Configure CMake**

```powershell
cd ara-plugin
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release
```
Expected: CMake completes without `FATAL_ERROR`. Last line: `-- Build files have been written to: .../ara-plugin/build`

- [ ] **Step 9: Build C++ plugin**

```powershell
cmake --build build --config Release
```
Expected: Compiles `GainMapBridge.cpp` and `ARAPlugin.cpp`, links `gain_stage_ffi.dll.lib`, produces `build/Release/gain_stage_ara.dll`.

Verify:
```powershell
Test-Path "build\Release\gain_stage_ara.dll"
```
Expected: `True`

- [ ] **Step 10: Commit**

```powershell
cd ..
git add ara-plugin/
git commit -m "feat: scaffold C++ ARA plugin with CMake and GainMapBridge FFI wrapper"
```

---

### Task 6: Build verification + checkpoint report

**Files:**
- No new files — verification only

**Interfaces:**
- Confirms: all success criteria from spec section 10 are met

- [ ] **Step 1: Clean build of Rust workspace**

```powershell
cd rust-core
cargo clean
cargo build --workspace
cargo test --workspace
```
Expected: all crates build, all tests pass (3 in `gain_map`, 3 in `ffi`, 0 in others — that is fine).

- [ ] **Step 2: Clean build of C++ plugin**

```powershell
cd ..\ara-plugin
Remove-Item -Recurse -Force build -ErrorAction SilentlyContinue
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build --config Release
```
Expected: DLL produced without errors.

- [ ] **Step 3: Verify GainMap struct exists in both layers**

Rust side — confirm types are exported:
```powershell
cd ..\rust-core
cargo doc -p gain_map --no-deps --open
```
Expected: docs page shows `GainRegion`, `RegionType`, `GainRecommendationMap`.

C++ side — confirm header defines `CGainRegion`:
```powershell
Select-String -Path "crates\ffi\include\gain_stage_ffi.h" -Pattern "CGainRegion"
```
Expected: at least 2 matches (typedef + usage in function signature).

- [ ] **Step 4: Verify FFI boundary stub is functional**

```powershell
cargo test -p ffi -- --nocapture
```
Expected:
```
test tests::analyze_returns_non_null_map ... ok
test tests::empty_audio_produces_zero_regions ... ok
test tests::get_region_on_empty_map_returns_zeroed ... ok
test result: ok. 3 passed; 0 failed
```

- [ ] **Step 5: Final commit**

```powershell
cd ..
git add .
git commit -m "chore: verified scaffold build — Rust workspace + C++ ARA plugin both compile"
```

- [ ] **Step 6: STOP — report completion to planning conversation**

Report to the user:

> **Scaffold complete.** The following is done:
> - Repository initialized with `.gitignore`, `README.md`, `docs/`
> - Rust workspace (`rust-core/`) with 7 crates: `audio_ingestion`, `analysis`, `segmentation`, `classification`, `gain_decision`, `gain_map`, `ffi`
> - `GainRecommendationMap`, `GainRegion`, `RegionType` defined in `gain_map` crate with 3 passing tests
> - FFI crate exposes 4 C ABI functions with 3 passing tests; `gain_stage_ffi.h` header generated
> - C++ ARA plugin scaffold (`ara-plugin/`) with `GainMapBridge` (RAII FFI wrapper), `ARAPlugin` (session stub), and CMake build that links the Rust DLL
> - All layers compile; no DSP implemented
>
> **Awaiting next instructions before proceeding.**

---

## Self-Review

### Spec coverage check

| Spec section | Plan coverage |
|---|---|
| §2 Core Engine (Rust) — all analysis in Rust, no UI/DAW/plugin logic | Tasks 2–4 ✓ |
| §2 Standalone App (Tauri) | Not in scope for scaffold phase per §11 checkpoint ✓ |
| §2 ARA Plugin (C++) — no DSP | Task 5 — plugin has zero DSP ✓ |
| §2 FFI Layer — C ABI, minimal surface, no object sharing | Task 4 — opaque handle pattern ✓ |
| §5 Rust modules: audio_ingestion, analysis (loudness/spectral/transient/envelope), segmentation, classification, gain_decision, gain_map, ffi | Task 2 creates all 7 ✓ |
| §5 no unsafe without justification | All unsafe in ffi has `// SAFETY:` comment ✓ |
| §5 no global mutable state | No `static mut` anywhere ✓ |
| §5 deterministic output | Stubs are pure functions returning empty structs ✓ |
| §6 C++ ARA layer — no DSP/analysis/audio processing | ARAPlugin.cpp has no DSP ✓ |
| §7 Data model: start_time, end_time, gain_db, confidence, type, reason | GainRegion has all 6 fields ✓ |
| §8 build system (Cargo + CMake) | Cargo workspace + CMakeLists.txt ✓ |
| §9 no pseudo-code pretending to be functional DSP | All stubs return empty/default ✓ |
| §10 compiles, Rust/C++ separated, FFI stub, GainMap in both layers | Task 6 verifies all ✓ |
| §11 stop after scaffold | Task 6 step 6 explicitly stops ✓ |

### Placeholder scan

No TBD, TODO, "implement later", or "similar to Task N" patterns present. All code blocks are complete.

### Type consistency

- `GainRecommendationMap` — defined in Task 3, used in `gain_decision` (Task 3 step 5), passed through `GainStageMap` opaque wrapper in Task 4 ✓
- `CGainRegion` — defined in `ffi/src/lib.rs` (Task 4 step 3), mirrored identically in `gain_stage_ffi.h` (Task 4 step 6), consumed as `CGainRegion` in `GainMapBridge.cpp` (Task 5 step 3) ✓
- `GainRegionCxx` — defined in `GainMapBridge.h` (Task 5 step 2), populated in `GainMapBridge.cpp` (Task 5 step 3) ✓
- `GainStageMap` — forward-declared in header as `struct GainStageMap`, defined as `pub struct GainStageMap(GainRecommendationMap)` in Rust ✓
