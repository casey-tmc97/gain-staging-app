#[derive(Debug, Clone, PartialEq)]
pub enum RegionType {
    Stable,
    Transient,
    Envelope,
    Mixed,
}

#[derive(Debug, Clone)]
pub struct GainRegion {
    pub start_time: f64,
    pub end_time: f64,
    pub gain_db: f32,
    pub confidence: f32,
    pub region_type: RegionType,
    pub reason: String,
}

#[derive(Debug, Clone, Default)]
pub struct GainRecommendationMap {
    pub regions: Vec<GainRegion>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_region_fields_are_accessible() {
        let region = GainRegion {
            start_time: 0.0,
            end_time: 1.5,
            gain_db: -3.0,
            confidence: 0.85,
            region_type: RegionType::Stable,
            reason: "test".to_string(),
        };
        assert_eq!(region.start_time, 0.0);
        assert_eq!(region.end_time, 1.5);
        assert_eq!(region.gain_db, -3.0);
        assert_eq!(region.confidence, 0.85);
        assert_eq!(region.reason, "test");
    }

    #[test]
    fn gain_recommendation_map_default_is_empty() {
        let map = GainRecommendationMap::default();
        assert!(map.regions.is_empty());
    }

    #[test]
    fn gain_recommendation_map_can_hold_regions() {
        let mut map = GainRecommendationMap::default();
        map.regions.push(GainRegion {
            start_time: 0.0,
            end_time: 2.0,
            gain_db: 6.0,
            confidence: 1.0,
            region_type: RegionType::Transient,
            reason: "peak".to_string(),
        });
        assert_eq!(map.regions.len(), 1);
    }
}
