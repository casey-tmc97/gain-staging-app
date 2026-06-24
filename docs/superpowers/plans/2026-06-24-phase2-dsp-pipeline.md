# Phase 2 DSP Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace every Phase 1 stub with real audio analysis and preset-based gain recommendation, wiring the full `gain-error → gain_map → audio_ingestion → analysis → gain_decision → gain-api → ffi → gain-standalone` stack end-to-end.

**Architecture:** Two-step public API — `analyze_file()` decodes audio and measures Peak/RMS/CrestFactor; `generate_recommendation()` applies a preset to produce a `GainRecommendationMap`. All internal crates communicate through `gain_map` types to avoid diamond dependencies. LUFS is always a `Placeholder` in Phase 2.

**Tech Stack:** Rust 2021 edition, Symphonia 0.5 (`wav`, `aiff`, `pcm`), Tauri 2, tempfile 3 (dev), TDD throughout.

## Global Constraints

- `SILENCE_FLOOR_DBFS = -120.0` — never use `f32::MIN_POSITIVE` as a silence clamp
- `integrated_lufs.value` MUST be `None`; `quality` MUST be `MeasurementQuality::Placeholder` — no fake LUFS
- `reason` strings are human-readable only — never encode identity or control flow in them; `PresetId` carries identity
- All `unsafe` blocks require a `// SAFETY:` comment
- No `unwrap()` in production code paths — use `?` or explicit `match`
- No global mutable state in Rust
- `gain_decision` MUST NOT import from `analysis` — dependency graph is enforced
- Phase 2 output: exactly 1 `GainRegion` per file, `RegionType::Stable`, `confidence: 1.0`
- `gain-standalone` and `gain-ara` import only `gain-api` (ADR-005)

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `gain-core/Cargo.toml` | Modify | Add `gain-error` to workspace members |
| `gain-core/crates/gain-error/Cargo.toml` | Create | New crate, no deps |
| `gain-core/crates/gain-error/src/lib.rs` | Create | Named struct GainError variants + Display + Error |
| `gain-core/crates/gain_map/src/lib.rs` | Modify | Add MeasurementQuality/Value/Measurements/PresetId; update GainRecommendationMap |
| `gain-core/crates/audio_ingestion/Cargo.toml` | Modify | Add symphonia + gain-error deps |
| `gain-core/crates/audio_ingestion/src/lib.rs` | Replace | Real symphonia-based loader returning (AudioBuffer, AudioMetadata) |
| `gain-core/crates/analysis/Cargo.toml` | Modify | Add audio_ingestion + gain_map + gain-error deps |
| `gain-core/crates/analysis/src/lib.rs` | Replace | Real Peak/RMS/CrestFactor measurement (old modules become dead) |
| `gain-core/crates/gain_decision/Cargo.toml` | Modify | Add gain-error dep |
| `gain-core/crates/gain_decision/src/lib.rs` | Replace | MeasureType enum + recommend() function |
| `gain-core/crates/gain-api/Cargo.toml` | Modify | Add gain-error, audio_ingestion, analysis deps |
| `gain-core/crates/gain-api/src/lib.rs` | Replace | Two-step public API + re-exports; remove stub tests |
| `gain-core/crates/gain-api/tests/pipeline.rs` | Create | End-to-end integration test through real file |
| `gain-core/crates/ffi/src/lib.rs` | Modify | Wire gain_stage_analyze; add file entry point + error introspection |
| `gain-core/crates/ffi/include/gain_stage_ffi.h` | Modify | Add preset/error constants + 3 new function declarations |
| `gain-standalone/src-tauri/src/commands/analyze.rs` | Modify | Two-step API call with optional preset |
| `gain-standalone/src-tauri/src/dto.rs` | Modify | Add preset_used field to GainMapDto |

---

## Task 1: `gain-error` Crate

**Files:**
- Create: `gain-core/crates/gain-error/Cargo.toml`
- Create: `gain-core/crates/gain-error/src/lib.rs`
- Modify: `gain-core/Cargo.toml`

**Interfaces:**
- Produces: `pub enum GainError { FileNotFound { path }, UnsupportedFormat { format }, DecodeFailure { details }, InvalidAudio { details }, AnalysisFailure { details }, InternalError { details } }` — used by all subsequent tasks

---

- [ ] **Step 1: Write the failing test**

Create `gain-core/crates/gain-error/src/lib.rs` with only the test module — no implementation yet:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn gain_error_variants_display() {
        use crate::GainError;
        assert!(GainError::FileNotFound { path: "/a.wav".into() }.to_string().contains("/a.wav"));
        assert!(GainError::UnsupportedFormat { format: ".mp3".into() }.to_string().contains(".mp3"));
        assert!(GainError::DecodeFailure { details: "eof".into() }.to_string().contains("eof"));
        assert!(GainError::InvalidAudio { details: "nan".into() }.to_string().contains("nan"));
        assert!(GainError::AnalysisFailure { details: "empty".into() }.to_string().contains("empty"));
        assert!(GainError::InternalError { details: "oops".into() }.to_string().contains("oops"));
    }
}
```

Create `gain-core/crates/gain-error/Cargo.toml`:

```toml
[package]
name = "gain-error"
version = "0.1.0"
edition = "2021"
```

Add to `gain-core/Cargo.toml` members list:

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
    "crates/gain-api",
    "crates/ffi",
    "crates/gain-error",
]
```

- [ ] **Step 2: Run test to verify it fails**

```
cd gain-core && cargo test -p gain-error
```

Expected: compile error — `GainError` not defined.

- [ ] **Step 3: Implement**

Replace the contents of `gain-core/crates/gain-error/src/lib.rs`:

```rust
#[derive(Debug)]
pub enum GainError {
    FileNotFound    { path: String },
    UnsupportedFormat { format: String },
    DecodeFailure   { details: String },
    InvalidAudio    { details: String },
    AnalysisFailure { details: String },
    InternalError   { details: String },
}

impl std::fmt::Display for GainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GainError::FileNotFound    { path }    => write!(f, "file not found: {path}"),
            GainError::UnsupportedFormat { format } => write!(f, "unsupported format: {format}"),
            GainError::DecodeFailure   { details } => write!(f, "decode failure: {details}"),
            GainError::InvalidAudio    { details } => write!(f, "invalid audio: {details}"),
            GainError::AnalysisFailure { details } => write!(f, "analysis failure: {details}"),
            GainError::InternalError   { details } => write!(f, "internal error: {details}"),
        }
    }
}

impl std::error::Error for GainError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_error_variants_display() {
        assert!(GainError::FileNotFound    { path: "/a.wav".into() }.to_string().contains("/a.wav"));
        assert!(GainError::UnsupportedFormat { format: ".mp3".into() }.to_string().contains(".mp3"));
        assert!(GainError::DecodeFailure   { details: "eof".into() }.to_string().contains("eof"));
        assert!(GainError::InvalidAudio    { details: "nan".into() }.to_string().contains("nan"));
        assert!(GainError::AnalysisFailure { details: "empty".into() }.to_string().contains("empty"));
        assert!(GainError::InternalError   { details: "oops".into() }.to_string().contains("oops"));
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

```
cd gain-core && cargo test -p gain-error
```

Expected: `test tests::gain_error_variants_display ... ok`

- [ ] **Step 5: Commit**

```bash
git add gain-core/Cargo.toml gain-core/crates/gain-error/
git commit -m "feat: add gain-error crate with named struct variants"
```

---

## Task 2: `gain_map` Type Expansion

**Files:**
- Modify: `gain-core/crates/gain_map/src/lib.rs`

**Interfaces:**
- Consumes: nothing new
- Produces:
  - `pub enum MeasurementQuality { Placeholder, Estimated, Verified }`
  - `pub struct MeasurementValue { pub value: Option<f32>, pub quality: MeasurementQuality }`
  - `pub struct Measurements { pub peak_dbfs: f32, pub rms_dbfs: f32, pub crest_factor_db: f32, pub integrated_lufs: MeasurementValue }`
  - `pub enum PresetId { MixPrepConservative, MixPrepStandard, MixPrepAggressive, AnalogConsole, AnalogConsoleHot, DialoguePrep, Custom }`
  - Updated: `GainRecommendationMap` gains `pub preset_used: Option<PresetId>`; `Default` produces `preset_used: None`

---

- [ ] **Step 1: Write the failing tests**

Add these tests to the existing `tests` module in `gain-core/crates/gain_map/src/lib.rs`:

```rust
#[test]
fn measurement_quality_placeholder_exists() {
    let q = MeasurementQuality::Placeholder;
    assert_eq!(q, MeasurementQuality::Placeholder);
}

#[test]
fn measurement_value_none_when_placeholder() {
    let v = MeasurementValue { value: None, quality: MeasurementQuality::Placeholder };
    assert!(v.value.is_none());
}

#[test]
fn measurements_struct_accessible() {
    let m = Measurements {
        peak_dbfs: -12.0,
        rms_dbfs: -18.0,
        crest_factor_db: 6.0,
        integrated_lufs: MeasurementValue { value: None, quality: MeasurementQuality::Placeholder },
    };
    assert_eq!(m.peak_dbfs, -12.0);
    assert_eq!(m.crest_factor_db, 6.0);
    assert!(m.integrated_lufs.value.is_none());
}

