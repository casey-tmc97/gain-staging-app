#[derive(serde::Serialize)]
pub struct GainMapDto {
    pub version:      u32,
    pub preset_used:  Option<String>,
    pub recommendations: Vec<GainRegionDto>,
}

#[derive(serde::Serialize)]
pub struct GainRegionDto {
    pub start_time:    f64,
    pub end_time:      f64,
    pub gain_db:       f32,
    pub confidence:    f32,
    pub is_applicable: bool,
    pub region_type:   String,
    pub content_class: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_region_dto_serializes_is_applicable() {
        let dto = GainRegionDto {
            start_time: 0.0, end_time: 1.0,
            gain_db: -3.0, confidence: 0.8,
            is_applicable: true,
            region_type: "Music".to_string(),
            content_class: "Music".to_string(),
        };
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"is_applicable\":true"));
        assert!(json.contains("\"content_class\":\"Music\""));
        assert!(!json.contains("\"reason\""), "reason must not appear in Phase 3 DTO");
    }

    #[test]
    fn gain_map_dto_uses_recommendations_field() {
        let dto = GainMapDto {
            version: 2,
            preset_used: Some("MixPrepStandard".to_string()),
            recommendations: vec![],
        };
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"recommendations\""));
        assert!(!json.contains("\"regions\""), "field must be recommendations, not regions");
    }
}
