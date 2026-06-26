use std::cell::RefCell;
use std::ffi::CStr;
use std::os::raw::c_char;

use gain_api::{
    AnalysisBundle, ContentClass, GainError, GainRecommendationMap,
    RegionType, RecommendationPreset,
};

// ── Region type byte constants ──────────────────────────────────────────────
pub const GAIN_STAGE_REGION_TYPE_STABLE:     u8 = 0;
pub const GAIN_STAGE_REGION_TYPE_SILENCE:    u8 = 1;
pub const GAIN_STAGE_REGION_TYPE_DIALOGUE:   u8 = 2;
pub const GAIN_STAGE_REGION_TYPE_MUSIC:      u8 = 3;
pub const GAIN_STAGE_REGION_TYPE_AMBIENCE:   u8 = 4;
pub const GAIN_STAGE_REGION_TYPE_PERCUSSIVE: u8 = 5;
pub const GAIN_STAGE_REGION_TYPE_MIXED:      u8 = 6;
pub const GAIN_STAGE_REGION_TYPE_UNKNOWN:    u8 = 7;

// ── Content class byte constants ────────────────────────────────────────────
pub const GAIN_STAGE_CLASS_SILENCE:    u8 = 0;
pub const GAIN_STAGE_CLASS_DIALOGUE:   u8 = 1;
pub const GAIN_STAGE_CLASS_MUSIC:      u8 = 2;
pub const GAIN_STAGE_CLASS_AMBIENCE:   u8 = 3;
pub const GAIN_STAGE_CLASS_PERCUSSIVE: u8 = 4;
pub const GAIN_STAGE_CLASS_MIXED:      u8 = 5;
pub const GAIN_STAGE_CLASS_UNKNOWN:    u8 = 6;

fn region_type_to_byte(rt: RegionType) -> u8 {
    match rt {
        RegionType::Stable     => GAIN_STAGE_REGION_TYPE_STABLE,
        RegionType::Silence    => GAIN_STAGE_REGION_TYPE_SILENCE,
        RegionType::Dialogue   => GAIN_STAGE_REGION_TYPE_DIALOGUE,
        RegionType::Music      => GAIN_STAGE_REGION_TYPE_MUSIC,
        RegionType::Ambience   => GAIN_STAGE_REGION_TYPE_AMBIENCE,
        RegionType::Percussive => GAIN_STAGE_REGION_TYPE_PERCUSSIVE,
        RegionType::Mixed      => GAIN_STAGE_REGION_TYPE_MIXED,
        RegionType::Unknown    => GAIN_STAGE_REGION_TYPE_UNKNOWN,
    }
}

fn content_class_to_byte(cc: ContentClass) -> u8 {
    match cc {
        ContentClass::Silence    => GAIN_STAGE_CLASS_SILENCE,
        ContentClass::Dialogue   => GAIN_STAGE_CLASS_DIALOGUE,
        ContentClass::Music      => GAIN_STAGE_CLASS_MUSIC,
        ContentClass::Ambience   => GAIN_STAGE_CLASS_AMBIENCE,
        ContentClass::Percussive => GAIN_STAGE_CLASS_PERCUSSIVE,
        ContentClass::Mixed      => GAIN_STAGE_CLASS_MIXED,
        ContentClass::Unknown    => GAIN_STAGE_CLASS_UNKNOWN,
    }
}

// ── Thread-local error state ─────────────────────────────────────────────────
thread_local! {
    static LAST_ERROR_CODE: RefCell<u8>     = RefCell::new(0);
    static LAST_ERROR_MSG:  RefCell<String> = RefCell::new(String::new());
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

fn ffi_guard<T>(f: impl FnOnce() -> T) -> Option<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).ok()
}