#[test]
fn preset_id_variants_exist() {
    let _ = PresetId::MixPrepConservative;
    let _ = PresetId::MixPrepStandard;
    let _ = PresetId::MixPrepAggressive;
    let _ = PresetId::AnalogConsole;
    let _ = PresetId::AnalogConsoleHot;
    let _ = PresetId::DialoguePrep;
    let _ = PresetId::Custom;
}

#[test]
fn gain_recommendation_map_default_has_no_preset() {
    let map = GainRecommendationMap::default();
    assert_eq!(map.preset_used, None);
}
```

- [ ] **Step 2: Run tests to verify they fail**

```
cd gain-core && cargo test -p gain_map
```

Expected: compile errors — new types not yet defined.

- [ ] **Step 3: Implement**

Replace the full contents of `gain-core/crates/gain_map/src/lib.rs`:

```rust
pub const GAIN_MAP_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq)]
pub enum MeasurementQuality {
    Placeholder,
    Estimated,
    Verified,
}

#[derive(Debug, Clone)]
pub struct MeasurementValue {
    pub value: Option<f32>,
    pub quality: MeasurementQuality,
}

#[derive(Debug, Clone)]
pub struct Measurements {
    pub peak_dbfs: f32,
    pub rms_dbfs: f32,
    pub crest_factor_db: f32,
    pub integrated_lufs: MeasurementValue,
}

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

