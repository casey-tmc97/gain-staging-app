use crate::dto::{GainMapDto, GainRegionDto};
use gain_api::RecommendationPreset;

fn preset_from_u8(code: u8) -> Result<RecommendationPreset, String> {
    match code {
        0  => Ok(RecommendationPreset::MixPrepConservative),
        1  => Ok(RecommendationPreset::MixPrepStandard),
        2  => Ok(RecommendationPreset::MixPrepAggressive),
        3  => Ok(RecommendationPreset::AnalogConsole),
        4  => Ok(RecommendationPreset::AnalogConsoleHot),
        5  => Ok(RecommendationPreset::DialoguePrep),
        6  => Ok(RecommendationPreset::DialogueBroadcast),
        7  => Ok(RecommendationPreset::PodcastPrep),
        8  => Ok(RecommendationPreset::VoiceoverPrep),
        9  => Ok(RecommendationPreset::MusicStemPrep),
        10 => Ok(RecommendationPreset::FilmDialogue),
        11 => Ok(RecommendationPreset::AlbumConsistency),
        n  => Err(format!("unknown preset code {n}")),
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
        version:      map.version,
        preset_used:  map.preset_used.map(|p| format!("{p:?}")),
        recommendations: map.recommendations.iter().map(|rec| GainRegionDto {
            start_time:    rec.start_time,
            end_time:      rec.end_time,
            gain_db:       rec.decision.gain_db,
            confidence:    rec.decision.confidence,
            is_applicable: rec.decision.is_applicable,
            region_type:   format!("{:?}", rec.analysis.region_type()),
            content_class: format!("{:?}", rec.analysis.content_class),
        }).collect(),
    })
}
