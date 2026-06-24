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

Defined in `gain_map` (the shared types crate), re-exported by `gain-api`. Placing them in `gain_map` — rather than in `analysis` — lets `gain_decision` depend only on `gain_map` and eliminates the diamond dependency where both `gain-api` and `gain_decision` would otherwise import from `analysis`.

```rust
pub enum MeasurementQuality {
    Placeholder,  // not yet computed
    Estimated,    // approximated, not spec-compliant
    Verified,     // spec-compliant implementation
}

pub struct MeasurementValue {
    pub value: Option<f32>,   // None when quality is Placeholder
    pub quality: MeasurementQuality,
}

pub struct Measurements {
    pub peak_dbfs: f32,                      // Verified in Phase 2
    pub rms_dbfs: f32,                       // Verified in Phase 2
    pub crest_factor_db: f32,                // log-domain dB difference (peak_dbfs − rms_dbfs); Verified in Phase 2
    pub integrated_lufs: MeasurementValue,   // Placeholder in Phase 2: { value: None, quality: Placeholder }
}
```

`value: Option<f32>` rather than a bare `f32` makes misuse a compile error — UI code cannot accidentally render a placeholder as a number without explicitly handling `None`. `quality: Placeholder` always coincides with `value: None`; `Estimated` and `Verified` always coincide with `value: Some(f32)`.

Crest factor is the **log-domain dB difference** between peak and RMS, not the linear amplitude ratio. This is the correct representation for gain staging UI and is unambiguous for future classification heuristics.

LUFS is always present in the struct so callers can display its quality state. It is never approximated — shipping a fake value and labelling it `Estimated` would mislead users comparing against DAW meters.

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

`preset_used: Option<PresetId>` is added to `GainRecommendationMap` in `gain_map`. A structured enum is used instead of a string so that Phase 3 provenance tracking, serialization, and match exhaustiveness are all type-safe. `PresetId` is also defined in `gain_map`.

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum PresetId {
    MixPrepConservative,
    MixPrepStandard,
    MixPrepAggressive,
    AnalogConsole,
    AnalogConsoleHot,
    DialoguePrep,
    Custom,
}

pub struct GainRecommendationMap {
    pub version: u32,
    pub preset_used: Option<PresetId>,
    pub regions: Vec<GainRegion>,
}
```

`Default` still produces `version: 1, preset_used: None, regions: vec![]`. The `reason` string on each `GainRegion` carries human-readable description; `preset_used` carries structured identity.

---

## Internal Pipeline

### Dependency graph

```
gain-error       (no deps)
gain_map         (no deps) ← holds GainRegion, GainRecommendationMap, Measurements, PresetId, MeasurementValue
audio_ingestion  → gain-error + [symphonia]
analysis         → gain_map + audio_ingestion + gain-error
gain_decision    → gain_map + gain-error
gain-api         → audio_ingestion + analysis + gain_decision + gain_map + gain-error
ffi              → gain-api
gain-standalone  → gain-api
```

`gain_decision` depends only on `gain_map` and `gain-error` — it never imports from `analysis`. This eliminates the diamond dependency and keeps `gain_decision` reusable independently of the audio decoding stack (e.g. for future batch or server contexts that supply pre-computed `Measurements`).

`segmentation` and `classification` are unchanged stubs; they are not in the Phase 2 call path.

### Crate responsibilities

**`audio_ingestion`**
- Decodes WAV and AIFF via symphonia
- Returns `AudioBuffer { samples: Vec<f32>, sample_rate: u32, channels: u16 }` and `AudioMetadata`
- Errors: `FileNotFound`, `UnsupportedFormat`, `DecodeFailure`

**`analysis`**
- Takes `&AudioBuffer`; all samples **must** be normalized to `[-1.0, 1.0]`. Any sample with `abs() > 1.0` or containing `NaN`/`Inf` is an `InvalidAudio` error.
- **Normalization contract**: `audio_ingestion` is responsible for delivering normalized `f32` samples via symphonia's built-in PCM conversion. `analysis` enforces the contract but does not re-normalize.
- Computes **Peak dBFS**: `max_amplitude = samples.iter().map(|s| s.abs()).fold(0f32, f32::max)`. If `max_amplitude == 0.0` (silent buffer), Peak dBFS is `SILENCE_FLOOR_DBFS = -120.0`. Otherwise `20.0 * max_amplitude.log10()`. **Do not use `f32::MIN_POSITIVE` as the clamp** — it maps silence to ≈ −758 dBFS, which is acoustically meaningless and will corrupt crest factor and future classification heuristics. The −120 dBFS floor matches DAW convention (industry range: −120 to −144 dBFS).
- Computes **RMS dBFS**: computed across **all samples in the flattened interleaved buffer** (no per-channel weighting). `rms = sqrt(sum(s²) / n)`. If `rms == 0.0`, RMS dBFS is `SILENCE_FLOOR_DBFS`. Otherwise `20.0 * rms.log10()`. **Phase 2 RMS is a diagnostic metric, not a perceptual model.** It is not LUFS-compatible and must not be used as a perceptual proxy. Phase 3 LUFS will not extend this RMS implementation — it will replace the conceptual model with gating, frequency weighting, and channel awareness. Stereo panning energy may read slightly higher than mono-equivalent in Phase 2.
- Computes **Crest Factor**: `peak_dbfs − rms_dbfs` (log-domain dB difference, not linear ratio). When audio is silent, both peak and RMS clamp to `SILENCE_FLOOR_DBFS`, so crest factor is `0.0`. **Silence is treated as a zero-dynamic-range signal for crest factor purposes** — future classification logic must not interpret a `0.0` crest factor as "stable audio with a known dynamic range."
- Sets `integrated_lufs = MeasurementValue { value: None, quality: Placeholder }`. **`integrated_lufs.value` MUST NOT be used in any gain calculation in Phase 2.** It is present for struct completeness only. Consumers must check `quality` before using `value`.
- Errors: `InvalidAudio` (empty buffer, NaN/Inf samples, samples outside `[-1.0, 1.0]`), `AnalysisFailure`

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
- `reason`: purely descriptive string, e.g. `"Applied target of −12 dBFS using Peak measurement"`. **Never encode preset identity in the reason string** — `PresetId` carries identity; `reason` carries description. `reason` is strictly non-parsable and must never be used for control flow.
- Sets `preset_used: Some(preset_id)` — structured `PresetId`, not a string

**`gain-api`** — orchestrates both steps, owns all public types, re-exports `GainError`.

---

## Phase 2 Output Characteristics

Every `GainRecommendationMap` produced in Phase 2 has exactly these properties:
- `regions.len() == 1`
- `regions[0].region_type == RegionType::Stable`
- `regions[0].confidence == 1.0`
- `regions[0].start_time == 0.0`
- `regions[0].end_time == metadata.duration_secs`
- `preset_used == Some(PresetId::…)`

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
    uint8_t     preset   /* see GAIN_STAGE_PRESET_* constants below */
);

/* Error introspection — call immediately after a NULL return */
uint8_t     gain_stage_last_error_code(void);
const char* gain_stage_last_error_message(void);
```

