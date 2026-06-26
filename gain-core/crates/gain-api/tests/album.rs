use gain_api::{
    analyze_album_files, compute_album_anchor, generate_album_recommendations,
    AlbumAnchorMethod, GainError, RecommendationPreset,
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

fn temp_wav(amplitude: f32) -> tempfile::NamedTempFile {
    let wav = make_wav_constant(amplitude, 44100 * 5, 44100);
    let mut f = tempfile::NamedTempFile::with_suffix(".wav").unwrap();
    f.write_all(&wav).unwrap();
    f
}

#[test]
fn analyze_album_files_returns_one_result_per_path() {
    let f1 = temp_wav(0.5);
    let f2 = temp_wav(0.3);
    let paths: Vec<&std::path::Path> = vec![f1.path(), f2.path()];
    let results = analyze_album_files(&paths);
    assert_eq!(results.len(), 2);
    assert!(results[0].is_ok());
    assert!(results[1].is_ok());
}

#[test]
fn analyze_album_files_missing_file_is_err_not_panic() {
    let f1 = temp_wav(0.5);
    let paths: Vec<&std::path::Path> = vec![
        f1.path(),
        std::path::Path::new("/no/such/file.wav"),
    ];
    let results = analyze_album_files(&paths);
    assert_eq!(results.len(), 2);
    assert!(results[0].is_ok());
    assert!(matches!(&results[1], Err(GainError::FileNotFound { .. })));
}

#[test]
fn compute_album_anchor_uses_median_lufs() {
    let f1 = temp_wav(0.5);
    let f2 = temp_wav(0.3);
    let f3 = temp_wav(0.1);
    let paths: Vec<&std::path::Path> = vec![f1.path(), f2.path(), f3.path()];
    let results: Vec<_> = analyze_album_files(&paths)
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();
    let anchor = compute_album_anchor(&results).unwrap();
    assert!(matches!(anchor.method, AlbumAnchorMethod::Median));
    // median LUFS of 3 files at different amplitudes
    let lufs_values: Vec<f32> = results.iter()
        .filter_map(|r| r.measurements.integrated_lufs.value)
        .collect();
    assert_eq!(lufs_values.len(), 3, "all files should have Verified LUFS");
    let mut sorted = lufs_values.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert!((anchor.target_lufs - sorted[1]).abs() < 0.001,
        "anchor should be median LUFS");
}

#[test]
fn compute_album_anchor_fails_when_no_lufs_data() {
    // Very short files produce Placeholder LUFS
    let wav = make_wav_constant(0.5, 100, 44100);
    let mut f = tempfile::NamedTempFile::with_suffix(".wav").unwrap();
    f.write_all(&wav).unwrap();
    let results: Vec<_> = analyze_album_files(&[f.path()])
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();
    let result = compute_album_anchor(&results);
    assert!(matches!(result, Err(GainError::AnalysisFailure { .. })));
}

#[test]
fn generate_album_recommendations_returns_one_map_per_file() {
    let f1 = temp_wav(0.5);
    let f2 = temp_wav(0.3);
    let paths: Vec<&std::path::Path> = vec![f1.path(), f2.path()];
    let results: Vec<_> = analyze_album_files(&paths)
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();
    let anchor = compute_album_anchor(&results).unwrap();
    let maps = generate_album_recommendations(
        &results, &anchor, RecommendationPreset::AlbumConsistency,
    ).unwrap();
    assert_eq!(maps.len(), 2);
    for map in &maps {
        assert_eq!(map.recommendations.len(), 1);
    }
}

#[test]
fn generate_album_recommendations_louder_file_gets_negative_gain() {
    // file at 0.8 amplitude (loud) should get negative gain relative to a softer anchor
    let loud  = temp_wav(0.8);
    let soft1 = temp_wav(0.2);
    let soft2 = temp_wav(0.2);
    let paths: Vec<&std::path::Path> = vec![loud.path(), soft1.path(), soft2.path()];
    let results: Vec<_> = analyze_album_files(&paths)
        .into_iter()
        .filter_map(|r| r.ok())
        .collect();
    let anchor = compute_album_anchor(&results).unwrap();
    // anchor ≈ median (soft files' LUFS) — louder than loud file's LUFS
    let maps = generate_album_recommendations(
        &results, &anchor, RecommendationPreset::AlbumConsistency,
    ).unwrap();
    // The loud file (index 0) should get a negative gain recommendation
    let loud_gain = maps[0].recommendations[0].decision.gain_db;
    assert!(loud_gain < 0.0,
        "louder-than-anchor file should get negative gain, got {loud_gain}");
}
