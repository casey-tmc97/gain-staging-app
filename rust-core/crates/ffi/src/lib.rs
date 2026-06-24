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
    // SAFETY: map was created by Box::into_raw in gain_stage_analyze,
    // was returned by gain_stage_analyze, and this function is only
    // called once per map (caller contract).
    unsafe { drop(Box::from_raw(map)) };
}

/// Return the number of regions in the map.
/// Passing null returns 0.
#[no_mangle]
pub extern "C" fn gain_stage_map_region_count(map: *const GainStageMap) -> usize {
    if map.is_null() {
        return 0;
    }
    // Caller guarantees: map is non-null and was returned by gain_stage_analyze.
    // SAFETY: map is non-null and was created by gain_stage_analyze;
    // we hold a shared reference for the duration of this call.
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

    // Caller guarantees: map is non-null and was returned by gain_stage_analyze.
    // SAFETY: map is non-null and was created by gain_stage_analyze;
    // we hold a shared reference for the duration of this call.
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
        let map = gain_stage_analyze(samples.as_ptr(), samples.len(), 44100);
        assert!(!map.is_null());
        gain_stage_free_map(map);
    }

    #[test]
    fn empty_audio_produces_zero_regions() {
        let samples: Vec<f32> = vec![0.0f32; 1024];
        let map = gain_stage_analyze(samples.as_ptr(), samples.len(), 44100);
        assert!(!map.is_null());
        let count = gain_stage_map_region_count(map);
        assert_eq!(count, 0);
        gain_stage_free_map(map);
    }

    #[test]
    fn get_region_on_empty_map_returns_zeroed() {
        let samples: Vec<f32> = vec![0.0f32; 1024];
        let map = gain_stage_analyze(samples.as_ptr(), samples.len(), 44100);
        let region = gain_stage_map_get_region(map, 0);
        assert_eq!(region.gain_db, 0.0);
        gain_stage_free_map(map);
    }
}
