# Phase 2 Design Spec
## Gain Stage App — DSP Pipeline Implementation

**Date:** 2026-06-24
**Status:** Approved
**Author:** casey-tmc97

---

## Scope

Phase 2 implements the real audio analysis and gain recommendation pipeline inside `gain-core`. Every internal crate that was a stub in Phase 1 gets a real implementation, except `segmentation` and `classification`, which remain stubs pending Phase 3.

**In scope:**
- WAV and AIFF decoding via symphonia
- Real Peak dBFS, RMS dBFS, and Crest Factor measurement
- Preset-based gain recommendation math (Peak and RMS targets)
- Single full-file `GainRegion` output (`RegionType::Stable`)
- Two-step public API: `analyze_file()` → `AnalysisResult`, `generate_recommendation()` → `GainRecommendationMap`
- Dedicated `gain-error` crate
- FFI file-path entry point and error reporting

**Explicitly deferred to Phase 3:**
- ITU-R BS.1770 K-weighted LUFS (LUFS is `MeasurementQuality::Placeholder` in Phase 2)
- True Peak (oversampling)
- Segmentation (multiple regions)
- Classification intelligence (CrestFactor heuristics)
- Per-region gain recommendations
- FLAC, CAF, Broadcast WAV
- `album-consistency` two-pass batch preset

---

## Goals

1. Validate the architecture and crate dependency graph end-to-end
2. Validate all public API contracts before UI work begins
3. Deliver correct gain recommendations for all Peak and RMS presets
4. Establish honest measurement quality metadata (no fake LUFS)

---

## New Crate: `gain-error`

A dedicated crate with no internal dependencies. All other crates import from it instead of defining their own error types.

```rust
pub enum GainError {
    FileNotFound {
        path: String,
    },
    UnsupportedFormat {
        format: String,
    },
    DecodeFailure {
        details: String,
    },
    InvalidAudio {
        details: String,
    },
    AnalysisFailure {
        details: String,
    },
    InternalError {
        details: String,
    },
}
```

Named struct variants are used throughout for self-documentation, easier serialization, and forward compatibility (new fields can be added without breaking match arms that use `..`).

`gain-api` re-exports `GainError` via `pub use gain_error::GainError` so callers see it at the same public path.

---

## Public API Contract (`gain-api`)

### Measurement types

Defined in the `analysis` crate; re-exported by `gain-api` as public types.

```rust
pub enum MeasurementQuality {
    Placeholder,  // not yet computed
    Estimated,    // approximated, not spec-compliant
    Verified,     // spec-compliant implementation
}

pub struct MeasurementValue {
    pub value: f32,
    pub quality: MeasurementQuality,
}

pub struct Measurements {
    pub peak_dbfs: f32,                      // Verified in Phase 2
    pub rms_dbfs: f32,                       // Verified in Phase 2
    pub crest_factor_db: f32,                // peak_dbfs − rms_dbfs; Verified in Phase 2
    pub integrated_lufs: MeasurementValue,   // Placeholder in Phase 2 (value = 0.0)
}
```

LUFS is always present in the struct so callers can display it and query its quality. It is never approximated — `quality: Placeholder` signals "not yet computed" explicitly.

### Metadata and analysis result

```rust
pub enum ContainerFormat {
    Wav,
    Aiff,
}

pub struct AudioMetadata {
    pub duration_secs: f64,
    pub sample_rate: u32,
    pub channels: u16,
    pub format: ContainerFormat,
}

pub struct AnalysisResult {
    pub metadata: AudioMetadata,
    pub measurements: Measurements,
}
```

### Preset types

`MeasureType` is defined in `gain_decision` and re-exported by `gain-api`. This avoids a circular dependency: `gain_decision` uses `MeasureType` in its function signature and cannot import it from `gain-api` (which imports `gain_decision`).

