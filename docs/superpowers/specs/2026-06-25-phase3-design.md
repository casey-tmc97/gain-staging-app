# Phase 3 Design Spec
## Gain Stage App — Intelligent Analysis, Loudness Measurement, Segmentation, and Regional Gain Recommendations

**Date:** 2026-06-25
**Status:** Approved
**Author:** casey-tmc97

---

## Scope

Phase 3 transforms Gain Stage from a whole-file gain normalization engine into an intelligent audio analysis system. Every stub crate from Phase 1 that remained in Phase 2 (`segmentation`, `classification`) gets a real implementation.

**In scope:**
- ITU-R BS.1770-4 integrated, short-term peak, and momentary peak LUFS via `ebur128`
- True Peak dBTP at 4× oversampling via `ebur128`
- Energy/silence-based audio segmentation (`segmentation` crate activated)
- Deterministic content classification: Silence, Dialogue, Music, Ambience, Percussive, Mixed, Unknown (`classification` crate activated)
- Per-region gain recommendations via `AnalysisBundle`
- New LUFS-based presets: DialogueBroadcast, PodcastPrep, VoiceoverPrep, MusicStemPrep, FilmDialogue
- Album consistency: three-function explicit API
- `GainRegion` repurposed as analysis+decision bundle; `RegionDecision` introduced
- `AnalysisBundle` introduced to unify single-region and multi-region paths
- `GAIN_MAP_SCHEMA_VERSION` bumped to 2
- FLAC and CAF format support (symphonia feature flags activated)
- FFI two-step entry points: `gain_stage_begin_analysis` / `gain_stage_generate_recommendation`

**Explicitly deferred to Phase 4:**
- ARA integration
- Real-time / streaming DSP
- Machine learning / neural classification
- DAW session import
- `AlbumAnchorMethod::ReferenceTrack` (user-selected anchor)
- FFI `Custom` preset ABI
- Per-session FFI error context (non-reentrancy fix)
- Album analysis parallelism (rayon / tokio)

---

## Goals

1. Deliver standards-compliant loudness measurement (BS.1770-4) with no approximations
2. Activate segmentation and classification with deterministic, explainable rules
3. Produce per-region gain recommendations as the primary output
4. Introduce album consistency workflow
5. Preserve Phase 2 function signature compatibility (`analyze_file()`, `generate_recommendation()`)
6. Prepare architecture cleanly for ARA integration in Phase 4

---

## Updated Dependency Graph

`gain_decision` remains data-only. `gain-api` is the sole orchestrator.

```
gain-error       (no deps)
gain_map         (no deps)  ← all shared types live here
audio_ingestion  → gain-error + [symphonia]
analysis         → gain_map + audio_ingestion + gain-error + [ebur128]
segmentation     → gain_map + gain-error
classification   → gain_map + gain-error
gain_decision    → gain_map + gain-error
gain-api         → audio_ingestion + analysis + segmentation + classification + gain_decision + gain_map + gain-error
ffi              → gain-api
gain-standalone  → gain-api
```

`segmentation` and `classification` do not depend on each other. Neither depends on `audio_ingestion` or `analysis`. `gain-api` passes decoded buffers and per-region `Measurements` to them and assembles `RegionAnalysis` values before calling `gain_decision`.

The boundary between the analysis pipeline and the decision engine is `RegionAnalysis` — a data-only type in `gain_map`. `gain_decision` remains reusable independently of the audio pipeline. It operates on `&[RegionAnalysis]` and knows nothing about audio decoding, segmentation algorithms, or classification rules. This resolves the structural scaling pressure point identified in the Phase 2 future watch items.

---

## gain_map Type Evolution

### MeasurementValue and MeasurementQuality (carried from Phase 2, unchanged)

`MeasurementValue` and `MeasurementQuality` are defined in the Phase 2 spec and implemented in `gain_map`. They are reproduced here for completeness:

```rust
pub struct MeasurementValue {
    pub value: Option<f32>,       // None when quality is Placeholder
    pub quality: MeasurementQuality,
}

pub enum MeasurementQuality {
    Placeholder,  // not yet computed
    Estimated,    // approximated, not spec-compliant
    Verified,     // spec-compliant implementation
}
```

`value: None` always coincides with `Placeholder`; `value: Some(f32)` always coincides with `Estimated` or `Verified`. These invariants are unchanged in Phase 3. The new Phase 3 fields (`short_term_lufs_peak`, `momentary_lufs_peak`, `true_peak_dbtp`) all report `Verified` when successfully computed.

### Measurements (extended)

```rust
pub struct Measurements {
    pub peak_dbfs: f32,
    pub rms_dbfs: f32,
    pub crest_factor_db: f32,
    pub integrated_lufs: MeasurementValue,       // Verified in Phase 3
    pub short_term_lufs_peak: MeasurementValue,  // peak of 3s windows; Verified in Phase 3
    pub momentary_lufs_peak: MeasurementValue,   // peak of 400ms windows; Verified in Phase 3
    pub true_peak_dbtp: MeasurementValue,        // Verified in Phase 3
}
```

