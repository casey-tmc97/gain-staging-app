use crate::dto::{GainMapDto, GainRegionDto};

#[tauri::command]
pub fn analyze(path: String) -> Result<GainMapDto, String> {
    let map = gain_api::analyze_file(std::path::Path::new(&path))
        .map_err(|e| format!("{e:?}"))?;
    Ok(GainMapDto {
        version: map.version,
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
