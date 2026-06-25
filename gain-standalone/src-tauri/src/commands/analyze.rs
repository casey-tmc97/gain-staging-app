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