#[derive(Debug, Clone, PartialEq)]
pub enum RegionType {
    Stable,
    Transient,
    EnvelopeControlled,
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

#[derive(Debug, Clone)]
pub struct GainRecommendationMap {
    pub version: u32,
    pub preset_used: Option<PresetId>,
    pub regions: Vec<GainRegion>,
}

impl Default for GainRecommendationMap {
    fn default() -> Self {
        Self { version: GAIN_MAP_SCHEMA_VERSION, preset_used: None, regions: Vec::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_region_fields_are_accessible() {
        let region = GainRegion {
            start_time: 0.0, end_time: 1.5, gain_db: -3.0,
            confidence: 0.85, region_type: RegionType::Stable, reason: "test".to_string(),
        };
        assert_eq!(region.start_time, 0.0);
        assert_eq!(region.gain_db, -3.0);
        assert_eq!(region.reason, "test");
    }

    #[test]
    fn gain_recommendation_map_default_is_empty() {
        let map = GainRecommendationMap::default();
        assert!(map.regions.is_empty());
    }

    #[test]
    fn gain_recommendation_map_default_has_no_preset() {
        let map = GainRecommendationMap::default();
        assert_eq!(map.preset_used, None);
    }

    #[test]
    fn gain_recommendation_map_can_hold_regions() {
        let mut map = GainRecommendationMap::default();
        map.regions.push(GainRegion {
            start_time: 0.0, end_time: 2.0, gain_db: 6.0,
            confidence: 1.0, region_type: RegionType::Transient, reason: "peak".to_string(),
        });
        assert_eq!(map.regions.len(), 1);
    }

    #[test]
    fn region_type_envelope_controlled_exists() {
        assert_eq!(RegionType::EnvelopeControlled, RegionType::EnvelopeControlled);
    }

    #[test]
    fn gain_recommendation_map_default_version_is_one() {
        assert_eq!(GainRecommendationMap::default().version, 1);
    }

    #[test]
    fn measurement_quality_placeholder_exists() {
        assert_eq!(MeasurementQuality::Placeholder, MeasurementQuality::Placeholder);
    }

    #[test]
    fn measurement_value_none_when_placeholder() {
        let v = MeasurementValue { value: None, quality: MeasurementQuality::Placeholder };
        assert!(v.value.is_none());
    }

    #[test]
    fn measurements_struct_accessible() {
        let m = Measurements {
            peak_dbfs: -12.0, rms_dbfs: -18.0, crest_factor_db: 6.0,
            integrated_lufs: MeasurementValue { value: None, quality: MeasurementQuality::Placeholder },
        };
        assert_eq!(m.peak_dbfs, -12.0);
        assert!(m.integrated_lufs.value.is_none());
    }

    #[test]
    fn preset_id_variants_exist() {
        let _ = PresetId::MixPrepConservative;
        let _ = PresetId::MixPrepStandard;
        let _ = PresetId::MixPrepAggressive;
        let _ = PresetId::AnalogConsole;
        let _ = PresetId::AnalogConsoleHot;
        let _ = PresetId::DialoguePrep;
        let _ = PresetId::Custom;
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

```
cd gain-core && cargo test -p gain_map
```

Expected: all 10 tests pass.

- [ ] **Step 5: Commit**

```bash
git add gain-core/crates/gain_map/src/lib.rs
git commit -m "feat: add Measurements, MeasurementValue, MeasurementQuality, PresetId to gain_map; add preset_used to GainRecommendationMap"
```

---

## Task 3: `audio_ingestion` — Symphonia-Based Loader

**Files:**
- Modify: `gain-core/crates/audio_ingestion/Cargo.toml`
- Replace: `gain-core/crates/audio_ingestion/src/lib.rs`

**Interfaces:**
- Consumes: `gain-error::GainError`
- Produces:
  - `pub struct AudioBuffer { pub samples: Vec<f32>, pub sample_rate: u32, pub channels: u16 }`
  - `pub enum ContainerFormat { Wav, Aiff }`
  - `pub struct AudioMetadata { pub duration_secs: f64, pub sample_rate: u32, pub channels: u16, pub format: ContainerFormat }`
  - `pub fn load_file(path: &std::path::Path) -> Result<(AudioBuffer, AudioMetadata), GainError>`

---

- [ ] **Step 1: Update Cargo.toml**

Replace `gain-core/crates/audio_ingestion/Cargo.toml`:

```toml
[package]
name = "audio_ingestion"
version = "0.1.0"
edition = "2021"

[dependencies]
gain-error = { path = "../gain-error" }
symphonia  = { version = "0.5", features = ["wav", "aiff", "pcm"] }

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Write the failing tests**

Replace `gain-core/crates/audio_ingestion/src/lib.rs` with only the test module and stub types:

```rust
pub struct AudioBuffer {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone)]
pub enum ContainerFormat { Wav, Aiff }

pub struct AudioMetadata {
    pub duration_secs: f64,
    pub sample_rate: u32,
    pub channels: u16,
    pub format: ContainerFormat,
}

pub fn load_file(_path: &std::path::Path) -> Result<(AudioBuffer, AudioMetadata), gain_error::GainError> {
    Err(gain_error::GainError::DecodeFailure { details: "not implemented".to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_wav(samples_i16: &[i16], sample_rate: u32) -> Vec<u8> {
        let data_len = samples_i16.len() * 2;
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36u32 + data_len as u32).to_le_bytes());
        b.extend_from_slice(b"WAVE");
        b.extend_from_slice(b"fmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());       // PCM
        b.extend_from_slice(&1u16.to_le_bytes());       // mono
        b.extend_from_slice(&sample_rate.to_le_bytes());
        b.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());       // block align
        b.extend_from_slice(&16u16.to_le_bytes());      // bits per sample
        b.extend_from_slice(b"data");
        b.extend_from_slice(&(data_len as u32).to_le_bytes());
        for s in samples_i16 { b.extend_from_slice(&s.to_le_bytes()); }
        b
    }

    fn make_aiff(samples_i16: &[i16], sample_rate: u32) -> Vec<u8> {
        let n_frames = samples_i16.len() as u32;
        let data_len = samples_i16.len() * 2;
        let ssnd_size = 8u32 + data_len as u32;
        let form_size = 4 + 26 + 8 + ssnd_size;
        let sr_bytes: [u8; 10] = match sample_rate {
            44100 => [0x40, 0x0E, 0xAC, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            48000 => [0x40, 0x0E, 0xBB, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            _ => panic!("unsupported sample rate in test helper"),
        };
        let mut b = Vec::new();
        b.extend_from_slice(b"FORM");
        b.extend_from_slice(&form_size.to_be_bytes());
        b.extend_from_slice(b"AIFF");
        b.extend_from_slice(b"COMM");
        b.extend_from_slice(&18u32.to_be_bytes());
        b.extend_from_slice(&1u16.to_be_bytes());       // numChannels
        b.extend_from_slice(&n_frames.to_be_bytes());
        b.extend_from_slice(&16u16.to_be_bytes());      // sampleSize
        b.extend_from_slice(&sr_bytes);
        b.extend_from_slice(b"SSND");
        b.extend_from_slice(&ssnd_size.to_be_bytes());
        b.extend_from_slice(&0u32.to_be_bytes());       // offset
        b.extend_from_slice(&0u32.to_be_bytes());       // blockSize
        for s in samples_i16 { b.extend_from_slice(&s.to_be_bytes()); }
        b
    }

    #[test]
    fn load_wav_returns_samples_and_metadata() {
        let samples_i16: Vec<i16> = vec![16383; 100]; // ≈ 0.5 amplitude
        let bytes = make_wav(&samples_i16, 44100);
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(&bytes).unwrap();
        let (buf, meta) = load_file(f.path()).unwrap();
        assert_eq!(meta.sample_rate, 44100);
        assert_eq!(meta.channels, 1);
        assert_eq!(buf.samples.len(), 100);
        for s in &buf.samples {
            assert!((s - 0.5).abs() < 0.002, "sample {s} not near 0.5");
        }
    }

    #[test]
    fn load_aiff_returns_samples_and_metadata() {
        let samples_i16: Vec<i16> = vec![16383; 50];
        let bytes = make_aiff(&samples_i16, 44100);
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(&bytes).unwrap();
        let (buf, meta) = load_file(f.path()).unwrap();
        assert_eq!(meta.sample_rate, 44100);
        assert_eq!(buf.samples.len(), 50);
        for s in &buf.samples {
            assert!((s - 0.5).abs() < 0.002, "sample {s} not near 0.5");
        }
    }

    #[test]
    fn load_missing_file_returns_file_not_found() {
        let result = load_file(std::path::Path::new("/no/such/file.wav"));
        assert!(matches!(result, Err(gain_error::GainError::FileNotFound { .. })));
    }

    #[test]
    fn load_unsupported_extension_returns_error() {
        let mut f = tempfile::NamedTempFile::with_suffix(".mp3").unwrap();
        f.write_all(b"not audio").unwrap();
        let result = load_file(f.path());
        assert!(matches!(result, Err(gain_error::GainError::UnsupportedFormat { .. })));
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

```
cd gain-core && cargo test -p audio_ingestion
```

Expected: `load_wav_returns_samples_and_metadata` and `load_aiff_returns_samples_and_metadata` fail (stub returns error); missing file and unsupported format tests may pass accidentally — that's fine.

- [ ] **Step 4: Implement**

Replace the full contents of `gain-core/crates/audio_ingestion/src/lib.rs`:

```rust
use gain_error::GainError;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct AudioBuffer {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone)]
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

pub fn load_file(path: &std::path::Path) -> Result<(AudioBuffer, AudioMetadata), GainError> {
    if !path.exists() {
        return Err(GainError::FileNotFound { path: path.to_string_lossy().into_owned() });
    }

    let ext = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase());
    let container = match ext.as_deref() {
        Some("wav")             => ContainerFormat::Wav,
        Some("aif") | Some("aiff") => ContainerFormat::Aiff,
        other => return Err(GainError::UnsupportedFormat {
            format: other.unwrap_or("(no extension)").to_string(),
        }),
    };

    let file = std::fs::File::open(path)
        .map_err(|e| GainError::DecodeFailure { details: e.to_string() })?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext_str) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext_str);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| GainError::UnsupportedFormat { format: e.to_string() })?;

    let mut format = probed.format;
    let track = format.default_track()
        .ok_or_else(|| GainError::DecodeFailure { details: "no default track".to_string() })?;

    let track_id  = track.id;
    let sample_rate = track.codec_params.sample_rate
        .ok_or_else(|| GainError::DecodeFailure { details: "missing sample rate".to_string() })?;
    let channels = track.codec_params.channels
        .ok_or_else(|| GainError::DecodeFailure { details: "missing channel info".to_string() })?;
    let channel_count = channels.count() as u16;
    let n_frames = track.codec_params.n_frames;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| GainError::DecodeFailure { details: e.to_string() })?;

    let mut all_samples: Vec<f32> = Vec::new();
    if let Some(frames) = n_frames {
        all_samples.reserve(frames as usize * channel_count as usize);
    }

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(GainError::DecodeFailure { details: e.to_string() }),
        };

        if packet.track_id() != track_id { continue; }

        let decoded = decoder.decode(&packet)
            .map_err(|e| GainError::DecodeFailure { details: e.to_string() })?;

        let spec = *decoded.spec();
        let mut sample_buf = SampleBuffer::<f32>::new(decoded.frames() as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);
        all_samples.extend_from_slice(sample_buf.samples());
    }

    let total_frames = all_samples.len() as f64 / channel_count as f64;
    let duration_secs = total_frames / sample_rate as f64;

    let buf  = AudioBuffer  { samples: all_samples, sample_rate, channels: channel_count };
    let meta = AudioMetadata { duration_secs, sample_rate, channels: channel_count, format: container };
    Ok((buf, meta))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_wav(samples_i16: &[i16], sample_rate: u32) -> Vec<u8> {
        let data_len = samples_i16.len() * 2;
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36u32 + data_len as u32).to_le_bytes());
        b.extend_from_slice(b"WAVE");
        b.extend_from_slice(b"fmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&sample_rate.to_le_bytes());
        b.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&(data_len as u32).to_le_bytes());
        for s in samples_i16 { b.extend_from_slice(&s.to_le_bytes()); }
        b
    }

    fn make_aiff(samples_i16: &[i16], sample_rate: u32) -> Vec<u8> {
        let n_frames = samples_i16.len() as u32;
        let data_len = samples_i16.len() * 2;
        let ssnd_size = 8u32 + data_len as u32;
        let form_size = 4 + 26 + 8 + ssnd_size;
        let sr_bytes: [u8; 10] = match sample_rate {
            44100 => [0x40, 0x0E, 0xAC, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            48000 => [0x40, 0x0E, 0xBB, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            _ => panic!("unsupported sample rate in test helper"),
        };
        let mut b = Vec::new();
        b.extend_from_slice(b"FORM");
        b.extend_from_slice(&form_size.to_be_bytes());
        b.extend_from_slice(b"AIFF");
        b.extend_from_slice(b"COMM");
        b.extend_from_slice(&18u32.to_be_bytes());
        b.extend_from_slice(&1u16.to_be_bytes());
        b.extend_from_slice(&n_frames.to_be_bytes());
        b.extend_from_slice(&16u16.to_be_bytes());
        b.extend_from_slice(&sr_bytes);
        b.extend_from_slice(b"SSND");
        b.extend_from_slice(&ssnd_size.to_be_bytes());
        b.extend_from_slice(&0u32.to_be_bytes());
        b.extend_from_slice(&0u32.to_be_bytes());
        for s in samples_i16 { b.extend_from_slice(&s.to_be_bytes()); }
        b
    }

    #[test]
    fn load_wav_returns_samples_and_metadata() {
        let samples_i16: Vec<i16> = vec![16383; 100];
        let bytes = make_wav(&samples_i16, 44100);
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(&bytes).unwrap();
        let (buf, meta) = load_file(f.path()).unwrap();
        assert_eq!(meta.sample_rate, 44100);
        assert_eq!(meta.channels, 1);
        assert_eq!(buf.samples.len(), 100);
        for s in &buf.samples {
            assert!((s - 0.5).abs() < 0.002, "sample {s} not near 0.5");
        }
    }

    #[test]
    fn load_aiff_returns_samples_and_metadata() {
        let samples_i16: Vec<i16> = vec![16383; 50];
        let bytes = make_aiff(&samples_i16, 44100);
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(&bytes).unwrap();
        let (buf, meta) = load_file(f.path()).unwrap();
        assert_eq!(meta.sample_rate, 44100);
        assert_eq!(buf.samples.len(), 50);
        for s in &buf.samples {
            assert!((s - 0.5).abs() < 0.002, "sample {s} not near 0.5");
        }
    }

    #[test]
    fn load_missing_file_returns_file_not_found() {
        let result = load_file(std::path::Path::new("/no/such/file.wav"));
        assert!(matches!(result, Err(gain_error::GainError::FileNotFound { .. })));
    }

    #[test]
    fn load_unsupported_extension_returns_error() {
        let mut f = tempfile::NamedTempFile::with_suffix(".mp3").unwrap();
        f.write_all(b"not audio").unwrap();
        let result = load_file(f.path());
        assert!(matches!(result, Err(gain_error::GainError::UnsupportedFormat { .. })));
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

```
cd gain-core && cargo test -p audio_ingestion
```

Expected: all 4 tests pass. If symphonia returns `Estimated` vs. exact 0.5 f32 values, the tolerance `< 0.002` accounts for 16-bit quantization. If you see `UnexpectedEof` variant name mismatches, check the symphonia version's exact error variant — it may be `Error::ResetRequired` instead; consult `symphonia::core::errors::Error` enum docs.

- [ ] **Step 6: Commit**

```bash
git add gain-core/crates/audio_ingestion/
git commit -m "feat: implement audio_ingestion with symphonia WAV/AIFF decoding"
```

---

## Task 4: `analysis` — Peak / RMS / Crest Factor

**Files:**
- Modify: `gain-core/crates/analysis/Cargo.toml`
- Replace: `gain-core/crates/analysis/src/lib.rs` (the four stub module files — `loudness.rs`, `spectral.rs`, `transient.rs`, `envelope.rs` — become orphaned dead code; leave them in place, Cargo will not compile them)

**Interfaces:**
- Consumes: `audio_ingestion::AudioBuffer`, `gain_map::{Measurements, MeasurementValue, MeasurementQuality}`, `gain_error::GainError`
- Produces: `pub fn measure(buf: &AudioBuffer) -> Result<Measurements, GainError>`

---

- [ ] **Step 1: Update Cargo.toml**

Replace `gain-core/crates/analysis/Cargo.toml`:

```toml
[package]
name = "analysis"
version = "0.1.0"
edition = "2021"

[dependencies]
audio_ingestion = { path = "../audio_ingestion" }
gain_map        = { path = "../gain_map" }
gain-error      = { path = "../gain-error" }
```

- [ ] **Step 2: Write the failing tests**

Replace `gain-core/crates/analysis/src/lib.rs` with only an empty stub and the test module:

```rust
use audio_ingestion::AudioBuffer;
use gain_error::GainError;
use gain_map::{MeasurementQuality, MeasurementValue, Measurements};

pub fn measure(_buf: &AudioBuffer) -> Result<Measurements, GainError> {
    Err(GainError::AnalysisFailure { details: "not implemented".to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(samples: Vec<f32>) -> AudioBuffer {
        AudioBuffer { samples, sample_rate: 44100, channels: 1 }
    }

    #[test]
    fn full_scale_peak_is_zero_dbfs() {
        let result = measure(&buf(vec![1.0f32; 1024])).unwrap();
        assert!((result.peak_dbfs - 0.0).abs() < 0.001);
    }

    #[test]
    fn half_amplitude_peak_is_approx_minus_6() {
        let result = measure(&buf(vec![0.5f32; 1024])).unwrap();
        assert!((result.peak_dbfs - (-6.0206)).abs() < 0.001);
    }

    #[test]
    fn constant_signal_crest_factor_is_zero() {
        let result = measure(&buf(vec![0.5f32; 1024])).unwrap();
        assert!((result.crest_factor_db - 0.0).abs() < 0.001);
    }

    #[test]
    fn sine_wave_crest_factor_is_approx_3() {
        // sine peak = 1.0, rms = 1/sqrt(2) ≈ 0.7071 → rms_dbfs ≈ -3.0103
        let samples: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f32::consts::PI * i as f32 / 100.0).sin())
            .collect();
        let result = measure(&buf(samples)).unwrap();
        assert!((result.peak_dbfs - 0.0).abs() < 0.01);
        assert!((result.rms_dbfs - (-3.0103)).abs() < 0.05);
        assert!((result.crest_factor_db - 3.0103).abs() < 0.05);
    }

    #[test]
    fn silent_audio_returns_silence_floor() {
        let result = measure(&buf(vec![0.0f32; 1024])).unwrap();
        assert_eq!(result.peak_dbfs, -120.0);
        assert_eq!(result.rms_dbfs, -120.0);
        assert_eq!(result.crest_factor_db, 0.0);
    }

    #[test]
    fn lufs_is_always_placeholder_none() {
        let result = measure(&buf(vec![0.5f32; 1024])).unwrap();
        assert_eq!(result.integrated_lufs.quality, MeasurementQuality::Placeholder);
        assert!(result.integrated_lufs.value.is_none());
    }

    #[test]
    fn empty_buffer_returns_error() {
        let result = measure(&buf(vec![]));
        assert!(matches!(result, Err(GainError::InvalidAudio { .. })));
    }

    #[test]
    fn nan_sample_returns_error() {
        let result = measure(&buf(vec![f32::NAN]));
        assert!(matches!(result, Err(GainError::InvalidAudio { .. })));
    }

    #[test]
    fn out_of_range_sample_returns_error() {
        let result = measure(&buf(vec![1.5f32]));
        assert!(matches!(result, Err(GainError::InvalidAudio { .. })));
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

```
cd gain-core && cargo test -p analysis
```

Expected: most tests fail because stub returns `Err(AnalysisFailure)`.

- [ ] **Step 4: Implement**

Replace the full contents of `gain-core/crates/analysis/src/lib.rs`:

```rust
use audio_ingestion::AudioBuffer;
use gain_error::GainError;
use gain_map::{MeasurementQuality, MeasurementValue, Measurements};

const SILENCE_FLOOR_DBFS: f32 = -120.0;

pub fn measure(buf: &AudioBuffer) -> Result<Measurements, GainError> {
    if buf.samples.is_empty() {
        return Err(GainError::InvalidAudio { details: "empty audio buffer".to_string() });
    }

    for s in &buf.samples {
        if !s.is_finite() {
            return Err(GainError::InvalidAudio { details: format!("non-finite sample: {s}") });
        }
        if s.abs() > 1.0 {
            return Err(GainError::InvalidAudio { details: format!("sample out of [-1,1] range: {s}") });
        }
    }

    let max_amplitude = buf.samples.iter().map(|s| s.abs()).fold(0f32, f32::max);
    let peak_dbfs = if max_amplitude == 0.0 {
        SILENCE_FLOOR_DBFS
    } else {
        20.0 * max_amplitude.log10()
    };

    let n = buf.samples.len() as f32;
    let sum_sq: f32 = buf.samples.iter().map(|s| s * s).sum();
    let rms = (sum_sq / n).sqrt();
    let rms_dbfs = if rms == 0.0 {
        SILENCE_FLOOR_DBFS
    } else {
        20.0 * rms.log10()
    };

    let crest_factor_db = peak_dbfs - rms_dbfs;

    Ok(Measurements {
        peak_dbfs,
        rms_dbfs,
        crest_factor_db,
        integrated_lufs: MeasurementValue {
            value: None,
            quality: MeasurementQuality::Placeholder,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(samples: Vec<f32>) -> AudioBuffer {
        AudioBuffer { samples, sample_rate: 44100, channels: 1 }
    }

    #[test]
    fn full_scale_peak_is_zero_dbfs() {
        let result = measure(&buf(vec![1.0f32; 1024])).unwrap();
        assert!((result.peak_dbfs - 0.0).abs() < 0.001);
    }

    #[test]
    fn half_amplitude_peak_is_approx_minus_6() {
        let result = measure(&buf(vec![0.5f32; 1024])).unwrap();
        assert!((result.peak_dbfs - (-6.0206)).abs() < 0.001);
    }

    #[test]
    fn constant_signal_crest_factor_is_zero() {
        let result = measure(&buf(vec![0.5f32; 1024])).unwrap();
        assert!((result.crest_factor_db - 0.0).abs() < 0.001);
    }

    #[test]
    fn sine_wave_crest_factor_is_approx_3() {
        let samples: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f32::consts::PI * i as f32 / 100.0).sin())
            .collect();
        let result = measure(&buf(samples)).unwrap();
        assert!((result.peak_dbfs - 0.0).abs() < 0.01);
        assert!((result.rms_dbfs - (-3.0103)).abs() < 0.05);
        assert!((result.crest_factor_db - 3.0103).abs() < 0.05);
    }

    #[test]
    fn silent_audio_returns_silence_floor() {
        let result = measure(&buf(vec![0.0f32; 1024])).unwrap();
        assert_eq!(result.peak_dbfs, -120.0);
        assert_eq!(result.rms_dbfs, -120.0);
        assert_eq!(result.crest_factor_db, 0.0);
    }

    #[test]
    fn lufs_is_always_placeholder_none() {
        let result = measure(&buf(vec![0.5f32; 1024])).unwrap();
        assert_eq!(result.integrated_lufs.quality, MeasurementQuality::Placeholder);
        assert!(result.integrated_lufs.value.is_none());
    }

    #[test]
    fn empty_buffer_returns_error() {
        let result = measure(&buf(vec![]));
        assert!(matches!(result, Err(GainError::InvalidAudio { .. })));
    }

    #[test]
    fn nan_sample_returns_error() {
        let result = measure(&buf(vec![f32::NAN]));
        assert!(matches!(result, Err(GainError::InvalidAudio { .. })));
    }

    #[test]
    fn out_of_range_sample_returns_error() {
        let result = measure(&buf(vec![1.5f32]));
        assert!(matches!(result, Err(GainError::InvalidAudio { .. })));
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

```
cd gain-core && cargo test -p analysis
```

Expected: all 9 tests pass.

- [ ] **Step 6: Commit**

```bash
git add gain-core/crates/analysis/
git commit -m "feat: implement Peak/RMS/CrestFactor measurement in analysis crate"
```

---

## Task 5: `gain_decision` — Preset Math

**Files:**
- Modify: `gain-core/crates/gain_decision/Cargo.toml`
- Replace: `gain-core/crates/gain_decision/src/lib.rs`

**Interfaces:**
- Consumes: `gain_map::{GainRecommendationMap, GainRegion, Measurements, PresetId, RegionType, GAIN_MAP_SCHEMA_VERSION}`, `gain_error::GainError`
- Produces:
  - `pub enum MeasureType { Peak, Rms }`
  - `pub fn recommend(measurements: &Measurements, measure: MeasureType, target_db: f32, duration_secs: f64, preset_id: PresetId) -> Result<GainRecommendationMap, GainError>`

---

- [ ] **Step 1: Update Cargo.toml**

Replace `gain-core/crates/gain_decision/Cargo.toml`:

```toml
[package]
name = "gain_decision"
version = "0.1.0"
edition = "2021"

[dependencies]
gain_map   = { path = "../gain_map" }
gain-error = { path = "../gain-error" }
```

- [ ] **Step 2: Write the failing tests**

Replace `gain-core/crates/gain_decision/src/lib.rs` with stub + tests:

```rust
use gain_error::GainError;
use gain_map::{GainRecommendationMap, Measurements, MeasurementQuality, MeasurementValue, PresetId};

#[derive(Debug, Clone, PartialEq)]
pub enum MeasureType { Peak, Rms }

pub fn recommend(
    _measurements: &Measurements,
    _measure: MeasureType,
    _target_db: f32,
    _duration_secs: f64,
    _preset_id: PresetId,
) -> Result<GainRecommendationMap, GainError> {
    Err(GainError::AnalysisFailure { details: "not implemented".to_string() })
}

fn placeholder_measurements(peak: f32, rms: f32) -> Measurements {
    Measurements {
        peak_dbfs: peak, rms_dbfs: rms, crest_factor_db: peak - rms,
        integrated_lufs: MeasurementValue { value: None, quality: MeasurementQuality::Placeholder },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gain_map::RegionType;

    #[test]
    fn peak_target_minus_12_with_peak_minus_6_gives_minus_6_gain() {
        let m = placeholder_measurements(-6.0, -10.0);
        let map = recommend(&m, MeasureType::Peak, -12.0, 2.0, PresetId::MixPrepStandard).unwrap();
        assert!((map.regions[0].gain_db - (-6.0)).abs() < 0.001);
    }

    #[test]
    fn rms_target_minus_18_with_rms_minus_20_gives_plus_2_gain() {
        let m = placeholder_measurements(-14.0, -20.0);
        let map = recommend(&m, MeasureType::Rms, -18.0, 3.0, PresetId::AnalogConsole).unwrap();
        assert!((map.regions[0].gain_db - 2.0).abs() < 0.001);
    }

    #[test]
    fn region_spans_full_duration() {
        let m = placeholder_measurements(-12.0, -18.0);
        let map = recommend(&m, MeasureType::Peak, -12.0, 5.5, PresetId::MixPrepStandard).unwrap();
        assert_eq!(map.regions.len(), 1);
        assert_eq!(map.regions[0].start_time, 0.0);
        assert!((map.regions[0].end_time - 5.5).abs() < 0.001);
    }

    #[test]
    fn region_type_is_stable() {
        let m = placeholder_measurements(-12.0, -18.0);
        let map = recommend(&m, MeasureType::Peak, -12.0, 1.0, PresetId::MixPrepStandard).unwrap();
        assert_eq!(map.regions[0].region_type, RegionType::Stable);
    }

    #[test]
    fn confidence_is_one() {
        let m = placeholder_measurements(-12.0, -18.0);
        let map = recommend(&m, MeasureType::Peak, -12.0, 1.0, PresetId::MixPrepStandard).unwrap();
        assert_eq!(map.regions[0].confidence, 1.0);
    }

    #[test]
    fn preset_used_is_set() {
        let m = placeholder_measurements(-12.0, -18.0);
        let map = recommend(&m, MeasureType::Rms, -14.0, 1.0, PresetId::AnalogConsoleHot).unwrap();
        assert_eq!(map.preset_used, Some(PresetId::AnalogConsoleHot));
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

```
cd gain-core && cargo test -p gain_decision
```

Expected: all tests fail (stub returns Err).

- [ ] **Step 4: Implement**

Replace the full contents of `gain-core/crates/gain_decision/src/lib.rs`:

```rust
use gain_error::GainError;
use gain_map::{
    GainRecommendationMap, GainRegion, Measurements, MeasurementQuality, MeasurementValue,
    PresetId, RegionType, GAIN_MAP_SCHEMA_VERSION,
};

#[derive(Debug, Clone, PartialEq)]
pub enum MeasureType {
    Peak,
    Rms,
}

pub fn recommend(
    measurements: &Measurements,
    measure: MeasureType,
    target_db: f32,
    duration_secs: f64,
    preset_id: PresetId,
) -> Result<GainRecommendationMap, GainError> {
    let (measured_db, measure_label) = match &measure {
        MeasureType::Peak => (measurements.peak_dbfs, "Peak"),
        MeasureType::Rms  => (measurements.rms_dbfs,  "RMS"),
    };

    let gain_db = target_db - measured_db;
    let reason  = format!("Applied target of {target_db:.1} dBFS using {measure_label} measurement");

    Ok(GainRecommendationMap {
        version:     GAIN_MAP_SCHEMA_VERSION,
        preset_used: Some(preset_id),
        regions: vec![GainRegion {
            start_time:  0.0,
            end_time:    duration_secs,
            gain_db,
            confidence:  1.0,
            region_type: RegionType::Stable,
            reason,
        }],
    })
}

fn placeholder_measurements(peak: f32, rms: f32) -> Measurements {
    Measurements {
        peak_dbfs: peak, rms_dbfs: rms, crest_factor_db: peak - rms,
        integrated_lufs: MeasurementValue { value: None, quality: MeasurementQuality::Placeholder },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gain_map::RegionType;

    #[test]
    fn peak_target_minus_12_with_peak_minus_6_gives_minus_6_gain() {
        let m = placeholder_measurements(-6.0, -10.0);
        let map = recommend(&m, MeasureType::Peak, -12.0, 2.0, PresetId::MixPrepStandard).unwrap();
        assert!((map.regions[0].gain_db - (-6.0)).abs() < 0.001);
    }

    #[test]
    fn rms_target_minus_18_with_rms_minus_20_gives_plus_2_gain() {
        let m = placeholder_measurements(-14.0, -20.0);
        let map = recommend(&m, MeasureType::Rms, -18.0, 3.0, PresetId::AnalogConsole).unwrap();
        assert!((map.regions[0].gain_db - 2.0).abs() < 0.001);
    }

    #[test]
    fn region_spans_full_duration() {
        let m = placeholder_measurements(-12.0, -18.0);
        let map = recommend(&m, MeasureType::Peak, -12.0, 5.5, PresetId::MixPrepStandard).unwrap();
        assert_eq!(map.regions.len(), 1);
        assert_eq!(map.regions[0].start_time, 0.0);
        assert!((map.regions[0].end_time - 5.5).abs() < 0.001);
    }

    #[test]
    fn region_type_is_stable() {
        let m = placeholder_measurements(-12.0, -18.0);
        let map = recommend(&m, MeasureType::Peak, -12.0, 1.0, PresetId::MixPrepStandard).unwrap();
        assert_eq!(map.regions[0].region_type, RegionType::Stable);
    }

    #[test]
    fn confidence_is_one() {
        let m = placeholder_measurements(-12.0, -18.0);
        let map = recommend(&m, MeasureType::Peak, -12.0, 1.0, PresetId::MixPrepStandard).unwrap();
        assert_eq!(map.regions[0].confidence, 1.0);
    }

    #[test]
    fn preset_used_is_set() {
        let m = placeholder_measurements(-12.0, -18.0);
        let map = recommend(&m, MeasureType::Rms, -14.0, 1.0, PresetId::AnalogConsoleHot).unwrap();
        assert_eq!(map.preset_used, Some(PresetId::AnalogConsoleHot));
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

```
cd gain-core && cargo test -p gain_decision
```

Expected: all 6 tests pass.

- [ ] **Step 6: Commit**

```bash
git add gain-core/crates/gain_decision/
git commit -m "feat: implement gain_decision recommend() with MeasureType and preset math"
```

---

## Task 6: `gain-api` — Two-Step Public API

**Files:**
- Modify: `gain-core/crates/gain-api/Cargo.toml`
- Replace: `gain-core/crates/gain-api/src/lib.rs`
- Create: `gain-core/crates/gain-api/tests/pipeline.rs`

**Interfaces:**
- Consumes: all crates in the graph
- Produces (public API):
  - `pub use gain_error::GainError`
  - `pub use gain_map::{GainRecommendationMap, GainRegion, Measurements, MeasurementQuality, MeasurementValue, PresetId, RegionType, GAIN_MAP_SCHEMA_VERSION}`
  - `pub use audio_ingestion::{AudioBuffer, AudioMetadata, ContainerFormat}`
  - `pub use gain_decision::MeasureType`
  - `pub struct AnalysisResult { pub metadata: AudioMetadata, pub measurements: Measurements }`
  - `pub enum RecommendationPreset { MixPrepConservative, MixPrepStandard, MixPrepAggressive, AnalogConsole, AnalogConsoleHot, DialoguePrep, Custom { measure: MeasureType, target_db: f32 } }`
  - `pub fn analyze_file(path: &Path) -> Result<AnalysisResult, GainError>`
  - `pub fn analyze_pcm(samples: &[f32], sample_rate: u32, channels: u16, duration_secs: f64) -> Result<AnalysisResult, GainError>`
  - `pub fn generate_recommendation(analysis: &AnalysisResult, preset: RecommendationPreset) -> Result<GainRecommendationMap, GainError>`

---

- [ ] **Step 1: Update Cargo.toml**

Replace `gain-core/crates/gain-api/Cargo.toml`:

```toml
[package]
name = "gain-api"
version = "0.1.0"
edition = "2021"

[dependencies]
gain_map        = { path = "../gain_map" }
gain_decision   = { path = "../gain_decision" }
gain-error      = { path = "../gain-error" }
audio_ingestion = { path = "../audio_ingestion" }
analysis        = { path = "../analysis" }

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Write the integration test (it will fail to compile until the API is in place)**

Create `gain-core/crates/gain-api/tests/pipeline.rs`:

```rust
use gain_api::{
    analyze_file, analyze_pcm, generate_recommendation,
    MeasurementQuality, PresetId, RecommendationPreset,
};
use std::io::Write;

fn make_wav_constant(amplitude: f32, n_samples: usize, sample_rate: u32) -> Vec<u8> {
    let amp_i16 = (amplitude * i16::MAX as f32) as i16;
    let samples_i16: Vec<i16> = vec![amp_i16; n_samples];
    let data_len = n_samples * 2;
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36u32 + data_len as u32).to_le_bytes());
    b.extend_from_slice(b"WAVE");
    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&sample_rate.to_le_bytes());
    b.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(data_len as u32).to_le_bytes());
    for s in samples_i16 { b.extend_from_slice(&s.to_le_bytes()); }
    b
}

#[test]
fn mix_prep_standard_on_minus_20_peak_gives_plus_8_gain() {
    // Constant 0.1 amplitude → peak_dbfs = 20*log10(0.1) = -20 dBFS
    // MixPrepStandard target = -12 dBFS → gain_db = -12 - (-20) = +8
    let wav = make_wav_constant(0.1, 1000, 44100);
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(&wav).unwrap();

    let analysis = analyze_file(f.path()).unwrap();
    assert!((analysis.measurements.peak_dbfs - (-20.0)).abs() < 0.1,
        "expected peak ≈ -20, got {}", analysis.measurements.peak_dbfs);
    assert_eq!(analysis.measurements.integrated_lufs.quality, MeasurementQuality::Placeholder);
    assert!(analysis.measurements.integrated_lufs.value.is_none());

    let map = generate_recommendation(&analysis, RecommendationPreset::MixPrepStandard).unwrap();
    assert_eq!(map.regions.len(), 1);
    assert!((map.regions[0].gain_db - 8.0).abs() < 0.1,
        "expected gain ≈ +8, got {}", map.regions[0].gain_db);
    assert_eq!(map.preset_used, Some(PresetId::MixPrepStandard));
}

#[test]
fn analyze_pcm_produces_same_measurements_as_analyze_file() {
    let wav = make_wav_constant(0.5, 4410, 44100);
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(&wav).unwrap();

    let from_file = analyze_file(f.path()).unwrap();
    let samples: Vec<f32> = vec![0.5; 4410];
    let from_pcm = analyze_pcm(&samples, 44100, 1, 0.1).unwrap();

    assert!((from_file.measurements.peak_dbfs - from_pcm.measurements.peak_dbfs).abs() < 0.01);
    assert!((from_file.measurements.rms_dbfs  - from_pcm.measurements.rms_dbfs ).abs() < 0.01);
}

#[test]
fn file_not_found_returns_error() {
    let result = analyze_file(std::path::Path::new("/no/such/file.wav"));
    assert!(matches!(result, Err(gain_api::GainError::FileNotFound { .. })));
}
```

- [ ] **Step 3: Run test to verify it fails to compile**

```
cd gain-core && cargo test -p gain-api
```

Expected: compile error — new types not yet defined in gain-api.

- [ ] **Step 4: Implement**

Replace the full contents of `gain-core/crates/gain-api/src/lib.rs`. **The two old stub tests (`analyze_file_stub_returns_default_map`, `gain_error_variants_exist`) are removed entirely.**

```rust
pub use gain_error::GainError;
pub use gain_map::{
    GainRecommendationMap, GainRegion, Measurements, MeasurementQuality, MeasurementValue,
    PresetId, RegionType, GAIN_MAP_SCHEMA_VERSION,
};
pub use audio_ingestion::{AudioBuffer, AudioMetadata, ContainerFormat};
pub use gain_decision::MeasureType;

pub struct AnalysisResult {
    pub metadata:     AudioMetadata,
    pub measurements: Measurements,
}

pub enum RecommendationPreset {
    MixPrepConservative,              // Peak −18 dBFS
    MixPrepStandard,                  // Peak −12 dBFS
    MixPrepAggressive,                // Peak −6 dBFS
    AnalogConsole,                    // RMS −18 dBFS
    AnalogConsoleHot,                 // RMS −14 dBFS
    DialoguePrep,                     // Peak −10 dBFS
    Custom { measure: MeasureType, target_db: f32 },
}

/// Step 1 of the public API: decode an audio file and measure Peak/RMS/CrestFactor.
pub fn analyze_file(path: &std::path::Path) -> Result<AnalysisResult, GainError> {
    let (buf, metadata) = audio_ingestion::load_file(path)?;
    let measurements = analysis::measure(&buf)?;
    Ok(AnalysisResult { metadata, measurements })
}

/// Step 1 variant: measure raw PCM samples already in memory.
/// `duration_secs` must reflect the true playback duration of the supplied samples.
pub fn analyze_pcm(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
    duration_secs: f64,
) -> Result<AnalysisResult, GainError> {
    let buf = AudioBuffer { samples: samples.to_vec(), sample_rate, channels };
    let measurements = analysis::measure(&buf)?;
    let metadata = AudioMetadata { duration_secs, sample_rate, channels, format: ContainerFormat::Wav };
    Ok(AnalysisResult { metadata, measurements })
}

/// Step 2 of the public API: apply a preset to produce a GainRecommendationMap.
pub fn generate_recommendation(
    analysis: &AnalysisResult,
    preset: RecommendationPreset,
) -> Result<GainRecommendationMap, GainError> {
    let (measure, target_db, preset_id) = match preset {
        RecommendationPreset::MixPrepConservative => (MeasureType::Peak, -18.0f32, PresetId::MixPrepConservative),
        RecommendationPreset::MixPrepStandard     => (MeasureType::Peak, -12.0,    PresetId::MixPrepStandard),
        RecommendationPreset::MixPrepAggressive   => (MeasureType::Peak,  -6.0,    PresetId::MixPrepAggressive),
        RecommendationPreset::AnalogConsole        => (MeasureType::Rms,  -18.0,   PresetId::AnalogConsole),
        RecommendationPreset::AnalogConsoleHot     => (MeasureType::Rms,  -14.0,   PresetId::AnalogConsoleHot),
        RecommendationPreset::DialoguePrep         => (MeasureType::Peak, -10.0,   PresetId::DialoguePrep),
        RecommendationPreset::Custom { measure, target_db } => (measure, target_db, PresetId::Custom),
    };

    gain_decision::recommend(
        &analysis.measurements,
        measure,
        target_db,
        analysis.metadata.duration_secs,
        preset_id,
    )
}
```

- [ ] **Step 5: Run all tests to verify they pass**

```
cd gain-core && cargo test -p gain-api
```

Expected: the 3 integration tests in `tests/pipeline.rs` all pass. No tests remain in `src/lib.rs` (the stubs were removed).

- [ ] **Step 6: Run the full workspace to check nothing is broken**

```
cd gain-core && cargo test
```

Expected: all tests in all crates pass.

- [ ] **Step 7: Commit**

```bash
git add gain-core/crates/gain-api/
git commit -m "feat: implement two-step gain-api (analyze_file + generate_recommendation) wiring full pipeline"
```

---

## Task 7: FFI Updates

**Files:**
- Modify: `gain-core/crates/ffi/src/lib.rs`
- Modify: `gain-core/crates/ffi/include/gain_stage_ffi.h`

**Interfaces:**
- Consumes: `gain_api::{GainError, RecommendationPreset, analyze_pcm, analyze_file, generate_recommendation}`
- Produces (C ABI additions):
  - `gain_stage_analyze` — wired to real pipeline (MixPrepStandard default)
  - `gain_stage_analyze_file(path: *const c_char, preset: u8) -> *mut GainStageMap`
  - `gain_stage_last_error_code() -> u8`
  - `gain_stage_last_error_message() -> *const c_char`

---

- [ ] **Step 1: Write the new tests (into the existing test module)**

The tests that change are noted below. Add new tests and update the two broken ones.

In the `tests` module of `gain-core/crates/ffi/src/lib.rs`, the test `empty_audio_produces_zero_regions` must change to reflect that Phase 2 always produces 1 region (silent audio is valid, not empty). The test `get_region_on_empty_map_returns_zeroed` must change to use an out-of-bounds index instead. Add these as replacements/additions — the full updated test section is included in Step 4.

- [ ] **Step 2: Run existing tests to see which ones break**

```
cd gain-core && cargo test -p ffi
```

Expected: `empty_audio_produces_zero_regions` should break once the implementation is wired (it expects 0 regions but will get 1). The other existing tests should still pass. The compile will succeed since we haven't changed ffi yet.

- [ ] **Step 3: Update the C header**

Replace the full contents of `gain-core/crates/ffi/include/gain_stage_ffi.h`:

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
#define GAIN_STAGE_REGION_STABLE              0
#define GAIN_STAGE_REGION_TRANSIENT           1
#define GAIN_STAGE_REGION_ENVELOPE_CONTROLLED 2
#define GAIN_STAGE_REGION_MIXED               3

/* Preset codes for gain_stage_analyze_file */
#define GAIN_STAGE_PRESET_MIX_PREP_CONSERVATIVE  0
#define GAIN_STAGE_PRESET_MIX_PREP_STANDARD      1
#define GAIN_STAGE_PRESET_MIX_PREP_AGGRESSIVE    2
#define GAIN_STAGE_PRESET_ANALOG_CONSOLE         3
#define GAIN_STAGE_PRESET_ANALOG_CONSOLE_HOT     4
#define GAIN_STAGE_PRESET_DIALOGUE_PREP          5

/* Error codes returned by gain_stage_last_error_code */
#define GAIN_STAGE_ERR_NONE             0
#define GAIN_STAGE_ERR_FILE_NOT_FOUND   1
#define GAIN_STAGE_ERR_UNSUPPORTED_FMT  2
#define GAIN_STAGE_ERR_DECODE_FAILURE   3
#define GAIN_STAGE_ERR_INVALID_AUDIO    4
#define GAIN_STAGE_ERR_ANALYSIS_FAILURE 5
#define GAIN_STAGE_ERR_INTERNAL         6

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
 * Analyze raw interleaved f32 PCM (assumed mono) and return a GainStageMap.
 * Uses MixPrepStandard preset (-12 dBFS Peak).
 * Returns NULL on error; call gain_stage_last_error_code() for details.
 * Caller must release with gain_stage_free_map().
 */
GainStageMap* gain_stage_analyze(
    const float*  samples,
    size_t        count,
    uint32_t      sample_rate
);

/*
 * Analyze an audio file and return a GainStageMap.
 * path   - null-terminated UTF-8 file path (WAV or AIFF)
 * preset - GAIN_STAGE_PRESET_* constant (unknown values return NULL)
 * Returns NULL on error; call gain_stage_last_error_code() for details.
 * Caller must release with gain_stage_free_map().
 *
 * WARNING: Not re-entrant. Treat as single-threaded at the call site.
 * Thread-local error state may produce incorrect results if called
 * concurrently from the same host thread pool.
 */
GainStageMap* gain_stage_analyze_file(const char* path, uint8_t preset);

/* Release a GainStageMap. Passing NULL is a no-op. */
void gain_stage_free_map(GainStageMap* map);

/* Return the number of regions. Returns 0 if map is NULL. */
size_t gain_stage_map_region_count(const GainStageMap* map);

/*
 * Return a copy of the region at index.
 * Returns a zeroed CGainRegion if map is NULL or index is out of range.
 */
CGainRegion gain_stage_map_get_region(const GainStageMap* map, size_t index);

/* Return the schema version of the map. Returns 0 if map is NULL. */
uint32_t gain_stage_map_version(const GainStageMap* map);

/* Return the error code from the most recent failed call on this thread (0 = no error). */
uint8_t gain_stage_last_error_code(void);

/*
 * Return a null-terminated error message for the last failed call on this thread.
 * The pointer is valid only until the next gain_stage_* call on this thread.
 */
const char* gain_stage_last_error_message(void);

#ifdef __cplusplus
} /* extern "C" */
#endif
```

- [ ] **Step 4: Implement the full updated ffi/src/lib.rs**

Replace the full contents of `gain-core/crates/ffi/src/lib.rs`:

```rust
use std::cell::RefCell;
use std::ffi::CStr;
use std::os::raw::c_char;

use gain_api::{GainError, GainRecommendationMap, RegionType, RecommendationPreset};

fn ffi_guard<T>(f: impl FnOnce() -> T) -> Option<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).ok()
}

thread_local! {
    static LAST_ERROR_CODE: RefCell<u8>    = RefCell::new(0);
    static LAST_ERROR_MSG:  RefCell<String> = RefCell::new(String::new());
    // Null-terminated C string buffer; valid until next set_last_error call on this thread.
    static LAST_ERROR_CSTR: RefCell<Vec<u8>> = RefCell::new(vec![0u8]);
}

fn gain_error_to_code(e: &GainError) -> u8 {
    match e {
        GainError::FileNotFound    { .. } => 1,
        GainError::UnsupportedFormat { .. } => 2,
        GainError::DecodeFailure   { .. } => 3,
        GainError::InvalidAudio    { .. } => 4,
        GainError::AnalysisFailure { .. } => 5,
        GainError::InternalError   { .. } => 6,
    }
}

fn set_last_error(e: &GainError) {
    let code = gain_error_to_code(e);
    let msg  = e.to_string();
    LAST_ERROR_CODE.with(|c| *c.borrow_mut() = code);
    LAST_ERROR_MSG.with( |m| *m.borrow_mut() = msg);
}

fn preset_from_u8(code: u8) -> Result<RecommendationPreset, GainError> {
    match code {
        0 => Ok(RecommendationPreset::MixPrepConservative),
        1 => Ok(RecommendationPreset::MixPrepStandard),
        2 => Ok(RecommendationPreset::MixPrepAggressive),
        3 => Ok(RecommendationPreset::AnalogConsole),
        4 => Ok(RecommendationPreset::AnalogConsoleHot),
        5 => Ok(RecommendationPreset::DialoguePrep),
        n => Err(GainError::InternalError { details: format!("unknown preset code {n}") }),
    }
}

pub struct GainStageMap(GainRecommendationMap);

#[repr(C)]
#[derive(Copy, Clone)]
pub struct CGainRegion {
    pub start_time:  f64,
    pub end_time:    f64,
    pub gain_db:     f32,
    pub confidence:  f32,
    pub region_type: u8,
    pub reason:      [u8; 64],
}

/// Analyze raw interleaved mono f32 PCM. Uses MixPrepStandard preset.
/// Returns NULL on error. Caller must free with gain_stage_free_map.
#[no_mangle]
pub extern "C" fn gain_stage_analyze(
    samples: *const f32,
    count: usize,
    sample_rate: u32,
) -> *mut GainStageMap {
    ffi_guard(|| {
        if samples.is_null() || count == 0 {
            set_last_error(&GainError::InvalidAudio { details: "null or empty samples".to_string() });
            return std::ptr::null_mut();
        }
        // SAFETY: samples is non-null; count is caller-guaranteed to be the valid slice length.
        let slice = unsafe { std::slice::from_raw_parts(samples, count) };
        let duration_secs = count as f64 / sample_rate as f64;

        let analysis = match gain_api::analyze_pcm(slice, sample_rate, 1, duration_secs) {
            Ok(a) => a,
            Err(e) => { set_last_error(&e); return std::ptr::null_mut(); }
        };
        let map = match gain_api::generate_recommendation(&analysis, RecommendationPreset::MixPrepStandard) {
            Ok(m) => m,
            Err(e) => { set_last_error(&e); return std::ptr::null_mut(); }
        };
        Box::into_raw(Box::new(GainStageMap(map)))
    })
    .unwrap_or(std::ptr::null_mut())
}

/// Analyze an audio file. preset must be a GAIN_STAGE_PRESET_* constant.
/// Returns NULL on error. Caller must free with gain_stage_free_map.
#[no_mangle]
pub extern "C" fn gain_stage_analyze_file(
    path: *const c_char,
    preset: u8,
) -> *mut GainStageMap {
    ffi_guard(|| {
        if path.is_null() {
            set_last_error(&GainError::InvalidAudio { details: "null path pointer".to_string() });
            return std::ptr::null_mut();
        }
        // SAFETY: path is non-null; caller guarantees a valid null-terminated C string.
        let c_str = unsafe { CStr::from_ptr(path) };
        let path_str = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => {
                set_last_error(&GainError::InvalidAudio { details: "path is not valid UTF-8".to_string() });
                return std::ptr::null_mut();
            }
        };

        let preset_val = match preset_from_u8(preset) {
            Ok(p) => p,
            Err(e) => { set_last_error(&e); return std::ptr::null_mut(); }
        };

        let analysis = match gain_api::analyze_file(std::path::Path::new(path_str)) {
            Ok(a) => a,
            Err(e) => { set_last_error(&e); return std::ptr::null_mut(); }
        };
        let map = match gain_api::generate_recommendation(&analysis, preset_val) {
            Ok(m) => m,
            Err(e) => { set_last_error(&e); return std::ptr::null_mut(); }
        };
        Box::into_raw(Box::new(GainStageMap(map)))
    })
    .unwrap_or(std::ptr::null_mut())
}

/// Release a GainStageMap. Passing null is a no-op.
#[no_mangle]
pub extern "C" fn gain_stage_free_map(map: *mut GainStageMap) {
    if map.is_null() { return; }
    ffi_guard(|| {
        // SAFETY: map is non-null and was created by Box::into_raw; caller guarantees single free.
        unsafe { drop(Box::from_raw(map)) };
    });
}

/// Return the number of regions. Returns 0 if map is NULL.
#[no_mangle]
pub extern "C" fn gain_stage_map_region_count(map: *const GainStageMap) -> usize {
    if map.is_null() { return 0; }
    ffi_guard(|| {
        // SAFETY: map is non-null; caller holds unique ownership for this call's duration.
        unsafe { (*map).0.regions.len() }
    })
    .unwrap_or(0)
}

/// Return a copy of the region at index. Returns zeroed CGainRegion on null/out-of-range.
#[no_mangle]
pub extern "C" fn gain_stage_map_get_region(
    map: *const GainStageMap,
    index: usize,
) -> CGainRegion {
    let zeroed = CGainRegion {
        start_time: 0.0, end_time: 0.0, gain_db: 0.0, confidence: 0.0,
        region_type: 0, reason: [0u8; 64],
    };
    if map.is_null() { return zeroed; }

    ffi_guard(|| {
        // SAFETY: map is non-null; caller holds ownership for this call's duration.
        let regions = unsafe { &(*map).0.regions };
        let Some(region) = regions.get(index) else { return zeroed; };

        let region_type_byte = match region.region_type {
            RegionType::Stable             => 0u8,
            RegionType::Transient          => 1u8,
            RegionType::EnvelopeControlled => 2u8,
            RegionType::Mixed              => 3u8,
        };

        let mut reason_buf = [0u8; 64];
        let bytes = region.reason.as_bytes();
        let len = bytes.len().min(63);
        reason_buf[..len].copy_from_slice(&bytes[..len]);

        CGainRegion {
            start_time: region.start_time, end_time: region.end_time,
            gain_db: region.gain_db, confidence: region.confidence,
            region_type: region_type_byte, reason: reason_buf,
        }
    })
    .unwrap_or(zeroed)
}

/// Return the schema version. Returns 0 if map is NULL.
#[no_mangle]
pub extern "C" fn gain_stage_map_version(map: *const GainStageMap) -> u32 {
    if map.is_null() { return 0; }
    ffi_guard(|| {
        // SAFETY: map is non-null; caller holds ownership for this call's duration.
        unsafe { (*map).0.version }
    })
    .unwrap_or(0)
}

/// Return the error code from the last failed FFI call on this thread. 0 means no error.
#[no_mangle]
pub extern "C" fn gain_stage_last_error_code() -> u8 {
    LAST_ERROR_CODE.with(|c| *c.borrow())
}

/// Return a null-terminated error message for the last failure on this thread.
/// Valid until the next gain_stage_* call on this thread.
#[no_mangle]
pub extern "C" fn gain_stage_last_error_message() -> *const c_char {
    let bytes = LAST_ERROR_MSG.with(|m| {
        let mut b = m.borrow().as_bytes().to_vec();
        b.push(0);
        b
    });
    LAST_ERROR_CSTR.with(|buf| {
        *buf.borrow_mut() = bytes;
        buf.borrow().as_ptr() as *const c_char
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_returns_non_null_map() {
        let samples = vec![0.5f32; 1024];
        let map = gain_stage_analyze(samples.as_ptr(), samples.len(), 44100);
        assert!(!map.is_null());
        gain_stage_free_map(map);
    }

    #[test]
    fn silent_audio_produces_one_region() {
        // Phase 2: silent audio is valid; silence floor (-120 dBFS) + MixPrepStandard → 1 region
        let samples = vec![0.0f32; 1024];
        let map = gain_stage_analyze(samples.as_ptr(), samples.len(), 44100);
        assert!(!map.is_null());
        let count = gain_stage_map_region_count(map);
        assert_eq!(count, 1);
        gain_stage_free_map(map);
    }

    #[test]
    fn get_region_out_of_bounds_returns_zeroed() {
        let samples = vec![0.5f32; 1024];
        let map = gain_stage_analyze(samples.as_ptr(), samples.len(), 44100);
        // Index 5 is always out of range in Phase 2 (only 1 region)
        let region = gain_stage_map_get_region(map, 5);
        assert_eq!(region.gain_db, 0.0);
        gain_stage_free_map(map);
    }

    #[test]
    fn map_version_is_one() {
        let samples = vec![0.5f32; 1024];
        let map = gain_stage_analyze(samples.as_ptr(), samples.len(), 44100);
        assert_eq!(gain_stage_map_version(map), 1);
        gain_stage_free_map(map);
    }

    #[test]
    fn null_samples_returns_null_and_sets_error() {
        let map = gain_stage_analyze(std::ptr::null(), 0, 44100);
        assert!(map.is_null());
        assert_ne!(gain_stage_last_error_code(), 0);
    }

    #[test]
    fn unknown_preset_returns_null() {
        let path = std::ffi::CString::new("/no/such/file.wav").unwrap();
        let map = gain_stage_analyze_file(path.as_ptr(), 99);
        assert!(map.is_null());
        assert_eq!(gain_stage_last_error_code(), 6); // InternalError
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

```
cd gain-core && cargo test -p ffi
```

Expected: all 6 tests pass.

- [ ] **Step 6: Commit**

```bash
git add gain-core/crates/ffi/
git commit -m "feat: wire FFI to real pipeline; add gain_stage_analyze_file, error introspection"
```

---

## Task 8: `gain-standalone` Command Update

**Files:**
- Modify: `gain-standalone/src-tauri/src/commands/analyze.rs`
- Modify: `gain-standalone/src-tauri/src/dto.rs`

**Interfaces:**
- Consumes: `gain_api::{analyze_file, generate_recommendation, RecommendationPreset, PresetId}`
- Produces: Tauri command `analyze(path: String, preset: Option<u8>)` — `preset` defaults to `1` (MixPrepStandard) when omitted so the existing frontend continues to work

---

- [ ] **Step 1: Update the DTO**

Replace `gain-standalone/src-tauri/src/dto.rs`:

```rust
#[derive(serde::Serialize)]
pub struct GainMapDto {
    pub version:    u32,
    pub preset_used: Option<String>,
    pub regions:    Vec<GainRegionDto>,
}

#[derive(serde::Serialize)]
pub struct GainRegionDto {
    pub start_time:  f64,
    pub end_time:    f64,
    pub gain_db:     f32,
    pub confidence:  f32,
    pub region_type: String,
    pub reason:      String,
}
```

- [ ] **Step 2: Update the analyze command**

Replace `gain-standalone/src-tauri/src/commands/analyze.rs`:

```rust
use crate::dto::{GainMapDto, GainRegionDto};
use gain_api::RecommendationPreset;

fn preset_from_u8(code: u8) -> Result<RecommendationPreset, String> {
    match code {
        0 => Ok(RecommendationPreset::MixPrepConservative),
        1 => Ok(RecommendationPreset::MixPrepStandard),
        2 => Ok(RecommendationPreset::MixPrepAggressive),
        3 => Ok(RecommendationPreset::AnalogConsole),
        4 => Ok(RecommendationPreset::AnalogConsoleHot),
        5 => Ok(RecommendationPreset::DialoguePrep),
        n => Err(format!("unknown preset code {n}")),
    }
}

#[tauri::command]
pub fn analyze(path: String, preset: Option<u8>) -> Result<GainMapDto, String> {
    let preset_val = preset_from_u8(preset.unwrap_or(1))?;
    let analysis = gain_api::analyze_file(std::path::Path::new(&path))
        .map_err(|e| format!("{e}"))?;
    let map = gain_api::generate_recommendation(&analysis, preset_val)
        .map_err(|e| format!("{e}"))?;
    Ok(GainMapDto {
        version:     map.version,
        preset_used: map.preset_used.map(|p| format!("{p:?}")),
        regions: map.regions.iter().map(|r| GainRegionDto {
            start_time:  r.start_time,
            end_time:    r.end_time,
            gain_db:     r.gain_db,
            confidence:  r.confidence,
            region_type: format!("{:?}", r.region_type),
            reason:      r.reason.clone(),
        }).collect(),
    })
}
```

- [ ] **Step 3: Verify the Tauri app compiles**

```
cd gain-standalone && cargo build
```

Expected: compiles without errors. The `invoke_handler` in `main.rs` does not need changes — the command is still named `analyze`.

- [ ] **Step 4: Run the Tauri app and manually test the golden path**

```
cd gain-standalone && cargo tauri dev
```

Open a WAV or AIFF file via the import dialog. Expected: the gain recommendation shows a single region with a non-zero `gain_db` value and `preset_used` populated.

- [ ] **Step 5: Commit**

```bash
git add gain-standalone/src-tauri/src/commands/analyze.rs \
        gain-standalone/src-tauri/src/dto.rs
git commit -m "feat: update analyze command to two-step API with optional preset parameter"
```

---

## Self-Review Checklist

- [ ] **Spec coverage**
  - [x] WAV + AIFF decoding → Task 3
  - [x] Real Peak, RMS, CrestFactor → Task 4
  - [x] LUFS always Placeholder/None → Task 4 (`lufs_is_always_placeholder_none` test)
  - [x] SILENCE_FLOOR_DBFS = -120.0, not f32::MIN_POSITIVE → Task 4
  - [x] gain_decision never imports analysis → Task 5 Cargo.toml
  - [x] MeasureType defined in gain_decision, re-exported by gain-api → Tasks 5 & 6
  - [x] Two-step API → Task 6
  - [x] PresetId on GainRecommendationMap → Task 2 (gain_map)
  - [x] reason is descriptive only, never parsed → enforce by doc + no test parses it
  - [x] Single GainRegion per file, RegionType::Stable, confidence 1.0 → Task 5 tests
  - [x] FFI preset mapping table with InternalError on unknown → Task 7
  - [x] gain_stage_last_error_code + message → Task 7
  - [x] Old stub tests removed (gain-api stubs) → Task 6
  - [x] Old ffi test updated (0 regions → 1) → Task 7

- [ ] **Type consistency across tasks**
  - `Measurements` defined in `gain_map` (Task 2), used by `analysis::measure` (Task 4), `gain_decision::recommend` (Task 5), `analyze_pcm` / `analyze_file` (Task 6)
  - `PresetId` defined in `gain_map` (Task 2), used by `gain_decision::recommend` (Task 5), `generate_recommendation` (Task 6), FFI (Task 7)
  - `MeasureType` defined in `gain_decision` (Task 5), re-exported by `gain-api` (Task 6), used in `RecommendationPreset::Custom` (Task 6)
  - `AudioBuffer` / `AudioMetadata` / `ContainerFormat` defined in `audio_ingestion` (Task 3), re-exported by `gain-api` (Task 6)
  - `GainError` defined in `gain-error` (Task 1), re-exported by `gain-api` (Task 6)

- [ ] **No placeholder scan** — confirm no "TBD", "not implemented" stubs survive to the final commit

---

## Execution Options

Plan complete and saved to `docs/superpowers/plans/2026-06-24-phase2-dsp-pipeline.md`.

**1. Subagent-Driven (recommended)** — fresh subagent per task, two-stage review between tasks, fast iteration. Invoke `superpowers:subagent-driven-development`.

**2. Inline Execution** — execute tasks in this session using `superpowers:executing-plans`, batch execution with checkpoints for review.

Which approach?
