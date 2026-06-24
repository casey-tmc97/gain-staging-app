#[tauri::command]
pub fn get_version() -> u32 {
    gain_api::GainRecommendationMap::default().version
}