```rust
// defined in gain_decision, re-exported by gain-api
pub enum MeasureType { Peak, Rms }

pub enum RecommendationPreset {
    MixPrepConservative,   // Peak −18 dBFS
    MixPrepStandard,       // Peak −12 dBFS  ← default
    MixPrepAggressive,     // Peak −6 dBFS
    AnalogConsole,         // RMS −18 dBFS
    AnalogConsoleHot,      // RMS −14 dBFS
    DialoguePrep,          // Peak −10 dBFS
    Custom { measure: MeasureType, target_db: f32 },
}
```

`album-consistency` (two-pass batch RMS) is deferred to Phase 3.

### Public functions

```rust
/// Step 1: decode and measure
pub fn analyze_file(path: &Path) -> Result<AnalysisResult, GainError>

/// Step 2: apply preset and produce a Gain Recommendation Map
pub fn generate_recommendation(
    analysis: &AnalysisResult,
    preset: RecommendationPreset,
) -> Result<GainRecommendationMap, GainError>
```

`gain-api` translates `RecommendationPreset` into `(MeasureType, f32, &str)` before calling `gain_decision::recommend()`. The user-facing preset enum never enters internal crates.

### Updated `GainRecommendationMap`

`preset_used: Option<String>` is added to `GainRecommendationMap` in `gain_map`:

```rust
pub struct GainRecommendationMap {
    pub version: u32,
    pub preset_used: Option<String>,   // e.g. "MixPrepStandard"
    pub regions: Vec<GainRegion>,
}
```

`Default` still produces `version: 1, preset_used: None, regions: vec![]`.

---

## Internal Pipeline

### Dependency graph

```
gain-error       (no deps)
gain_map         (no deps)
audio_ingestion  → gain-error + [symphonia]
analysis         → audio_ingestion + gain-error
gain_decision    → analysis + gain_map + gain-error
gain-api         → audio_ingestion + analysis + gain_decision + gain_map + gain-error
ffi              → gain-api
gain-standalone  → gain-api
```

`segmentation` and `classification` are unchanged stubs; they are not in the Phase 2 call path.

### Crate responsibilities

**`audio_ingestion`**
- Decodes WAV and AIFF via symphonia
- Returns `AudioBuffer { samples: Vec<f32>, sample_rate: u32, channels: u16 }` and `AudioMetadata`
- Errors: `FileNotFound`, `UnsupportedFormat`, `DecodeFailure`

**`analysis`**
- Takes `&AudioBuffer`
- Computes Peak dBFS: `20 * log10(samples.iter().map(|s| s.abs()).fold(0f32, f32::max))`
- Computes RMS dBFS: `20 * log10(sqrt(sum(s²) / n))`
- Computes Crest Factor: `peak_dbfs − rms_dbfs`
- Sets `integrated_lufs = MeasurementValue { value: 0.0, quality: Placeholder }`
- Errors: `InvalidAudio` (empty buffer, zero-length, NaN samples), `AnalysisFailure`

**`gain_decision`**

```rust
pub fn recommend(
    measurements: &Measurements,
    measure: MeasureType,
    target_db: f32,
    preset_label: &str,
) -> Result<GainRecommendationMap, GainError>
```

- Picks `peak_dbfs` or `rms_dbfs` based on `measure`
- `gain_db = target_db − measured_db`
- Produces one `GainRegion` covering `0.0` to `duration_secs` with `region_type: RegionType::Stable`
- `confidence: 1.0` (whole-file measurement is always high confidence)
- `reason`: human-readable string, e.g. `"Peak −12 dBFS target (MixPrepStandard)"`
- Sets `preset_used: Some(preset_label.to_string())`

**`gain-api`** — orchestrates both steps, owns all public types, re-exports `GainError`.

---

## Phase 2 Output Characteristics

Every `GainRecommendationMap` produced in Phase 2 has exactly these properties:
- `regions.len() == 1`
- `regions[0].region_type == RegionType::Stable`
- `regions[0].confidence == 1.0`
- `regions[0].start_time == 0.0`
- `regions[0].end_time == metadata.duration_secs`
- `preset_used == Some("PresetName")`

This is by design. Multi-region output requires segmentation, which is Phase 3.

---

## FFI Changes