`short_term_lufs_peak` and `momentary_lufs_peak` use the `_peak` suffix per EBU R128 terminology. Both are the **maximum value observed across all windows during analysis** — not a current, instantaneous, or average value. This naming contract prevents future implementations from misinterpreting these as real-time window outputs.

Temporal aggregation semantics:
- `integrated_lufs` — whole-file gated loudness (single value)
- `short_term_lufs_peak` — maximum of all 3-second window outputs
- `momentary_lufs_peak` — maximum of all 400ms window outputs
- `true_peak_dbtp` — maximum inter-sample peak across the file

All four report `MeasurementQuality::Verified` when successfully computed in Phase 3.

### ContentClass (new)

```rust
pub enum ContentClass {
    Silence,      // positive identification: energy below silence threshold
    Dialogue,
    Music,
    Ambience,
    Percussive,
    Mixed,
    Unknown,      // no rule matched with sufficient confidence
}
```

`Silence` is a positive classification, not a fallback. A silent region is definitively identified by energy characteristics and is distinct from `Unknown`. This distinction matters for:
- Album anchor computation — silent tracks are excluded from LUFS anchor
- Future ARA display — silence gaps render as neutral zones
- Recommendations — silent regions receive `gain_db = 0.0` by convention; no gain math is applied

### RegionType (updated)

```rust
pub enum RegionType {
    Stable,      // Phase 2 compat: whole-file no-classification path
    Silence,
    Dialogue,
    Music,
    Ambience,
    Percussive,
    Mixed,
    Unknown,
}
```

**Breaking change:** `Transient` and `EnvelopeControlled` are removed. These variants were defined in Phase 1 but no real Phase 2 code path produced them. The schema version bump to 2 covers this change. FFI constants `GAIN_STAGE_REGION_TRANSIENT` and `GAIN_STAGE_REGION_ENVELOPE_CONTROLLED` are deprecated in the C header with `/* deprecated in schema v2 — see GAIN_STAGE_CLASS_* */` comments.

`Stable` is retained as the designated output for the Phase 2 compat path in `generate_recommendation()`, where segmentation has not run and the file is treated as a single region. `RegionType` maps directly from `ContentClass` for all classified regions. For the Phase 2 compat path (no classification): `region_type = RegionType::Stable`, `content_class = ContentClass::Unknown`.

**Enforcement constraint:** `RegionType` must never be constructed manually outside of two locations: (1) `impl From<ContentClass> for RegionType` in `gain_map`, which is the sole mapping function, and (2) the Phase 2 compat path in `gain-api` which sets `RegionType::Stable` explicitly. Anywhere else in the codebase, `RegionType` is derived from `ContentClass` via `From`. This prevents the two enums from drifting — `ContentClass` is the semantic source of truth; `RegionType` is the derived external label.

The `From` implementation:
```rust
impl From<ContentClass> for RegionType {
    fn from(class: ContentClass) -> Self {
        match class {
            ContentClass::Silence    => RegionType::Silence,
            ContentClass::Dialogue   => RegionType::Dialogue,
            ContentClass::Music      => RegionType::Music,
            ContentClass::Ambience   => RegionType::Ambience,
            ContentClass::Percussive => RegionType::Percussive,
            ContentClass::Mixed      => RegionType::Mixed,
            ContentClass::Unknown    => RegionType::Unknown,
        }
    }
}
```

`Stable` has no `ContentClass` counterpart by design — it is only reachable via the explicit Phase 2 compat assignment.

### RegionAnalysis (updated)

```rust
pub struct RegionAnalysis {
    pub start_time: f64,
    pub end_time: f64,
    pub measurements: Measurements,
    pub region_type: RegionType,
    pub content_class: ContentClass,
    pub classification_confidence: f32,   // 0.0–1.0
}
```

`region_type` and `content_class` coexist: `content_class` is the internal classifier output; `region_type` is the external/FFI-facing label. For classified regions they are redundant — `region_type` is always derivable from `content_class` — but keeping both prevents the `Stable` special case from leaking into `ContentClass`.

**Structural enforcement (stronger than the discipline-based constraint in the previous section):** `RegionAnalysis` has no public constructor. All fields are `pub` for reading; construction is `pub(crate)` within `gain_map`. External crates, including `gain-api`, construct `RegionAnalysis` only through two named factory functions exposed by `gain_map`:

```rust
impl RegionAnalysis {
    /// Used by gain-api for classified regions: region_type derived from content_class via From.
    pub(crate) fn from_classification(
        start_time: f64,
        end_time: f64,
        measurements: Measurements,
        content_class: ContentClass,
        classification_confidence: f32,
    ) -> Self {
        Self {
            start_time,
            end_time,
            measurements,
            region_type: RegionType::from(content_class),
            content_class,
            classification_confidence,
        }
    }

    /// Used exclusively by gain-api for the Phase 2 compat whole-file path.
    pub(crate) fn whole_file_stable(
        start_time: f64,
        end_time: f64,
        measurements: Measurements,
    ) -> Self {
        Self {
            start_time,
            end_time,
            measurements,
            region_type: RegionType::Stable,
            content_class: ContentClass::Unknown,
            classification_confidence: 1.0,
        }
    }
}
```

