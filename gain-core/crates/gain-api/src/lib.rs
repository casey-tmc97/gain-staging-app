pub use gain_error::GainError;
pub use gain_map::{
    AlbumAnchor, AlbumAnchorMethod, AnalysisBundle, ContentClass,
    GainRecommendationMap, GainRegion, Measurements, MeasurementQuality,
    MeasurementValue, MeasureType, PresetId, RegionAnalysis, RegionDecision,
    RegionType, GAIN_MAP_SCHEMA_VERSION,
};
pub use audio_ingestion::{AudioBuffer, AudioMetadata, ContainerFormat};

#[derive(Debug)]
pub struct AnalysisResult {
    pub metadata:     AudioMetadata,
    pub measurements: Measurements,
}

#[derive(Debug)]
pub enum RecommendationPreset {
    MixPrepConservative,
    MixPrepStandard,
    MixPrepAggressive,
    AnalogConsole,
    AnalogConsoleHot,
    DialoguePrep,
    DialogueBroadcast,
    PodcastPrep,
    VoiceoverPrep,
    MusicStemPrep,
    FilmDialogue,
    AlbumConsistency,
    Custom { measure: MeasureType, target_db: f32 },
}

fn preset_to_params(preset: RecommendationPreset) -> (MeasureType, f32, PresetId) {
    match preset {
        RecommendationPreset::MixPrepConservative => (MeasureType::Peak, -18.0, PresetId::MixPrepConservative),
        RecommendationPreset::MixPrepStandard     => (MeasureType::Peak, -12.0, PresetId::MixPrepStandard),
        RecommendationPreset::MixPrepAggressive   => (MeasureType::Peak,  -6.0, PresetId::MixPrepAggressive),
        RecommendationPreset::AnalogConsole        => (MeasureType::Rms,  -18.0, PresetId::AnalogConsole),
        RecommendationPreset::AnalogConsoleHot     => (MeasureType::Rms,  -14.0, PresetId::AnalogConsoleHot),
        RecommendationPreset::DialoguePrep         => (MeasureType::Peak, -10.0, PresetId::DialoguePrep),
        RecommendationPreset::DialogueBroadcast    => (MeasureType::Lufs, -24.0, PresetId::DialogueBroadcast),
        RecommendationPreset::PodcastPrep          => (MeasureType::Lufs, -16.0, PresetId::PodcastPrep),
        RecommendationPreset::VoiceoverPrep        => (MeasureType::Lufs, -18.0, PresetId::VoiceoverPrep),
        RecommendationPreset::MusicStemPrep        => (MeasureType::Peak, -12.0, PresetId::MusicStemPrep),
        RecommendationPreset::FilmDialogue         => (MeasureType::Lufs, -27.0, PresetId::FilmDialogue),
        RecommendationPreset::AlbumConsistency     => (MeasureType::Lufs,   0.0, PresetId::AlbumConsistency),
        RecommendationPreset::Custom { measure, target_db } => (measure, target_db, PresetId::Custom),
    }
}

/// Step 1 (Phase 2): decode file and measure whole-file Peak/RMS/LUFS.
pub fn analyze_file(path: &std::path::Path) -> Result<AnalysisResult, GainError> {
    let (buf, metadata) = audio_ingestion::load_file(path)?;
    let measurements = analysis::measure(&buf)?;
    Ok(AnalysisResult { metadata, measurements })
}

/// Step 1 variant: measure raw PCM already in memory.
pub fn analyze_pcm(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
    duration_secs: f64,
) -> Result<AnalysisResult, GainError> {
    let buf = AudioBuffer { samples: samples.to_vec(), sample_rate, channels };
    let measurements = analysis::measure(&buf)?;
    let metadata = AudioMetadata {
        duration_secs, sample_rate, channels, format: ContainerFormat::Wav,
    };
    Ok(AnalysisResult { metadata, measurements })
}

