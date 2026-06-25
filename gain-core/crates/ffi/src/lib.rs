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