`gain_map` re-exports these constructors via `pub use` for `gain-api` only. No other crate can construct a `RegionAnalysis`, so mismatched `(content_class, region_type)` pairs are a compile error, not a discipline concern.

`classification_confidence` carries classifier certainty. Phase 3 deterministic rules produce 1.0 for unambiguous cases and fractional values for borderline cases. The field exists in Phase 3 so future heuristics have a typed slot without a type change.

### RegionDecision (new)

```rust
pub struct RegionDecision {
    pub is_applicable: bool,    // false = no recommendation applies (silence, error); ignore gain_db
    pub gain_db: f32,
    pub confidence: f32,
    pub reason: String,    // human-readable; never parsed for control flow
}
```

`RegionDecision` is the pure output of `gain_decision`. It carries no analysis provenance.

`is_applicable` separates the "recommendation domain" concern from the confidence score. When `false`, `gain_db` must be ignored — the region has no gain recommendation (silence, or future error states). This prevents downstream consumers from interpreting `confidence = 0.0` as "low-quality detection" when the actual meaning is "not a recommendation domain." `confidence` retains its meaning as recommendation certainty when `is_applicable = true`; it is undefined (treat as 0.0) when `is_applicable = false`.

Silence regions: `is_applicable = false`, `gain_db = 0.0`, `confidence = 0.0`, `reason = "Silence region — not a recommendation domain"`.
All other regions: `is_applicable = true`, `gain_db` and `confidence` per the decision logic below.

### GainRegion (repurposed)

```rust
pub struct GainRegion {
    pub analysis: RegionAnalysis,
    pub decision: RegionDecision,
}
```

`GainRegion` is the combined output of the full pipeline: analysis provenance bundled with gain recommendation. Previously a flat struct (`start_time`, `end_time`, `gain_db`, `confidence`, `region_type`, `reason`); now a bundle of the two concerns. Flat fields are accessible via `region.analysis.*` and `region.decision.*`.

**Breaking change from Phase 2.** Schema v2 covers this. Callers that previously accessed `region.gain_db` must update to `region.decision.gain_db`; callers that accessed `region.start_time` must update to `region.analysis.start_time`.

The separation matters for consumers: analysis fields (measurements, classification) are reusable for UI, debugging, and future ML training. Decision fields (gain_db, confidence, reason) are the recommendation output. Embedding them in the same struct without a named boundary would allow "decision contamination" of DSP data.

### AnalysisBundle (new)

```rust
pub struct AnalysisBundle {
    pub regions: Vec<RegionAnalysis>,
}
```

`AnalysisBundle` is the named output of the analysis pipeline and the input contract for `gain_decision`. Introducing a named type (rather than a raw `Vec<RegionAnalysis>`) formalizes the pipeline boundary and prevents "single region is a special case" reasoning anywhere in the codebase.

- Phase 3 segmented path: `regions.len() >= 1`
- Phase 2 compat path: `regions.len() == 1` (one synthetic whole-file region)

Both paths call `gain_decision::recommend_regions(&bundle.regions, ...)` identically.

### GainRecommendationMap (updated)

```rust
pub struct GainRecommendationMap {
    pub version: u32,
    pub preset_used: Option<PresetId>,
    pub recommendations: Vec<GainRegion>,
}
```

**Breaking change:** `regions: Vec<GainRegion>` (old flat type) → `recommendations: Vec<GainRegion>` (new bundled type). Field renamed and element type changed. Schema v2 signals this change to all consumers.

`Default` produces `version: GAIN_MAP_SCHEMA_VERSION, preset_used: None, recommendations: vec![]`.

### AlbumAnchor (new)

```rust
pub enum AlbumAnchorMethod {
    MedianLufs,
    MeanLufs,
}

pub struct AlbumAnchor {
    pub reference_lufs: f32,
    pub preset_id: PresetId,
    pub method: AlbumAnchorMethod,
}
```

Phase 3 anchor methods are purely statistical over the provided `AnalysisResult` set. `ReferenceTrack` (user-defined anchor selection) is deferred to Phase 4 — it requires UX state, track comparison semantics, and metadata persistence that are outside Phase 3 scope.

### MeasureType (updated)

```rust
pub enum MeasureType { Peak, Rms, Lufs }
```

`Lufs` added for LUFS-based presets. `gain_decision` reads `measurements.integrated_lufs.value` when `MeasureType::Lufs`. `gain-api` must verify `integrated_lufs.quality == Verified` before dispatching a `Lufs`-type preset; if quality is `Placeholder`, return `GainError::AnalysisFailure { details: "LUFS measurement unavailable for this preset" }`.

### GainError (updated)

```rust
pub enum GainError {
    // Phase 2 variants (unchanged)
    FileNotFound { path: String },
    UnsupportedFormat { format: String },
    DecodeFailure { details: String },
    InvalidAudio { details: String },
    AnalysisFailure { details: String },
    InternalError { details: String },
}
```

No new `GainError` variants are added in Phase 3. Per-file batch errors are surfaced via `Vec<Result<>>` (see album consistency API).

### PresetId and RecommendationPreset (Phase 3 additions)

