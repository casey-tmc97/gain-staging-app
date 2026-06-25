use gain_error::GainError;
use gain_map::{
    GainRecommendationMap, GainRegion, Measurements,
    PresetId, RegionType, GAIN_MAP_SCHEMA_VERSION,
};

#[derive(Debug, Clone, PartialEq)]
pub enum MeasureType {
    Peak,
    Rms,
}

pub fn recommend(
    measurements: &Measurements,
    measure: MeasureType,
    target_db: f32,
    duration_secs: f64,
    preset_id: PresetId,
) -> Result<GainRecommendationMap, GainError> {
    let (measured_db, measure_label) = match &measure {
        MeasureType::Peak => (measurements.peak_dbfs, "Peak"),
        MeasureType::Rms  => (measurements.rms_dbfs,  "RMS"),
    };

    let gain_db = target_db - measured_db;
    let reason  = format!("Applied target of {target_db:.1} dBFS using {measure_label} measurement");

    Ok(GainRecommendationMap {
        version:     GAIN_MAP_SCHEMA_VERSION,
        preset_used: Some(preset_id),
        regions: vec![GainRegion {
            start_time:  0.0,
            end_time:    duration_secs,
            gain_db,
            confidence:  1.0,
            region_type: RegionType::Stable,
            reason,
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gain_map::{RegionType, MeasurementQuality, MeasurementValue};

    fn placeholder_measurements(peak: f32, rms: f32) -> Measurements {
        Measurements {
            peak_dbfs: peak, rms_dbfs: rms, crest_factor_db: peak - rms,
            integrated_lufs: MeasurementValue { value: None, quality: MeasurementQuality::Placeholder },
        }
    }

    #[test]
    fn peak_target_minus_12_with_peak_minus_6_gives_minus_6_gain() {
        let m = placeholder_measurements(-6.0, -10.0);
        let map = recommend(&m, MeasureType::Peak, -12.0, 2.0, PresetId::MixPrepStandard).unwrap();
        assert!((map.regions[0].gain_db - (-6.0)).abs() < 0.001);
    }

    #[test]
    fn rms_target_minus_18_with_rms_minus_20_gives_plus_2_gain() {
        let m = placeholder_measurements(-14.0, -20.0);
        let map = recommend(&m, MeasureType::Rms, -18.0, 3.0, PresetId::AnalogConsole).unwrap();
        assert!((map.regions[0].gain_db - 2.0).abs() < 0.001);
    }

    #[test]
    fn region_spans_full_duration() {
        let m = placeholder_measurements(-12.0, -18.0);
        let map = recommend(&m, MeasureType::Peak, -12.0, 5.5, PresetId::MixPrepStandard).unwrap();
        assert_eq!(map.regions.len(), 1);
        assert_eq!(map.regions[0].start_time, 0.0);
        assert!((map.regions[0].end_time - 5.5).abs() < 0.001);
    }

    #[test]
    fn region_type_is_stable() {
        let m = placeholder_measurements(-12.0, -18.0);
        let map = recommend(&m, MeasureType::Peak, -12.0, 1.0, PresetId::MixPrepStandard).unwrap();
        assert_eq!(map.regions[0].region_type, RegionType::Stable);
    }

    #[test]
    fn confidence_is_one() {
        let m = placeholder_measurements(-12.0, -18.0);
        let map = recommend(&m, MeasureType::Peak, -12.0, 1.0, PresetId::MixPrepStandard).unwrap();
        assert_eq!(map.regions[0].confidence, 1.0);
    }

    #[test]
    fn preset_used_is_set() {
        let m = placeholder_measurements(-12.0, -18.0);
        let map = recommend(&m, MeasureType::Rms, -14.0, 1.0, PresetId::AnalogConsoleHot).unwrap();
        assert_eq!(map.preset_used, Some(PresetId::AnalogConsoleHot));
    }
}
