#[derive(serde::Serialize)]
pub struct GainMapDto {
    pub version: u32,
    pub regions: Vec<GainRegionDto>,
}

#[derive(serde::Serialize)]
pub struct GainRegionDto {
    pub start_time: f64,
    pub end_time: f64,
    pub gain_db: f32,
    pub confidence: f32,
    pub region_type: String,
    pub reason: String,
}