**Preset `uint8` mapping** (must be defined in the C header as `GAIN_STAGE_PRESET_*` constants):

| Constant | Value | `PresetId` |
|---|---|---|
| `GAIN_STAGE_PRESET_MIX_PREP_CONSERVATIVE` | `0` | `MixPrepConservative` |
| `GAIN_STAGE_PRESET_MIX_PREP_STANDARD` | `1` | `MixPrepStandard` |
| `GAIN_STAGE_PRESET_MIX_PREP_AGGRESSIVE` | `2` | `MixPrepAggressive` |
| `GAIN_STAGE_PRESET_ANALOG_CONSOLE` | `3` | `AnalogConsole` |
| `GAIN_STAGE_PRESET_ANALOG_CONSOLE_HOT` | `4` | `AnalogConsoleHot` |
| `GAIN_STAGE_PRESET_DIALOGUE_PREP` | `5` | `DialoguePrep` |
| Any other value | — | → `InternalError { details: "unknown preset code N" }`, returns NULL |

`Custom` preset is not exposed via the FFI in Phase 2 — it requires additional parameters (measure type + target dB) with no defined ABI yet. This is deferred to Phase 4.

**Error code mapping:**
| Code | Variant |
|------|---------|
| 1 | `FileNotFound` |
| 2 | `UnsupportedFormat` |
| 3 | `DecodeFailure` |
| 4 | `InvalidAudio` |
| 5 | `AnalysisFailure` |
| 6 | `InternalError` |

`gain_stage_last_error_message()` returns a pointer to a static thread-local buffer valid until the next FFI call on the same thread.

**Concurrency and reentrancy limitation:** The thread-local error buffer is **not safe for concurrent cross-thread usage**, and **FFI calls must be treated as non-reentrant at the host level**. Some ARA hosts run batch rendering across multiple audio tracks on a shared thread pool — thread-local state may still collide logically even when calls are technically on different threads. This is acceptable for Phase 2 (standalone app is single-threaded at the FFI boundary) but must be addressed before ARA integration in Phase 4.

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
- Every test asserts `integrated_lufs.quality == MeasurementQuality::Placeholder` and `integrated_lufs.value == None`
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

- `gain_map` types (`GainRegion`, `RegionType`) — `GainRecommendationMap` gets `preset_used: Option<PresetId>`; new types `Measurements`, `MeasurementValue`, `MeasurementQuality`, and `PresetId` are added to `gain_map`
- The `RegionType` enum — `Stable` is used exclusively in Phase 2 output
- The `ffi_guard` catch_unwind wrapper — already in place from Phase 1
- `gain-standalone` Tauri command signatures — updated internally to call the two-step API but external command names stay the same
- `GAIN_MAP_SCHEMA_VERSION` constant — stays at 1 (the data model version has not changed)

---

## Future Watch Items (not Phase 2 scope)

**`gain_map` as de facto ABI:** `gain_map` now serves as DSP result schema, UI model, and decision metadata — and because everything flows through it (DSP outputs, presets, FFI representation, UI), it has become the de facto ABI of the system. This is powerful but consequential: changes here ripple everywhere, versioning discipline becomes critical, and Phase 3 additions must be extremely intentional. If the crate continues to grow, consider splitting into `gain_types` (pure DSP outputs) and `gain_map` (recommendation/UI layer). `MeasurementValue` in particular is likely to evolve into a richer state machine in Phase 3 (gated values, short-term vs. integrated, trusted vs. untrusted). Design Phase 3 additions with this in mind before committing to `gain_map` structure.

**`gain_decision` structural scaling pressure:** `gain_decision → gain_map` is the only architectural coupling that could constrain future reuse. If a WASM DSP engine, server-side batch processor, or plugin-only decision context is introduced, this dependency may need to be broken by introducing a thin interface layer. This is the single known structural scaling pressure point in the Phase 2 design.

**FFI `Custom` preset ABI:** Deferred in Phase 2. When exposed, **use a struct-based ABI rather than extending the `uint8` flag space** — overloading byte flags for parameterized presets is a dead end. The ABI extension should be designed before Phase 4 FFI work begins.

**FFI non-reentrancy at scale:** Phase 4 ARA integration must design a per-call or per-session error context rather than thread-local state. The correct solution is likely a context handle passed into each FFI call, similar to OpenAL's `ALCcontext*` pattern.