```rust
pub enum PresetId {
    // Phase 2 (unchanged)
    MixPrepConservative, MixPrepStandard, MixPrepAggressive,
    AnalogConsole, AnalogConsoleHot, DialoguePrep,
    // Phase 3
    DialogueBroadcast,   // −24 LUFS  (US broadcast)
    PodcastPrep,         // −16 LUFS
    VoiceoverPrep,       // −19 LUFS
    MusicStemPrep,       // Peak −12 dBFS
    FilmDialogue,        // −27 LUFS  (cinema dialogue)
    AlbumConsistency,    // LUFS anchor (album batch only)
    Custom,
}
```

`RecommendationPreset` gains the same new variants. `gain-api` translates `RecommendationPreset` → `(MeasureType, f32, &str)` before calling `gain_decision`. LUFS-based presets use `MeasureType::Lufs`.

### GAIN_MAP_SCHEMA_VERSION

```rust
pub const GAIN_MAP_SCHEMA_VERSION: u32 = 2;
```

Bumped from 1. `GainRecommendationMap::default()` stamps version 2.

**Version compatibility rule (replaces Phase 2 strict equality):** Consumers must reject maps where `version < 2` (unknown older format). Consumers must accept maps where `version >= 2`, treating unknown future fields as absent or default. Hard rejection on `version != 2` would make Phase 3→3.1 fixes breaking changes for no real safety gain — a reader that understands v2 can safely ignore fields added in v3.

Per-field feature gating: Phase 3 fields (`short_term_lufs_peak`, `momentary_lufs_peak`, `true_peak_dbtp`, `content_class`, `classification_confidence`, `is_applicable`) must be treated as optional by any consumer that reads serialized maps. A missing field defaults to `MeasurementQuality::Placeholder` for `MeasurementValue` fields and to safe zero-values for numeric fields. Consumers must not hard-fail on unexpected fields.

The FFI version accessor `gain_stage_map_version()` continues to return the stamped integer; C callers must check `>= 2` rather than `== 2`.

---

## Crate: `analysis` (extended)

### ebur128 integration

```toml
# gain-core/crates/analysis/Cargo.toml
[dependencies]
ebur128 = "0.1"
symphonia = { version = "0.5", features = ["wav", "aiff", "flac", "caf", "pcm"] }
gain_map = { path = "../gain_map" }
gain-error = { path = "../gain-error" }
```

`ebur128` wraps libebur128, the ITU-R BS.1770-4 reference C implementation. It is initialized with the decoded channel count and sample rate.

**Processing model:** Samples are fed in 100ms blocks. After each block:
- `ebur128.loudness_shortterm()` is queried; running maximum updates `short_term_lufs_peak`
- `ebur128.loudness_momentary()` is queried; running maximum updates `momentary_lufs_peak`

After all blocks:
- `ebur128.loudness_global()` → `integrated_lufs`
- `ebur128.true_peak(channel)` across all channels → max → `true_peak_dbtp`

True Peak uses 4× oversampling (libebur128 default). EBU R128 prefers 8×; Phase 3 uses 4× and benchmarks it against the 10× real-time target. If 4× passes, 8× remains available as a quality upgrade without API changes.

**Channel handling:** `ebur128` applies BS.1770-4 channel weighting automatically for stereo. Mono proceeds without weighting. Surround (> 2 channels) is deferred to a future phase.

**Phase 3 format additions:** `symphonia` feature flags `flac` and `caf` are activated, enabling FLAC and CAF in `audio_ingestion` with no other changes to that crate.

### Per-region measurement

```rust
/// Measure the entire decoded buffer (Phase 2 compat path + file-level loudness)
pub fn measure(buffer: &AudioBuffer) -> Result<Measurements, GainError>

/// Measure a time-bounded slice of the buffer for per-region analysis
pub fn measure_region(
    buffer: &AudioBuffer,
    start_time: f64,
    end_time: f64,
) -> Result<Measurements, GainError>
```

`measure_region` computes sample indices from `start_time` / `end_time` and processes only the relevant frame range. A fresh `ebur128` state is constructed per region. No buffer copy is made — ebur128 processes a slice of the existing allocation.

Phase 2 measurement constraints are unchanged: NaN/Inf guard, −120 dBFS silence floor, crest factor 0.0 for silence.

---

## Crate: `segmentation` (activated)

### Public interface

```rust
pub struct Segment {
    pub start_time: f64,
    pub end_time: f64,
    pub is_silence: bool,
}

pub fn segment(buffer: &AudioBuffer) -> Result<Vec<Segment>, GainError>
```

Empty output is `AnalysisFailure`. If the entire file is below the silence threshold, returns one `Segment` with `is_silence: true`. Minimum output: 1 segment.

### Algorithm

1. **Frame energy:** Compute RMS energy in non-overlapping 20ms frames.
2. **Silence detection:** Frames where energy < −60 dBFS are silence candidates. Consecutive silence candidates with total duration ≥ 250ms form a silence segment.
3. **Energy change detection:** Smooth frame energies with a 100ms sliding window. Place segment boundaries where the smoothed delta between adjacent windows exceeds 6 dB.
4. **Merge short segments:** Non-silence segments shorter than 500ms are merged with the longer neighbor.

Internal constants (not user-facing):

