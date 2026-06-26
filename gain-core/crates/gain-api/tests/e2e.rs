//! End-to-end integration tests covering the full Phase 3 pipeline.

use gain_api::{
    analyze_album_files, analyze_file, analyze_regions, compute_album_anchor,
    generate_album_recommendations, generate_recommendation,
    generate_region_recommendations, AlbumAnchorMethod, ContentClass, GainError,
    MeasurementQuality, RecommendationPreset,
};
use std::io::Write;

fn make_wav(amplitude: f32, n_samples: usize, sample_rate: u32) -> Vec<u8> {
    let amp_i16 = (amplitude * i16::MAX as f32) as i16;
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
    for _ in 0..n_samples {
        b.extend_from_slice(&amp_i16.to_le_bytes());
    }
    b
}

fn make_silence_tone_silence_wav(sample_rate: u32) -> Vec<u8> {
    // 0.5s silence | 2s tone | 0.5s silence = 3s total
    let half_sec = (sample_rate / 2) as usize;
    let two_sec  = (sample_rate * 2) as usize;
    let amp_i16  = (0.5f32 * i16::MAX as f32) as i16;
    let n_samples = half_sec + two_sec + half_sec;
    let data_len  = n_samples * 2;
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
    for _ in 0..half_sec { b.extend_from_slice(&0i16.to_le_bytes()); }
    for _ in 0..two_sec  { b.extend_from_slice(&amp_i16.to_le_bytes()); }
    for _ in 0..half_sec { b.extend_from_slice(&0i16.to_le_bytes()); }
    b
}

fn write_wav(bytes: &[u8]) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::with_suffix(".wav").unwrap();
    f.write_all(bytes).unwrap();
    f
}

// ── Phase 2 compatibility ────────────────────────────────────────────────────

#[test]
fn phase2_pipeline_still_works_end_to_end() {
    let f = write_wav(&make_wav(0.1, 44100 * 5, 44100));
    let analysis = analyze_file(f.path()).unwrap();
    let map = generate_recommendation(&analysis, RecommendationPreset::MixPrepStandard).unwrap();
    assert_eq!(map.version, 2);
    assert_eq!(map.recommendations.len(), 1);
    assert!(map.recommendations[0].decision.is_applicable);
    // 0.1 amplitude = -20 dBFS peak; MixPrepStandard target = -12; gain = +8
    assert!((map.recommendations[0].decision.gain_db - 8.0).abs() < 0.2,
        "expected +8 dB, got {}", map.recommendations[0].decision.gain_db);
}

// ── Phase 3 pipeline ─────────────────────────────────────────────────────────

#[test]
fn silence_tone_silence_produces_three_regions() {
    let f = write_wav(&make_silence_tone_silence_wav(44100));
    let bundle = analyze_regions(f.path()).unwrap();
    assert_eq!(bundle.regions.len(), 3,
        "expected silence|tone|silence, got {} regions", bundle.regions.len());
}

#[test]
fn silence_regions_classified_as_silence() {
    let f = write_wav(&make_silence_tone_silence_wav(44100));
    let bundle = analyze_regions(f.path()).unwrap();
    assert_eq!(bundle.regions[0].content_class, ContentClass::Silence);
    assert_eq!(bundle.regions[2].content_class, ContentClass::Silence);
}

#[test]
fn silence_regions_have_is_applicable_false() {
    let f = write_wav(&make_silence_tone_silence_wav(44100));
    let bundle = analyze_regions(f.path()).unwrap();
    let map = generate_region_recommendations(&bundle, RecommendationPreset::MixPrepStandard).unwrap();
    assert!(!map.recommendations[0].decision.is_applicable);
    assert!(!map.recommendations[2].decision.is_applicable);
}

#[test]
fn active_region_has_is_applicable_true() {
    let f = write_wav(&make_silence_tone_silence_wav(44100));
    let bundle = analyze_regions(f.path()).unwrap();
    let map = generate_region_recommendations(&bundle, RecommendationPreset::MixPrepStandard).unwrap();
    assert!(map.recommendations[1].decision.is_applicable);
}

#[test]
fn long_file_integrated_lufs_is_verified() {
    let f = write_wav(&make_wav(0.5, 44100 * 5, 44100));
    let analysis = analyze_file(f.path()).unwrap();
    assert_eq!(analysis.measurements.integrated_lufs.quality, MeasurementQuality::Verified);
    assert!(analysis.measurements.integrated_lufs.value.is_some());
}

#[test]
fn long_file_true_peak_is_verified() {
    let f = write_wav(&make_wav(0.5, 44100 * 5, 44100));
    let analysis = analyze_file(f.path()).unwrap();
    assert_eq!(analysis.measurements.true_peak_dbtp.quality, MeasurementQuality::Verified);
    let tp = analysis.measurements.true_peak_dbtp.value.unwrap();
    // 0.5 amplitude → true peak near -6 dBFS (±1 dBTP for oversampling rounding)
    assert!(tp > -10.0 && tp < -4.0, "expected true peak near -6 dBTP, got {tp}");
}

// ── Album consistency ────────────────────────────────────────────────────────

#[test]
fn album_consistency_aligns_three_files_to_median() {
    let loud  = write_wav(&make_wav(0.8, 44100 * 5, 44100));
    let mid   = write_wav(&make_wav(0.5, 44100 * 5, 44100));
    let soft  = write_wav(&make_wav(0.2, 44100 * 5, 44100));
    let paths: Vec<&std::path::Path> = vec![loud.path(), mid.path(), soft.path()];

    let results: Vec<_> = analyze_album_files(&paths)
        .into_iter().filter_map(|r| r.ok()).collect();
    assert_eq!(results.len(), 3);

    let anchor = compute_album_anchor(&results, AlbumAnchorMethod::Median).unwrap();
    let lufs_mid = results[1].measurements.integrated_lufs.value.unwrap();
    assert!((anchor.target_lufs - lufs_mid).abs() < 0.5,
        "anchor should be near mid-file LUFS {lufs_mid}, got {}", anchor.target_lufs);

    let maps = generate_album_recommendations(
        &results, &anchor, RecommendationPreset::AlbumConsistency,
    ).unwrap();
    assert_eq!(maps.len(), 3);

    // Loud file gets negative gain; soft file gets positive gain
    assert!(maps[0].recommendations[0].decision.gain_db < 0.0,
        "loud file should be reduced");
    assert!(maps[2].recommendations[0].decision.gain_db > 0.0,
        "soft file should be boosted");
}

#[test]
fn album_consistency_missing_file_does_not_abort_batch() {
    let good = write_wav(&make_wav(0.5, 44100 * 5, 44100));
    let paths: Vec<&std::path::Path> = vec![
        good.path(),
        std::path::Path::new("/no/such/file.wav"),
    ];
    let results = analyze_album_files(&paths);
    assert_eq!(results.len(), 2);
    assert!(results[0].is_ok());
    assert!(matches!(&results[1], Err(GainError::FileNotFound { .. })));
}