### Unchanged surface
All existing functions (`gain_stage_analyze`, `gain_stage_free_map`, `gain_stage_map_region_count`, `gain_stage_map_get_region`, `gain_stage_map_version`) are unchanged. `gain_stage_analyze` gets wired to real data via `gain-api` in Phase 2.

### New in Phase 2

```c
/* File-path entry point for standalone and integration testing */
GainStageMap* gain_stage_analyze_file(
    const char* path,
    uint8_t     preset   /* see GAIN_STAGE_PRESET_* constants */
);

/* Error introspection — call immediately after a NULL return */
uint8_t     gain_stage_last_error_code(void);
const char* gain_stage_last_error_message(void);
```

Error code mapping:
| Code | Variant |
|------|---------|
| 1 | `FileNotFound` |
| 2 | `UnsupportedFormat` |
| 3 | `DecodeFailure` |
| 4 | `InvalidAudio` |
| 5 | `AnalysisFailure` |
| 6 | `InternalError` |

`gain_stage_last_error_message()` returns a pointer to a static thread-local buffer valid until the next FFI call on the same thread.

### Deferred to Phase 4
A C-ABI `AnalysisResult` struct and two-step `gain_stage_generate_recommendation()` — not needed until the ARA plugin requires the split.

---

## Dependencies

### New external dependency

`gain-error` must be added to `gain-core/Cargo.toml`'s `members` list alongside the existing crates.

```toml
# gain-core/crates/audio_ingestion/Cargo.toml
[dependencies]
symphonia = { version = "0.5", features = ["wav", "aiff", "pcm"] }
gain-error = { path = "../gain-error" }
```

Phase 2 format support: WAV, AIFF.
Phase 3 will add: `features = ["flac", "caf"]`.

### No other new external dependencies
All measurement math uses `std`. No FFT library is needed for Peak/RMS.

---

## Testing Strategy

### Tier 1 — Pure math (no files)
`analysis` and `gain_decision` unit tests use synthetic `Vec<f32>` buffers.

- Constant 1.0 samples → `peak_dbfs = 0.0`, `rms_dbfs = 0.0`
- Constant 0.5 samples → `peak_dbfs ≈ −6.02`, `rms_dbfs ≈ −6.02`
- Crest Factor for a sine wave: `peak ≈ 0.0`, `rms ≈ −3.01`, crest ≈ 3.01
- Every test asserts `integrated_lufs.quality == MeasurementQuality::Placeholder`
- Gain math: known peak −6 dBFS + MixPrepStandard (target −12) → `gain_db = −6.0`

### Tier 2 — File I/O (generated in test setup)
`audio_ingestion` tests write minimal valid WAV and AIFF byte sequences in `#[cfg(test)]` helpers using `tempfile`. No committed binary blobs. Covers: valid load, `FileNotFound`, `UnsupportedFormat`.

### Tier 3 — End-to-end pipeline (gain-api integration tests)
`gain-api/tests/pipeline.rs` generates a known WAV via `tempfile`, runs the full two-step pipeline, and asserts `gain_db` is within 0.1 dB of the expected value. This is the smoke test confirming all crates are wired correctly.

### `test-assets/` stays empty
Real audio fixtures (royalty-free, known-loudness files) are deferred to Phase 3 integration tests.

---

## Architecture Constraints (carried forward from Phase 1)

- All `unsafe` blocks require a `// SAFETY:` comment
- No global mutable state in Rust
- No exceptions cross the FFI boundary (`ffi_guard` catch_unwind wrapper already in place)
- `gain-standalone` and `gain-ara` may only import `gain-api` (ADR-005)
- No `unwrap()` in production code paths

---

## What Does Not Change

- `gain_map` types (`GainRegion`, `RegionType`) — only `GainRecommendationMap` gets `preset_used`
- The `RegionType` enum — `Stable` is used exclusively in Phase 2 output
- The `ffi_guard` catch_unwind wrapper — already in place from Phase 1
- `gain-standalone` Tauri command signatures — updated internally to call the two-step API but external command names stay the same
- `GAIN_MAP_SCHEMA_VERSION` constant — stays at 1 (the data model version has not changed)