fn preset_from_u8(code: u8) -> Result<RecommendationPreset, GainError> {
    match code {
        0 => Ok(RecommendationPreset::MixPrepConservative),
        1 => Ok(RecommendationPreset::MixPrepStandard),
        2 => Ok(RecommendationPreset::MixPrepAggressive),
        3 => Ok(RecommendationPreset::AnalogConsole),
        4 => Ok(RecommendationPreset::AnalogConsoleHot),
        5 => Ok(RecommendationPreset::DialoguePrep),
        6 => Ok(RecommendationPreset::DialogueBroadcast),
        7 => Ok(RecommendationPreset::PodcastPrep),
        8 => Ok(RecommendationPreset::VoiceoverPrep),
        9 => Ok(RecommendationPreset::MusicStemPrep),
        10 => Ok(RecommendationPreset::FilmDialogue),
        11 => Ok(RecommendationPreset::AlbumConsistency),
        n  => Err(GainError::InternalError { details: format!("unknown preset code {n}") }),
    }
}

// ── Opaque wrappers ───────────────────────────────────────────────────────────

pub struct GainStageMap(GainRecommendationMap);

/// Holds a classified AnalysisBundle and the chosen preset, for the two-step API.
pub struct GainStageAnalysis {
    bundle:  AnalysisBundle,
    preset:  RecommendationPreset,
}

// ── Phase 2 C structs ─────────────────────────────────────────────────────────

/// Legacy region struct — reason field preserved for ABI compat; always zeroed in Phase 3.
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

// ── Phase 3 C struct ──────────────────────────────────────────────────────────

/// Phase 3 recommendation struct. No reason field — heap-allocated strings
/// deferred to Phase 4 to avoid thread-local unsafety in DAW hosts.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct GainStageRegionV2 {
    pub start_time:    f64,
    pub end_time:      f64,
    pub gain_db:       f32,
    pub confidence:    f32,
    pub region_type:   u8,
    pub content_class: u8,
    pub is_applicable: u8,  // 0 = false, 1 = true
}

// ── Phase 2 functions ─────────────────────────────────────────────────────────

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
        let slice = unsafe { std::slice::from_raw_parts(samples, count) };
        let duration_secs = count as f64 / sample_rate as f64;
        let analysis = match gain_api::analyze_pcm(slice, sample_rate, 1, duration_secs) {
            Ok(a) => a, Err(e) => { set_last_error(&e); return std::ptr::null_mut(); }
        };
        let map = match gain_api::generate_recommendation(&analysis, RecommendationPreset::MixPrepStandard) {
            Ok(m) => m, Err(e) => { set_last_error(&e); return std::ptr::null_mut(); }
        };
        Box::into_raw(Box::new(GainStageMap(map)))
    }).unwrap_or(std::ptr::null_mut())
}

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
        let c_str = unsafe { CStr::from_ptr(path) };
        let path_str = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => {
                set_last_error(&GainError::InvalidAudio { details: "path is not valid UTF-8".to_string() });
                return std::ptr::null_mut();
            }
        };
        let preset_val = match preset_from_u8(preset) {
            Ok(p) => p, Err(e) => { set_last_error(&e); return std::ptr::null_mut(); }
        };
        let analysis = match gain_api::analyze_file(std::path::Path::new(path_str)) {
            Ok(a) => a, Err(e) => { set_last_error(&e); return std::ptr::null_mut(); }
        };
        let map = match gain_api::generate_recommendation(&analysis, preset_val) {
            Ok(m) => m, Err(e) => { set_last_error(&e); return std::ptr::null_mut(); }
        };
        Box::into_raw(Box::new(GainStageMap(map)))
    }).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn gain_stage_free_map(map: *mut GainStageMap) {
    if map.is_null() { return; }
    ffi_guard(|| { unsafe { drop(Box::from_raw(map)) }; });
}

