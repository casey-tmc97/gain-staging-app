# Phase 1 Completion Design

**Date:** 2026-06-23
**Status:** Approved
**Scope:** Completes all remaining Phase 1 checklist gaps before the Phase 2 DSP phase begins.

---

## Goal

Close the seven gaps identified in `docs/Validation/GainStage_Phase1_Validation_Checklist.md` so Phase 1 exit criteria are fully met. No DSP logic is introduced — all implementation crates remain stubs.

---

## Global Constraints

- No DSP algorithm implementations — all new crates and functions are stubs
- No global mutable state in Rust
- All Rust `unsafe` requires a `// SAFETY:` comment
- No exceptions cross the FFI boundary
- `gain-standalone` and `gain-ara` may ONLY import `gain-api`; direct imports of internal crates (`segmentation`, `analysis`, `classification`, `gain_decision`, `gain_map`) from outside `gain-core` are prohibited (ADR-005)
- `gain-api` is the sole stable public surface of `gain-core`

---

## Architecture

```
gain-standalone (Tauri)  ─────────────────────────────────────┐
                                                               ↓
gain-ara (C++) → ffi (C ABI) ─────────────────────────→ gain-api
                                                               ↓
                                          gain_decision, gain_map, (internals)
```

`gain-api` is the new public façade crate. Every consumer outside `gain-core` touches only this crate. The FFI crate and the Tauri standalone are both consumers of `gain-api`; they know nothing about the internal crate structure.

---

## Item 1 — `RegionType::EnvelopeControlled` rename

**Files:** `gain-core/crates/gain_map/src/lib.rs`, `gain-core/crates/ffi/src/lib.rs`, `gain-core/crates/ffi/include/gain_stage_ffi.h`

Rename the `Envelope` variant to `EnvelopeControlled` to match the checklist and domain terminology.

### Rust enum (gain_map)
```rust
pub enum RegionType {
    Stable,
    Transient,
    EnvelopeControlled,
    Mixed,
}
```

### FFI mapping (ffi/src/lib.rs)
```rust
RegionType::EnvelopeControlled => 2u8,
```

### C header (gain_stage_ffi.h)
```c
#define GAIN_STAGE_REGION_ENVELOPE_CONTROLLED 2
```
(replaces `GAIN_STAGE_REGION_ENVELOPE`)

All three tests in `gain_map` that reference `RegionType::Envelope` must be updated to `RegionType::EnvelopeControlled`.

---

## Item 2 — `version: u32` on `GainRecommendationMap`

**Files:** `gain-core/crates/gain_map/src/lib.rs`, `gain-core/crates/ffi/src/lib.rs`, `gain-core/crates/ffi/include/gain_stage_ffi.h`

### Rust struct
```rust
#[derive(Debug, Clone)]
pub struct GainRecommendationMap {
    pub version: u32,
    pub regions: Vec<GainRegion>,
}

impl Default for GainRecommendationMap {
    fn default() -> Self {
        Self { version: 1, regions: Vec::new() }
    }
}
```

### New FFI function
```rust
#[no_mangle]
pub extern "C" fn gain_stage_map_version(map: *const GainStageMap) -> u32 {
    if map.is_null() { return 0; }
    // SAFETY: map is non-null and was returned by gain_stage_analyze
    unsafe { (*map).0.version }
}
```

### C header addition
```c
/* Return the schema version of the map. Returns 0 if map is NULL. */
uint32_t gain_stage_map_version(const GainStageMap* map);
```

Existing test `gain_recommendation_map_default_is_empty` must be updated to also assert `map.version == 1`.

---

## Item 3 — `gain-api` public façade crate

**Files:** `gain-core/crates/gain-api/Cargo.toml`, `gain-core/crates/gain-api/src/lib.rs`; `gain-core/Cargo.toml` (add member)

### Purpose
Single stable entry point for all consumers outside `gain-core`. Internal crate topology may change freely without affecting callers.

### Cargo.toml
```toml
[package]
name = "gain-api"
version = "0.1.0"
edition = "2021"

[dependencies]
gain_map    = { path = "../gain_map" }
gain_decision = { path = "../gain_decision" }
```

### Public API (stub)
```rust
pub use gain_map::{GainRecommendationMap, GainRegion, RegionType};

#[derive(Debug)]
pub enum GainError {
    FileNotFound(String),
    UnsupportedFormat(String),
    AnalysisFailed(String),
}

/// Analyze an audio file and return a GainRecommendationMap.
/// Stub: returns an empty map with version = 1.
pub fn analyze_file(path: &std::path::Path) -> Result<GainRecommendationMap, GainError> {
    let _ = path;
    Ok(GainRecommendationMap::default())
}
```

### Workspace update
Add `"crates/gain-api"` to the `members` list in `gain-core/Cargo.toml`.

---

## Item 4 — Update `ffi` dependency to use `gain-api`

**Files:** `gain-core/crates/ffi/Cargo.toml`

Replace direct dependency on `gain_map` with `gain-api`:

```toml
[dependencies]
gain-api = { path = "../gain-api" }
```

Update `ffi/src/lib.rs` imports:
```rust
use gain_api::{GainRecommendationMap, RegionType};
```

This enforces that even the FFI crate goes through the public API surface.

---

## Item 5 — `gain-standalone/` Tauri skeleton

**Location:** `gain-standalone/` at the repo root (own Cargo workspace, separate from `gain-core`)