```rust
const SILENCE_THRESHOLD_DBFS: f32    = -60.0;
const SILENCE_MIN_DURATION_SECS: f64 = 0.250;
const FRAME_SIZE_SECS: f64           = 0.020;
const ENERGY_WINDOW_SECS: f64        = 0.100;
const ENERGY_CHANGE_THRESHOLD_DB: f32 = 6.0;
const MIN_SEGMENT_SECS: f64          = 0.500;
```

`segmentation` does not hold `AudioBuffer` data beyond its local processing scope.

**Known fragility:** Fixed thresholds will produce over-segmentation on heavily compressed material (crest factor < 3 dB, where the 6 dB change threshold is never exceeded) and under-segmentation on cinematic audio (very soft dialogue embedded in ambient bed). Phase 3 accepts this limitation. Phase 4 will add content-aware threshold adaptation. Tier 5 fuzz tests must cover these cases and assert that the output is valid (≥ 1 segment, no panic) even if boundary placement is suboptimal.

---

## Crate: `classification` (activated)

### Public interface

```rust
pub fn classify(measurements: &Measurements, is_silence: bool) -> (ContentClass, f32)
```

Returns `(ContentClass, classification_confidence)`. `is_silence == true` short-circuits to `(ContentClass::Silence, 1.0)` without running heuristics.

### Rules (evaluated in priority order; first match wins)

| Priority | Class | Condition |
|---|---|---|
| 1 | `Silence` | `is_silence == true` |
| 2 | `Percussive` | `crest_factor_db >= 14.0` AND `integrated_lufs < −30.0 LUFS` (sparse transients) |
| 3 | `Dialogue` | `crest_factor_db ∈ [8.0, 20.0]` AND `rms_dbfs ∈ [−40.0, −14.0]` |
| 4 | `Music` | `crest_factor_db ∈ [3.0, 12.0]` AND `rms_dbfs >= −30.0` |
| 5 | `Ambience` | `crest_factor_db >= 6.0` AND `rms_dbfs < −35.0` |
| 6 | `Mixed` | Multiple partial conditions overlap |
| 7 | `Unknown` | No rule matched |

Confidence values:
- `Silence`: always 1.0
- Rules 2–5: 1.0 when a single rule matched cleanly; 0.7 when the nearest competing rule boundary is within 2 dB of the threshold
- `Mixed`: 0.8
- `Unknown`: 0.4

All rules and thresholds must be co-located in `classification/src/lib.rs` and documented in `docs/GainDecisionModel.md`. No classification logic may live elsewhere.

---

## Crate: `gain_decision` (updated)

### Updated signature

```rust
pub fn recommend_regions(
    regions: &[RegionAnalysis],
    measure: MeasureType,
    target_db: f32,
    preset_label: &str,
) -> Result<GainRecommendationMap, GainError>
```

The Phase 2 `recommend(measurements, ...)` function is removed. This is a breaking internal change covered by schema v2.

### Logic per region

- `ContentClass::Silence` → `is_applicable = false`, `gain_db = 0.0`, `confidence = 0.0`, `reason = "Silence region — not a recommendation domain"`
- All other classes → `is_applicable = true`, then:
- All other classes → pick measurement by `MeasureType`:
  - `Peak` → `peak_dbfs`
  - `Rms` → `rms_dbfs`
  - `Lufs` → `integrated_lufs.value` (caller has verified `quality == Verified`)
- `gain_db = target_db − measured_db`
- `confidence = region.classification_confidence × base_confidence`
  - `base_confidence = 0.9` for duration ≥ 1.0s
  - `base_confidence = 0.75` for duration in [0.5s, 1.0s)
  - `base_confidence = 0.6` for duration < 0.5s

Output `recommendations` is in the same order as the input `regions` slice.

---

## Public API (`gain-api`) — Updated and New Functions

### Unchanged (Phase 2 compat)

```rust
pub fn analyze_file(path: &Path) -> Result<AnalysisResult, GainError>

pub fn generate_recommendation(
    analysis: &AnalysisResult,
    preset: RecommendationPreset,
) -> Result<GainRecommendationMap, GainError>
```

Signatures unchanged. Internally, `generate_recommendation` builds `AnalysisBundle { regions: vec![whole_file_region] }` from the `AnalysisResult`, where `whole_file_region` has `region_type = RegionType::Stable`, `content_class = ContentClass::Unknown`, `classification_confidence = 1.0`. Output has exactly 1 `GainRegion`. No segmentation runs.

### New in Phase 3

```rust
/// Full pipeline: decode → measure → segment → classify → return AnalysisBundle
pub fn analyze_regions(path: &Path) -> Result<AnalysisBundle, GainError>

/// Generate per-region recommendations from a pre-built AnalysisBundle
pub fn generate_region_recommendations(
    bundle: &AnalysisBundle,
    preset: RecommendationPreset,
) -> Result<GainRecommendationMap, GainError>
```

`analyze_regions` is the Phase 3 primary entry point for the full pipeline. `generate_region_recommendations` calls `gain_decision::recommend_regions(&bundle.regions, ...)`.

### Album consistency API (new)