#[no_mangle]
pub extern "C" fn gain_stage_map_region_count(map: *const GainStageMap) -> usize {
    if map.is_null() { return 0; }
    ffi_guard(|| unsafe { (*map).0.recommendations.len() }).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn gain_stage_map_get_region(map: *const GainStageMap, index: usize) -> CGainRegion {
    let zeroed = CGainRegion {
        start_time: 0.0, end_time: 0.0, gain_db: 0.0,
        confidence: 0.0, region_type: 0, reason: [0u8; 64],
    };
    if map.is_null() { return zeroed; }
    ffi_guard(|| {
        let recs = unsafe { &(*map).0.recommendations };
        let Some(rec) = recs.get(index) else { return zeroed; };
        CGainRegion {
            start_time:  rec.start_time,
            end_time:    rec.end_time,
            gain_db:     rec.decision.gain_db,
            confidence:  rec.decision.confidence,
            region_type: region_type_to_byte(rec.analysis.region_type()),
            reason:      [0u8; 64], // deferred to Phase 4
        }
    }).unwrap_or(zeroed)
}

#[no_mangle]
pub extern "C" fn gain_stage_map_version(map: *const GainStageMap) -> u32 {
    if map.is_null() { return 0; }
    ffi_guard(|| unsafe { (*map).0.version }).unwrap_or(0)
}

// ── Phase 3 functions ─────────────────────────────────────────────────────────

/// Two-step API step 1: decode, segment, and classify a file.
/// Returns NULL on error. Caller must free with gain_stage_free_analysis.
#[no_mangle]
pub extern "C" fn gain_stage_begin_analysis(
    path: *const c_char,
    preset: u8,
) -> *mut GainStageAnalysis {
    ffi_guard(|| {
        if path.is_null() {
            set_last_error(&GainError::InvalidAudio { details: "null path pointer".to_string() });
            return std::ptr::null_mut();
        }
        let c_str = unsafe { CStr::from_ptr(path) };
        let path_str = match c_str.to_str() {
            Ok(s) => s,
            Err(_) => {
                set_last_error(&GainError::InvalidAudio { details: "path not valid UTF-8".to_string() });
                return std::ptr::null_mut();
            }
        };
        let preset_val = match preset_from_u8(preset) {
            Ok(p) => p, Err(e) => { set_last_error(&e); return std::ptr::null_mut(); }
        };
        let bundle = match gain_api::analyze_regions(std::path::Path::new(path_str)) {
            Ok(b) => b, Err(e) => { set_last_error(&e); return std::ptr::null_mut(); }
        };
        Box::into_raw(Box::new(GainStageAnalysis { bundle, preset: preset_val }))
    }).unwrap_or(std::ptr::null_mut())
}

/// Two-step API step 2: apply the preset from begin_analysis to produce a map.
/// Returns NULL on error. Caller must free with gain_stage_free_map.
#[no_mangle]
pub extern "C" fn gain_stage_generate_recommendation(
    analysis: *mut GainStageAnalysis,
) -> *mut GainStageMap {
    ffi_guard(|| {
        if analysis.is_null() {
            set_last_error(&GainError::InvalidAudio { details: "null analysis pointer".to_string() });
            return std::ptr::null_mut();
        }
        // SAFETY: non-null; caller holds unique ownership for this call.
        let a = unsafe { &*analysis };
        // Clone the preset — RecommendationPreset is not Copy.
        let preset = match &a.preset {
            RecommendationPreset::MixPrepConservative => RecommendationPreset::MixPrepConservative,
            RecommendationPreset::MixPrepStandard     => RecommendationPreset::MixPrepStandard,
            RecommendationPreset::MixPrepAggressive   => RecommendationPreset::MixPrepAggressive,
            RecommendationPreset::AnalogConsole        => RecommendationPreset::AnalogConsole,
            RecommendationPreset::AnalogConsoleHot     => RecommendationPreset::AnalogConsoleHot,
            RecommendationPreset::DialoguePrep         => RecommendationPreset::DialoguePrep,
            RecommendationPreset::DialogueBroadcast    => RecommendationPreset::DialogueBroadcast,
            RecommendationPreset::PodcastPrep          => RecommendationPreset::PodcastPrep,
            RecommendationPreset::VoiceoverPrep        => RecommendationPreset::VoiceoverPrep,
            RecommendationPreset::MusicStemPrep        => RecommendationPreset::MusicStemPrep,
            RecommendationPreset::FilmDialogue         => RecommendationPreset::FilmDialogue,
            RecommendationPreset::AlbumConsistency     => RecommendationPreset::AlbumConsistency,
            RecommendationPreset::Custom { measure, target_db } => {
                RecommendationPreset::Custom { measure: measure.clone(), target_db: *target_db }
            }
        };
        let map = match gain_api::generate_region_recommendations(&a.bundle, preset) {
            Ok(m) => m, Err(e) => { set_last_error(&e); return std::ptr::null_mut(); }
        };
        Box::into_raw(Box::new(GainStageMap(map)))
    }).unwrap_or(std::ptr::null_mut())
}

/// Release a GainStageAnalysis. Passing null is a no-op.
#[no_mangle]
pub extern "C" fn gain_stage_free_analysis(analysis: *mut GainStageAnalysis) {
    if analysis.is_null() { return; }
    ffi_guard(|| { unsafe { drop(Box::from_raw(analysis)) }; });
}

/// Return the number of recommendations. Returns 0 if map is NULL.
#[no_mangle]
pub extern "C" fn gain_stage_map_recommendation_count(map: *const GainStageMap) -> usize {
    if map.is_null() { return 0; }
    ffi_guard(|| unsafe { (*map).0.recommendations.len() }).unwrap_or(0)
}

/// Return a copy of the Phase 3 recommendation at index. Returns zeroed struct on null/out-of-range.
#[no_mangle]
pub extern "C" fn gain_stage_map_get_recommendation(
    map: *const GainStageMap,
    index: usize,
) -> GainStageRegionV2 {
    let zeroed = GainStageRegionV2 {
        start_time: 0.0, end_time: 0.0, gain_db: 0.0,
        confidence: 0.0, region_type: 0, content_class: 0, is_applicable: 0,
    };
    if map.is_null() { return zeroed; }
    ffi_guard(|| {
        let recs = unsafe { &(*map).0.recommendations };
        let Some(rec) = recs.get(index) else { return zeroed; };
        GainStageRegionV2 {
            start_time:    rec.start_time,
            end_time:      rec.end_time,
            gain_db:       rec.decision.gain_db,
            confidence:    rec.decision.confidence,
            region_type:   region_type_to_byte(rec.analysis.region_type()),
            content_class: content_class_to_byte(rec.analysis.content_class),
            is_applicable: if rec.decision.is_applicable { 1 } else { 0 },
        }
    }).unwrap_or(zeroed)
}

// ── Error accessors ───────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn gain_stage_last_error_code() -> u8 {
    LAST_ERROR_CODE.with(|c| *c.borrow())
}