### Structure
```
gain-standalone/
  Cargo.toml              (workspace)
  src-tauri/
    Cargo.toml            (tauri app crate — depends on gain-api only)
    build.rs
    tauri.conf.json
    src/
      main.rs
      commands/
        mod.rs
        file.rs           (import_file stub)
        analyze.rs        (analyze stub)
        version.rs        (get_version stub)
  index.html              (minimal placeholder frontend)
```

### src-tauri/Cargo.toml dependencies
```toml
[dependencies]
tauri  = { version = "2", features = [] }
gain-api = { path = "../../gain-core/crates/gain-api" }
serde  = { version = "1", features = ["derive"] }
serde_json = "1"

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

### Three stub Tauri commands

**`import_file(path: String) -> Result<String, String>`**
Validates that the path exists and has an audio extension (`.wav`, `.aiff`, `.mp3`). Returns the canonical path. Does not read audio data yet.

**`analyze(path: String) -> Result<GainMapDto, String>`**
Calls `gain_api::analyze_file(&path)`, converts result to a serializable DTO, returns it. At scaffold level the result is always an empty map with version = 1.

**`get_version() -> u32`**
Returns the `gain-api` GainMap schema version (hardcoded `1` at scaffold level).

### GainMapDto (serde-serializable mirror of GainRecommendationMap)
```rust
#[derive(serde::Serialize)]
pub struct GainMapDto {
    pub version: u32,
    pub regions: Vec<GainRegionDto>,
}

#[derive(serde::Serialize)]
pub struct GainRegionDto {
    pub start_time: f64,
    pub end_time: f64,
    pub gain_db: f32,
    pub confidence: f32,
    pub region_type: String,
    pub reason: String,
}
```

### tauri.conf.json (minimal)
```json
{
  "productName": "Gain Stage",
  "version": "0.1.0",
  "identifier": "org.gainstage.app",
  "build": { "frontendDist": "../" },
  "app": {
    "windows": [{ "title": "Gain Stage", "width": 1200, "height": 800 }]
  },
  "bundle": { "active": false }
}
```

### index.html (placeholder)
```html
<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><title>Gain Stage</title></head>
<body><p>Gain Stage — scaffold placeholder</p></body>
</html>
```

### .gitignore addition
```
gain-standalone/src-tauri/target/
```

### Compilation target for Phase 1

`cargo build` in `gain-standalone/src-tauri/` must succeed as a Rust binary build. `tauri build` (full app bundle) is deferred — it requires platform-specific tooling and is not part of Phase 1 exit criteria.

---

## Item 6 — `test-assets/` directory

**Files:** `test-assets/README.md`, `test-assets/.gitkeep`

```markdown
# Test Assets

Reference audio files used by integration tests.

## Required files (to be added before Phase 2 integration tests)

- `sine_440hz_-18dBFS_5s.wav` — stable tone, known loudness
- `kick_transient.wav` — sharp transient for transient detection tests
- `envelope_swell.wav` — slow swell for envelope detection tests
- `mixed_content.wav` — composite content for classification tests

## Format

WAV, 44100 Hz, 32-bit float, mono. Files must be royalty-free or
generated synthetically. Do not commit large binary files — add them
to `.gitignore` and fetch via a test fixture script (to be defined).
```

---

## Item 7 — Documentation stubs

All files in `docs/`. Each stub contains the correct section structure; content sections are marked `<!-- TODO: ... -->` with a one-line description of what belongs there.

| File | Sections |
|---|---|
| `PRD.md` | Problem Statement, Target Users, Goals, Non-Goals, Success Metrics, Product Layers (Core / Standalone / ARA), Open Questions |
| `GainDecisionModel.md` | Overview, Region Classification Rules, Gain Recommendation Formula, Confidence Scoring, Edge Cases, References |
| `StandaloneSpec.md` | Overview, File Import Flow, Analysis Flow, Waveform Display, Gain Map Visualization, Export, Error Handling |
| `ARASpec.md` | Overview, Host Integration Lifecycle, Audio Source Events, Gain Map Display in DAW Timeline, Session Management, Error Handling |
| `CodingStandards.md` | Rust Standards, C++ Standards, FFI Rules, Test Requirements, Commit Convention, Forbidden Patterns |
| `ArchitectureDecisionLog.md` | ADR template, ADR-001 through ADR-005 (pre-populated) |

### ADR-001 through ADR-005 (pre-populated in ArchitectureDecisionLog.md)

| ADR | Decision |
|---|---|
| ADR-001 | Rust for core engine (safety, performance, no GC) |
| ADR-002 | C ABI FFI with opaque handle pattern (no object sharing, no exceptions across boundary) |
| ADR-003 | Tauri for standalone app (Rust backend, avoids Electron overhead) |
| ADR-004 | ARA protocol for DAW integration (non-destructive, host-managed audio) |
| ADR-005 | `gain-api` as sole public surface — standalone and ARA plugin may never import internal engine crates directly |

---

## Success Criteria

After this work, the Phase 1 checklist must show:

- [x] `/gain-core` present with all 8 crates (7 original + `gain-api`)
- [x] `/gain-standalone` present with Tauri skeleton that compiles
- [x] `/gain-ara` present (existing)
- [x] `/docs` with all required documents
- [x] `/test-assets` present
- [x] `GainRecommendationMap.version: u32` field exists
- [x] `RegionType::EnvelopeControlled` variant exists
- [x] gain-api accessible via FFI
- [x] ADR-005 documented
- [x] Section 8 deferred items remain absent