```rust
/// Analyze all files. Returns one Result per path in the same order; per-file errors
/// do not abort the batch. Callers filter on Ok/Err before passing to compute_album_anchor.
pub fn analyze_album_files(
    paths: &[&Path],
) -> Vec<Result<AnalysisResult, GainError>>

/// Compute LUFS anchor from analysis results.
/// Tracks with integrated_lufs.quality != Verified are excluded from anchor computation.
pub fn compute_album_anchor(
    results: &[AnalysisResult],
    preset: RecommendationPreset,
) -> Result<AlbumAnchor, GainError>

/// Generate one GainRecommendationMap per input AnalysisResult, in the same order as results.
pub fn generate_album_recommendations(
    results: &[AnalysisResult],
    anchor: &AlbumAnchor,
) -> Result<Vec<GainRecommendationMap>, GainError>
```

`analyze_album_files` returns `Vec<Result<AnalysisResult, GainError>>` with one entry per input path, preserving order. There is no outer `Result` — the function itself cannot fail, only individual files can. Callers do:
```rust
let all = analyze_album_files(&paths);
let good: Vec<&AnalysisResult> = all.iter().filter_map(|r| r.as_ref().ok()).collect();
let bad: Vec<&GainError>       = all.iter().filter_map(|r| r.as_ref().err()).collect();
```

This is FFI-friendlier than `BatchPartialFailure` (no recursive struct, no index-based error introspection) and caller-side failure handling is idiomatic Rust.

`compute_album_anchor` returns `GainError::AnalysisFailure` if no tracks have verified LUFS. Default anchor method: `AlbumAnchorMethod::MedianLufs`.

`generate_album_recommendations` produces `gain_db = anchor.reference_lufs − result.measurements.integrated_lufs.value` per file.

---

## FFI Changes

### Unchanged (Phase 2)

All Phase 2 FFI functions remain valid:
- `gain_stage_analyze` (stub entry point)
- `gain_stage_analyze_file(path, preset) → GainStageMap*`
- `gain_stage_free_map`
- `gain_stage_map_region_count` (returns `recommendations.len()`)
- `gain_stage_map_get_region` (C struct fields sourced from `region.analysis.*` and `region.decision.*`)
- `gain_stage_map_version`
- `gain_stage_last_error_code` (codes unchanged from Phase 2)
- `gain_stage_last_error_message`

### New in Phase 3

```c
/* Two-step analysis-first entry points */
GainStageAnalysis* gain_stage_begin_analysis(const char* path);
GainStageMap*      gain_stage_generate_recommendation(GainStageAnalysis* analysis, uint8_t preset);
void               gain_stage_free_analysis(GainStageAnalysis* analysis);

/* Per-region access from two-step path — enriched struct */
uint32_t           gain_stage_map_recommendation_count(const GainStageMap* map);
GainStageRegionV2  gain_stage_map_get_recommendation(const GainStageMap* map, uint32_t index);
```

`GainStageRegionV2` C struct:

```c
typedef struct {
    double  start_time;
    double  end_time;
    float   gain_db;
    float   confidence;
    float   classification_confidence;
    uint8_t region_type;
    uint8_t content_class;
    uint8_t is_applicable;   /* 0 = no recommendation (silence); 1 = recommendation valid */
} GainStageRegionV2;
```

`reason` is **excluded from the C struct in Phase 3.** The thread-local static string pattern used in Phase 2 for `gain_stage_last_error_message` is unsafe in multithreaded DAW hosts: if two calls interleave on different threads, the string pointer may be silently invalidated or read from the wrong thread's buffer. Extending this pattern to per-region reason strings in Phase 3 would compound the problem. Reason strings are diagnostic information useful at the Rust layer; Phase 4 will introduce proper string ownership semantics (heap-allocated, caller-freed via `gain_stage_free_string()`) when ARA integration requires them across the boundary.

`content_class` byte mapping (add `GAIN_STAGE_CLASS_*` constants to C header):

| Constant | Value |
|---|---|
| `GAIN_STAGE_CLASS_SILENCE` | 0 |
| `GAIN_STAGE_CLASS_DIALOGUE` | 1 |
| `GAIN_STAGE_CLASS_MUSIC` | 2 |
| `GAIN_STAGE_CLASS_AMBIENCE` | 3 |
| `GAIN_STAGE_CLASS_PERCUSSIVE` | 4 |
| `GAIN_STAGE_CLASS_MIXED` | 5 |
| `GAIN_STAGE_CLASS_UNKNOWN` | 6 |

`region_type` byte mapping for Phase 3 (`GAIN_STAGE_REGION_*` constants; `Transient` and `EnvelopeControlled` removed):

| Constant | Value |
|---|---|
| `GAIN_STAGE_REGION_STABLE` | 0 |
| `GAIN_STAGE_REGION_SILENCE` | 1 |
| `GAIN_STAGE_REGION_DIALOGUE` | 2 |
| `GAIN_STAGE_REGION_MUSIC` | 3 |
| `GAIN_STAGE_REGION_AMBIENCE` | 4 |
| `GAIN_STAGE_REGION_PERCUSSIVE` | 5 |
| `GAIN_STAGE_REGION_MIXED` | 6 |
| `GAIN_STAGE_REGION_UNKNOWN` | 7 |
| `GAIN_STAGE_REGION_TRANSIENT` | *(deprecated v2)* |
| `GAIN_STAGE_REGION_ENVELOPE_CONTROLLED` | *(deprecated v2)* |