/// Returns a null-terminated error message. Valid until the next gain_stage_* call on this thread.
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
    use std::ffi::CString;
    use std::io::Write as _;

    // ── WAV fixture helpers ───────────────────────────────────────────────────

    /// Build a minimal PCM WAV in memory: mono, 16-bit, `n_samples` frames at `amplitude`.
    fn make_wav(amplitude: f32, n_samples: usize, sample_rate: u32) -> Vec<u8> {
        let amp_i16 = (amplitude * i16::MAX as f32) as i16;
        let data_len = n_samples * 2;
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36u32 + data_len as u32).to_le_bytes());
        b.extend_from_slice(b"WAVE");
        b.extend_from_slice(b"fmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());  // PCM
        b.extend_from_slice(&1u16.to_le_bytes());  // mono
        b.extend_from_slice(&sample_rate.to_le_bytes());
        b.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
        b.extend_from_slice(&2u16.to_le_bytes());  // block align
        b.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        b.extend_from_slice(b"data");
        b.extend_from_slice(&(data_len as u32).to_le_bytes());
        for _ in 0..n_samples {
            b.extend_from_slice(&amp_i16.to_le_bytes());
        }
        b
    }

    fn write_wav_fixture(bytes: &[u8]) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::with_suffix(".wav").unwrap();
        f.write_all(bytes).unwrap();
        f
    }

    // --- Phase 2 compat tests (updated assertions) ---

    #[test]
    fn analyze_returns_non_null_map() {
        let samples = vec![0.5f32; 44100];
        let map = gain_stage_analyze(samples.as_ptr(), samples.len(), 44100);
        assert!(!map.is_null());
        gain_stage_free_map(map);
    }

    #[test]
    fn silent_audio_produces_one_region() {
        let samples = vec![0.0f32; 44100];
        let map = gain_stage_analyze(samples.as_ptr(), samples.len(), 44100);
        assert!(!map.is_null());
        let count = gain_stage_map_region_count(map);
        assert_eq!(count, 1);
        gain_stage_free_map(map);
    }

    #[test]
    fn map_version_is_two() {
        let samples = vec![0.5f32; 44100];
        let map = gain_stage_analyze(samples.as_ptr(), samples.len(), 44100);
        assert_eq!(gain_stage_map_version(map), 2);
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
        let path = CString::new("/no/such/file.wav").unwrap();
        let map = gain_stage_analyze_file(path.as_ptr(), 99);
        assert!(map.is_null());
        assert_eq!(gain_stage_last_error_code(), 6);
    }

    #[test]
    fn get_region_out_of_bounds_returns_zeroed() {
        let samples = vec![0.5f32; 44100];
        let map = gain_stage_analyze(samples.as_ptr(), samples.len(), 44100);
        let region = gain_stage_map_get_region(map, 99);
        assert_eq!(region.gain_db, 0.0);
        gain_stage_free_map(map);
    }

    // --- Phase 3 tests ---

    #[test]
    fn begin_analysis_null_path_returns_null() {
        let analysis = gain_stage_begin_analysis(std::ptr::null(), 1);
        assert!(analysis.is_null());
        assert_ne!(gain_stage_last_error_code(), 0);
    }

    #[test]
    fn recommendation_count_is_zero_on_null_map() {
        assert_eq!(gain_stage_map_recommendation_count(std::ptr::null()), 0);
    }

    #[test]
    fn get_recommendation_out_of_bounds_returns_zeroed() {
        let samples = vec![0.5f32; 44100];
        let map = gain_stage_analyze(samples.as_ptr(), samples.len(), 44100);
        let rec = gain_stage_map_get_recommendation(map, 99);
        assert_eq!(rec.gain_db, 0.0);
        assert_eq!(rec.is_applicable, 0);
        gain_stage_free_map(map);
    }

    #[test]
    fn region_and_recommendation_counts_match() {
        let samples = vec![0.5f32; 44100];
        let map = gain_stage_analyze(samples.as_ptr(), samples.len(), 44100);
        assert!(!map.is_null());
        assert_eq!(gain_stage_map_region_count(map), gain_stage_map_recommendation_count(map));
        gain_stage_free_map(map);
    }

    #[test]
    fn free_analysis_null_is_noop() {
        gain_stage_free_analysis(std::ptr::null_mut());
    }

    // ── Phase 3 real-file round-trip ─────────────────────────────────────────

    #[test]
    fn phase3_two_step_api_round_trip_on_real_file() {
        // 1. Create a 1-second sine-like WAV fixture (constant amplitude acts as tone).
        let fixture = write_wav_fixture(&make_wav(0.5, 44100, 44100));
        let path_str = fixture.path().to_str().expect("temp path is valid UTF-8");
        let c_path = CString::new(path_str).unwrap();

        // 2. Step 1: begin_analysis — must return non-null.
        let analysis = gain_stage_begin_analysis(c_path.as_ptr(), 0 /* MixPrepConservative */);
        assert!(!analysis.is_null(), "gain_stage_begin_analysis returned NULL");

        // 3. Step 2: generate_recommendation — must return non-null.
        let map = gain_stage_generate_recommendation(analysis);
        assert!(!map.is_null(), "gain_stage_generate_recommendation returned NULL");

        // 4. Recommendation count must be > 0.
        let count = gain_stage_map_recommendation_count(map);
        assert!(count > 0, "expected at least one recommendation, got 0");

        // 5. First recommendation must have finite gain_db and confidence > 0.
        let rec = gain_stage_map_get_recommendation(map, 0);
        assert!(rec.gain_db.is_finite(), "gain_db is not finite: {}", rec.gain_db);
        assert!(rec.confidence > 0.0, "confidence should be > 0, got {}", rec.confidence);

        // 6. Free map and analysis — must not panic or double-free.
        gain_stage_free_map(map);
        gain_stage_free_analysis(analysis);
    }
}
