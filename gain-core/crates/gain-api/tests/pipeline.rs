use gain_api::{
    analyze_file, analyze_pcm, analyze_regions,
    generate_recommendation, generate_region_recommendations,
    GainError, MeasurementQuality, PresetId, RecommendationPreset,
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

// --- Phase 2 compat tests (updated for .recommendations) ---

#[test]
fn mix_prep_standard_on_minus_20_peak_gives_plus_8_gain() {
    let wav = make_wav_constant(0.1, 44100 * 5, 44100);
    let mut f = tempfile::NamedTempFile::with_suffix(".wav").unwrap();
    f.write_all(&wav).unwrap();

    let analysis = analyze_file(f.path()).unwrap();
    assert!((analysis.measurements.peak_dbfs - (-20.0)).abs() < 0.1,
        "expected peak ≈ -20, got {}", analysis.measurements.peak_dbfs);

    let map = generate_recommendation(&analysis, RecommendationPreset::MixPrepStandard).unwrap();
    assert_eq!(map.recommendations.len(), 1);
    assert!((map.recommendations[0].decision.gain_db - 8.0).abs() < 0.1,
        "expected gain ≈ +8, got {}", map.recommendations[0].decision.gain_db);
    assert_eq!(map.preset_used, Some(PresetId::MixPrepStandard));
}

#[test]
fn analyze_pcm_produces_same_peak_as_analyze_file() {
    let wav = make_wav_constant(0.5, 4410, 44100);
    let mut f = tempfile::NamedTempFile::with_suffix(".wav").unwrap();
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
    assert!(matches!(result, Err(GainError::FileNotFound { .. })));
}

#[test]
fn generate_recommendation_schema_version_is_two() {
    let wav = make_wav_constant(0.5, 44100, 44100);
    let mut f = tempfile::NamedTempFile::with_suffix(".wav").unwrap();
    f.write_all(&wav).unwrap();
    let analysis = analyze_file(f.path()).unwrap();
    let map = generate_recommendation(&analysis, RecommendationPreset::MixPrepStandard).unwrap();
    assert_eq!(map.version, 2);
}

// --- Phase 3: analyze_regions + generate_region_recommendations ---

#[test]
fn analyze_regions_returns_analysis_bundle() {
    let wav = make_wav_constant(0.5, 44100 * 3, 44100);
    let mut f = tempfile::NamedTempFile::with_suffix(".wav").unwrap();
    f.write_all(&wav).unwrap();

    let bundle = analyze_regions(f.path()).unwrap();
    assert!(!bundle.regions.is_empty());
    assert_eq!(bundle.sample_rate, 44100);
}

#[test]
fn analyze_regions_file_not_found_returns_error() {
    let result = analyze_regions(std::path::Path::new("/no/such/file.wav"));
    assert!(matches!(result, Err(GainError::FileNotFound { .. })));
}

#[test]
fn generate_region_recommendations_produces_map() {
    let wav = make_wav_constant(0.5, 44100 * 3, 44100);
    let mut f = tempfile::NamedTempFile::with_suffix(".wav").unwrap();
    f.write_all(&wav).unwrap();

    let bundle = analyze_regions(f.path()).unwrap();
    let map = generate_region_recommendations(&bundle, RecommendationPreset::MixPrepStandard).unwrap();
    assert_eq!(map.recommendations.len(), bundle.regions.len());
    assert_eq!(map.version, 2);
}

#[test]
fn region_recommendations_include_is_applicable_flag() {
    let wav = make_wav_constant(0.5, 44100 * 3, 44100);
    let mut f = tempfile::NamedTempFile::with_suffix(".wav").unwrap();
    f.write_all(&wav).unwrap();

    let bundle = analyze_regions(f.path()).unwrap();
    let map = generate_region_recommendations(&bundle, RecommendationPreset::MixPrepStandard).unwrap();
    // Every recommendation has an is_applicable field (true for non-silence)
    for rec in &map.recommendations {
        let _ = rec.decision.is_applicable; // just assert it exists and compiles
    }
}

#[test]
fn long_file_has_verified_lufs_after_analyze_file() {
    // 5 seconds is long enough for BS.1770 gating
    let wav = make_wav_constant(0.5, 44100 * 5, 44100);
    let mut f = tempfile::NamedTempFile::with_suffix(".wav").unwrap();
    f.write_all(&wav).unwrap();
    let analysis = analyze_file(f.path()).unwrap();
    assert_eq!(analysis.measurements.integrated_lufs.quality, MeasurementQuality::Verified);
    assert!(analysis.measurements.integrated_lufs.value.is_some());
}