Phase 3 preset constants (extending Phase 2 `GAIN_STAGE_PRESET_*`):

| Constant | Value | Measure | Target |
|---|---|---|---|
| `GAIN_STAGE_PRESET_DIALOGUE_BROADCAST` | 6 | LUFS | −24 |
| `GAIN_STAGE_PRESET_PODCAST_PREP` | 7 | LUFS | −16 |
| `GAIN_STAGE_PRESET_VOICEOVER_PREP` | 8 | LUFS | −19 |
| `GAIN_STAGE_PRESET_MUSIC_STEM_PREP` | 9 | Peak | −12 |
| `GAIN_STAGE_PRESET_FILM_DIALOGUE` | 10 | LUFS | −27 |
| `GAIN_STAGE_PRESET_ALBUM_CONSISTENCY` | 11 | LUFS | anchor |

No new FFI error codes. Batch errors are surfaced in Rust return types, not the FFI error channel.

---

## Testing Strategy

### Tier 1 — Pure math (no files)

- `analysis`: synthetic buffers; assert `integrated_lufs ± 0.2 LU` against pre-computed reference values; assert `short_term_lufs_peak >= integrated_lufs` (mathematical invariant)
- `segmentation`: synthesized frame energies; assert correct segment boundary positions ± 1 frame
- `classification`: one test per rule; one test per borderline case (confidence < 1.0); verify `Unknown` fallback
- `gain_decision`: `recommend_regions` with synthetic `[RegionAnalysis]`; silence regions → `gain_db = 0.0`; assert `recommendations.len() == regions.len()`

### Tier 2 — File I/O (generated in test setup)

Extend Phase 2 tempfile approach with:
- Silent WAV → single `Silence` region
- Amplitude-modulated sine → `Dialogue`-classified region
- Dense sine → `Music`-classified region
- Concatenated silence + dense sine → 2 regions (Silence + Music)

### Tier 3 — Loudness validation

Integration tests compare `integrated_lufs` against expected ± 0.2 LU. Reference values sourced from `ffmpeg -af ebur128` output (which uses libebur128 internally — same reference implementation as the `ebur128` crate). This constitutes BS.1770-4 compliance verification.

True Peak validated against expected ± 0.2 dBTP on known test signals.

### Tier 4 — Album consistency

`analyze_album_files` → `compute_album_anchor` → `generate_album_recommendations` end-to-end with 3 generated files at known loudness levels. Assert anchor equals median LUFS of inputs. Assert each output `gain_db = anchor.reference_lufs − file.integrated_lufs`.

### Tier 5 — Real-world edge case corpus

Segmentation and ebur128 edge interactions are not caught by synthetic signals. A corpus of adversarial audio cases must be exercised before Phase 3 is considered complete:

- **Clipped audio:** samples at exactly ±1.0 or above (NaN/Inf propagation check)
- **DC offset:** non-zero mean signal (constant bias shifts RMS without moving peak)
- **Interleaved silence bursts:** alternating loud/silent 100ms windows (segmentation boundary stress test)
- **Heavily compressed material:** <3 dB crest factor (classification rule 4/Music boundary case)
- **Cinematic soft dialogue over ambience:** low RMS dialogue + ambient bed (Dialogue vs Ambience boundary)
- **Malformed CAF headers:** valid audio data, corrupt chunk headers (symphonia error propagation)
- **32-bit float with extreme values:** subnormal floats, ±INF samples

These cases are exercised as unit tests with programmatically generated inputs, not committed binary fixtures. The `segmentation` and `classification` crates must handle all cases without panicking or returning `InternalError` when the audio is technically valid.

### test-assets/ (activated)

Required files (generated by `scripts/gen_test_assets.py`; not committed as binaries):

- `sine_440hz_-18dBFS_5s.wav` — stable tone, known loudness
- `silence_2s.wav` — pure digital silence
- `speech_like_-23lufs.wav` — amplitude-modulated content
- `music_like_-14lufs.wav` — dense sine, high RMS

All: WAV, 44100 Hz, 32-bit float, mono.

---

## Performance Requirements

- Analysis speed: ≥ 10× real-time on a modern desktop CPU
- No duplicate buffer allocations: `measure_region` slices existing buffer; `ebur128` processes in-place
- `analyze_album_files` is sequential in Phase 3; parallelism deferred to Phase 4

---

## What Does Not Change

- `analyze_file()` and `generate_recommendation()` signatures — unchanged
- `AudioMetadata`, `AudioBuffer`, `ContainerFormat`, `AnalysisResult` — unchanged
- `ffi_guard` catch_unwind wrapper — unchanged
- Phase 2 FFI function signatures — unchanged
- `GAIN_MAP_SCHEMA_VERSION` constant name — unchanged (value bumps to 2)
- `GainError` variants — unchanged from Phase 2 (no new variants added)
- `MeasurementQuality` enum — unchanged
- All `// SAFETY:` constraints — unchanged
- ADR-001 through ADR-007 — unchanged

---

## Architecture Constraints (carried forward)

