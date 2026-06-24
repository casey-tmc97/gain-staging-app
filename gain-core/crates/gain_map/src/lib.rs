pub const GAIN_MAP_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq)]
pub enum MeasurementQuality {
    Placeholder,
    Estimated,
    Verified,
}

#[derive(Debug, Clone)]
pub struct MeasurementValue {
    pub value: Option<f32>,
    pub quality: MeasurementQuality,
}

#[derive(Debug, Clone)]
pub struct Measurements {
    pub peak_dbfs: f32,
    pub rms_dbfs: f32,
    pub crest_factor_db: f32,
    pub integrated_lufs: MeasurementValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PresetId {
    MixPrepConservative,
    MixPrepStandard,
    MixPrepAggressive,
    AnalogConsole,
    AnalogConsoleHot,
    DialoguePrep,
    Custom,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RegionType {
    Stable,
    Transient,
    EnvelopeControlled,
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

#[derive(Debug, Clone)]
pub struct GainRecommendationMap {
    pub version: u32,
    pub preset_used: Option<PresetId>,
    pub regions: Vec<GainRegion>,
}

impl Default for GainRecommendationMap {
    fn default() -> Self {
        Self { version: GAIN_MAP_SCHEMA_VERSION, preset_used: None, regions: Vec::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_region_fields_are_accessible() {
        let region = GainRegion {
            start_time: 0.0, end_time: 1.5, gain_db: -3.0,
            confidence: 0.85, region_type: RegionType::Stable, reason: "test".to_string(),
        };
        assert_eq!(region.start_time, 0.0);
        assert_eq!(region.gain_db, -3.0);
        assert_eq!(region.reason, "test");
    }

    #[test]
    fn gain_recommendation_map_default_is_empty() {
        let map = GainRecommendationMap::default();
        assert!(map.regions.is_empty());
    }

    #[test]
    fn gain_recommendation_map_default_has_no_preset() {
        let map = GainRecommendationMap::default();
        assert_eq!(map.preset_used, None);
    }

    #[test]
    fn gain_recommendation_map_can_hold_regions() {
        let mut map = GainRecommendationMap::default();
        map.regions.push(GainRegion {
            start_time: 0.0, end_time: 2.0, gain_db: 6.0,
            confidence: 1.0, region_type: RegionType::Transient, reason: "peak".to_string(),
        });
        assert_eq!(map.regions.len(), 1);
    }

    #[test]
    fn region_type_envelope_controlled_exists() {
        assert_eq!(RegionType::EnvelopeControlled, RegionType::EnvelopeControlled);
    }

    #[test]
    fn gain_recommendation_map_default_version_is_one() {
        assert_eq!(GainRecommendationMap::default().version, 1);
    }

    #[test]
    fn measurement_quality_placeholder_exists() {
        assert_eq!(MeasurementQuality::Placeholder, MeasurementQuality::Placeholder);
    }

    #[test]
    fn measurement_value_none_when_placeholder() {
        let v = MeasurementValue { value: None, quality: MeasurementQuality::Placeholder };
        assert!(v.value.is_none());
    }

    #[test]
    fn measurements_struct_accessible() {
        let m = Measurements {
            peak_dbfs: -12.0, rms_dbfs: -18.0, crest_factor_db: 6.0,
            integrated_lufs: MeasurementValue { value: None, quality: MeasurementQuality::Placeholder },
        };
        assert_eq!(m.peak_dbfs, -12.0);
        assert!(m.integrated_lufs.value.is_none());
    }

    #[test]
    fn preset_id_variants_exist() {
        let _ = PresetId::MixPrepConservative;
        let _ = PresetId::MixPrepStandard;
        let _ = PresetId::MixPrepAggressive;
        let _ = PresetId::AnalogConsole;
        let _ = PresetId::AnalogConsoleHot;
        let _ = PresetId::DialoguePrep;
        let _ = PresetId::Custom;
    }
}