/// Step 2 (Phase 2): apply preset to whole-file AnalysisResult.
/// Internally wraps into a single-region bundle and calls recommend_regions().
pub fn generate_recommendation(
    analysis: &AnalysisResult,
    preset: RecommendationPreset,
) -> Result<GainRecommendationMap, GainError> {
    let total_samples =
        (analysis.metadata.duration_secs * analysis.metadata.sample_rate as f64) as usize;
    let bundle = AnalysisBundle {
        regions: vec![RegionAnalysis::whole_file_stable(
            analysis.measurements.clone(),
            total_samples,
        )],
        sample_rate: analysis.metadata.sample_rate,
        total_samples,
    };
    let (measure, target_db, preset_id) = preset_to_params(preset);
    gain_decision::recommend_regions(&bundle, measure, target_db, preset_id)
}

/// Step 1 (Phase 3): decode file, segment, and classify each region.
pub fn analyze_regions(path: &std::path::Path) -> Result<AnalysisBundle, GainError> {
    let (buf, _meta) = audio_ingestion::load_file(path)?;
    let total_samples = buf.samples.len();
    let sample_rate = buf.sample_rate;
    let segments = segmentation::segment(&buf.samples, buf.sample_rate);
    let regions = classification::classify_segments(&buf, &segments)?;
    Ok(AnalysisBundle { regions, sample_rate, total_samples })
}

/// Step 2 (Phase 3): apply preset to each region in a bundle.
pub fn generate_region_recommendations(
    bundle: &AnalysisBundle,
    preset: RecommendationPreset,
) -> Result<GainRecommendationMap, GainError> {
    let (measure, target_db, preset_id) = preset_to_params(preset);
    gain_decision::recommend_regions(bundle, measure, target_db, preset_id)
}

/// Album pass 1: analyze each file independently.
/// Returns one Result per path; individual failures do not abort the batch.
pub fn analyze_album_files(
    paths: &[&std::path::Path],
) -> Vec<Result<AnalysisResult, GainError>> {
    paths.iter().map(|p| analyze_file(p)).collect()
}

/// Album pass 2: compute a loudness anchor from successful analyses.
/// Dispatches on `method` to pick the anchor value.
/// Falls back to rms_dbfs when integrated LUFS is unavailable for a track.
/// Returns AnalysisFailure if `results` is empty.
pub fn compute_album_anchor(
    results: &[AnalysisResult],
    method: AlbumAnchorMethod,
) -> Result<AlbumAnchor, GainError> {
    if results.is_empty() {
        return Err(GainError::AnalysisFailure {
            details: "no results to anchor".to_string(),
        });
    }

    let mut lufs_values: Vec<f32> = results
        .iter()
        .map(|r| {
            r.measurements
                .integrated_lufs
                .value
                .unwrap_or(r.measurements.rms_dbfs)
        })
        .collect();

    let target_lufs = match method {
        AlbumAnchorMethod::Median => {
            lufs_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let len = lufs_values.len();
            if len % 2 == 0 {
                (lufs_values[len / 2 - 1] + lufs_values[len / 2]) / 2.0
            } else {
                lufs_values[len / 2]
            }
        }
        AlbumAnchorMethod::Maximum => lufs_values
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max),
        AlbumAnchorMethod::Custom(v) => v,
    };

    Ok(AlbumAnchor {
        target_lufs,
        method,
    })
}

/// Album pass 3: generate per-file gain recommendations relative to the anchor.
pub fn generate_album_recommendations(
    results: &[AnalysisResult],
    anchor: &AlbumAnchor,
    preset: RecommendationPreset,
) -> Result<Vec<GainRecommendationMap>, GainError> {
    let (_, _, preset_id) = preset_to_params(preset);
    results
        .iter()
        .map(|result| {
            let total_samples =
                (result.metadata.duration_secs * result.metadata.sample_rate as f64) as usize;
            let bundle = AnalysisBundle {
                regions: vec![RegionAnalysis::whole_file_stable(
                    result.measurements.clone(),
                    total_samples,
                )],
                sample_rate: result.metadata.sample_rate,
                total_samples,
            };
            gain_decision::recommend_regions(
                &bundle,
                MeasureType::Lufs,
                anchor.target_lufs,
                preset_id.clone(),
            )
        })
        .collect()
}