- All `unsafe` blocks require `// SAFETY:` comment
- No global mutable state in Rust
- No exceptions cross the FFI boundary
- `gain-standalone` and `gain-ara` may only import `gain-api`
- No `unwrap()` in production code paths
- `reason` strings on `RegionDecision` are non-parsable and must never be used for control flow

---

## New ADRs

**ADR-008: ebur128 for BS.1770-4 loudness and True Peak**

Use `ebur128` (wrapping libebur128) rather than a from-scratch Rust BS.1770-4 implementation. Rationale: libebur128 is the reference implementation; using it eliminates correctness risk and months of filter-design validation work. Trade-off: adds a C dependency that must be statically linked in the ARA plugin build (Phase 4 concern).

**Containment constraint (enforced now, critical for Phase 4):** libebur128 types, symbols, and header includes must not appear on any public surface of `gain-api` or cross the FFI boundary. `ebur128` is an implementation detail of the `analysis` crate only. `gain-api` exposes `Measurements` (pure Rust data); it must never expose or re-export any `ebur128::*` type. ARA plugins are C++ and tend to inherit C dependency headers transitively — strict containment prevents symbol conflicts and simplifies the Phase 4 build.

**ADR-009: gain_decision remains data-only through Phase 3**

`gain_decision` depends only on `gain_map + gain-error`. The pipeline boundary between the analysis stack and the decision engine is `RegionAnalysis`. Rationale: preserves `gain_decision` reusability for ARA batch contexts and future server deployments that supply pre-computed `RegionAnalysis` data. Resolves the Phase 2 future watch item on structural scaling pressure.

---

## Future Watch Items

**`gain_map` structural split — not optional cleanup, required before Phase 4 ships:** By Phase 3 end, `gain_map` is acting as DSP domain model, analysis model, decision model, and protocol/FFI schema simultaneously. That is four abstraction layers in one crate. When ARA and album workflows land in Phase 4, dependency inversion will collapse if these layers are not separated. The split is not architectural cleanup — it is the prerequisite for Phase 4 not creating circular dependencies.

Phase 4 must introduce a formal split into four crates before ARA integration begins:
- `gain-dsp` — `Measurements`, `AudioBuffer` abstractions, ebur128 wrappers
- `gain-analysis` — `RegionAnalysis`, `AnalysisBundle`, segmentation/classification types
- `gain-decision` — presets, `RegionDecision`, `GainRegion`, gain math
- `gain-protocol` — `GainRecommendationMap`, FFI structs, schema versioning, `AlbumAnchor`

Phase 3 naming is chosen to make this split clean. No circular dependencies will be created by it. If this split is deferred past Phase 4 start, treat it as a Phase 4 blocker.

**`RegionAnalysis` internal split:** Currently `RegionAnalysis` bundles DSP measurements, segmentation timing, classification output, and confidence scoring. For Phase 4 (ARA region editing, user overrides, alternate classifications), consider splitting into:
```rust
struct RegionFeatures { measurements, start_time, end_time }
struct RegionInference { content_class, region_type, classification_confidence }
pub struct RegionAnalysis { features: RegionFeatures, inference: RegionInference }
```
The public API doesn't change; internal mutability and Phase 4 override semantics become cleaner.

**Classification vector (Phase 4):** Phase 3 classification returns a single winning `(ContentClass, f32)`. Borderline audio (e.g., speech over a music bed) will produce `Mixed` or oscillate between `Dialogue` and `Music` depending on subtle mastering differences. Phase 4 should change `classify()` to return `Vec<(ContentClass, f32)>` (per-class scores), stored as `classification_scores` on `RegionAnalysis`. Phase 3 uses only the top result; Phase 4 ML training and UI use the full vector.

**Segmentation adaptivity (Phase 4):** The Phase 3 segmentation constants (−60 dBFS silence threshold, 6 dB change threshold) are fixed. Heavily compressed material, cinematic audio with embedded ambience, and podcasts with music beds will produce over-segmentation or under-segmentation. Phase 4 should adapt segmentation strategy based on file-level content classification: detect the dominant `ContentClass` from a quick whole-file pass first, then apply content-aware thresholds.

**Classification rule maintenance:** Phase 3 thresholds are conservative and well-separated. As real audio fixtures accumulate, rules will need threshold refinement. Rules and documentation (`docs/GainDecisionModel.md`) must be updated together — neither is authoritative without the other.

**ebur128 in ARA packaging:** `ebur128` wraps a C library. ARA plugin packaging must statically link `libebur128`. Verify in the Phase 4 build pipeline before assuming it drops in.

**Album anchor Phase 4 extension:** `AlbumAnchorMethod::ReferenceTrack` is deferred. When added, extend via a struct-based variant rather than expanding the byte-flag space — the same lesson as the FFI `Custom` preset deferral.

**Album analysis scalability:** `analyze_album_files` is synchronous and sequential. For large catalogs, rayon parallelism at the `gain-api` level is the natural extension point. Phase 4.

**`GainMapDto` update:** The Tauri `GainMapDto` in `gain-standalone` must add `is_applicable: bool` to its `GainRegionDto` serialization mirror to expose `RegionDecision.is_applicable` to the frontend.
