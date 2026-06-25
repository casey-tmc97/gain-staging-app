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
    MixPrepConservative,              // Peak -18 dBFS
    MixPrepStandard,                  // Peak -12 dBFS
    MixPrepAggressive,                // Peak -6 dBFS
    AnalogConsole,                    // RMS -18 dBFS
    AnalogConsoleHot,                 // RMS -14 dBFS
    DialoguePrep,                     // Peak -10 dBFS
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
