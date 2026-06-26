use gain_error::GainError;
use gain_map::{
    AnalysisBundle, ContentClass, GainRegion, GainRecommendationMap,
    MeasureType, PresetId, RegionDecision, GAIN_MAP_SCHEMA_VERSION,
};

pub fn recommend_regions(
    bundle: &AnalysisBundle,
    measure: MeasureType,
    target_db: f32,
    preset_id: PresetId,
) -> Result<GainRecommendationMap, GainError> {
    let recommendations = bundle
        .regions
        .iter()
        .map(|region| {
            let start_time = region.start_sample as f64 / bundle.sample_rate as f64;
            let end_time   = region.end_sample   as f64 / bundle.sample_rate as f64;

            if region.content_class == ContentClass::Silence {
                return GainRegion {
                    start_time,
                    end_time,
                    analysis: region.clone(),
                    decision: RegionDecision {
                        is_applicable: false,
                        gain_db: 0.0,
                        confidence: 1.0,
                        reason: "silence".to_string(),
                    },
                };
            }

            let measured_db = match measure {
                MeasureType::Peak => region.measurements.peak_dbfs,
                MeasureType::Rms  => region.measurements.rms_dbfs,
                MeasureType::Lufs => region
                    .measurements
                    .integrated_lufs
                    .value
                    .unwrap_or(region.measurements.rms_dbfs),
            };
            let gain_db = target_db - measured_db;
            let measure_label = match measure {
                MeasureType::Peak => "Peak",
                MeasureType::Rms  => "RMS",
                MeasureType::Lufs => "LUFS",
            };

            GainRegion {
                start_time,
                end_time,
                analysis: region.clone(),
                decision: RegionDecision {
                    is_applicable: true,
                    gain_db,
                    confidence: region.classification_confidence,
                    reason: format!(
                        "Target {target_db:.1} dBFS via {measure_label}"
                    ),
                },
            }
        })
        .collect();

    Ok(GainRecommendationMap {
        version:      GAIN_MAP_SCHEMA_VERSION,
        preset_used:  Some(preset_id),
        recommendations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gain_map::{
        AnalysisBundle, ContentClass, MeasurementQuality, MeasurementValue,
        Measurements, PresetId, RegionAnalysis,
    };

    fn placeholder_measurements(peak: f32, rms: f32) -> Measurements {
        Measurements {
            peak_dbfs: peak, rms_dbfs: rms, crest_factor_db: peak - rms,
            integrated_lufs:      MeasurementValue { value: None, quality: MeasurementQuality::Placeholder },
            short_term_lufs_peak: MeasurementValue { value: None, quality: MeasurementQuality::Placeholder },
            momentary_lufs_peak:  MeasurementValue { value: None, quality: MeasurementQuality::Placeholder },
            true_peak_dbtp:       MeasurementValue { value: None, quality: MeasurementQuality::Placeholder },
        }
    }

    fn single_region_bundle(peak: f32, rms: f32, class: ContentClass, total: usize) -> AnalysisBundle {
        let r = RegionAnalysis::from_classification(
            0, total, placeholder_measurements(peak, rms), class, 1.0, None,
        );
        AnalysisBundle { regions: vec![r], sample_rate: 44100, total_samples: total }
    }

    fn stable_bundle(peak: f32, rms: f32, total: usize) -> AnalysisBundle {
        let r = RegionAnalysis::whole_file_stable(placeholder_measurements(peak, rms), total);
        AnalysisBundle { regions: vec![r], sample_rate: 44100, total_samples: total }
    }

    #[test]
    fn peak_target_minus_12_with_peak_minus_6_gives_minus_6_gain() {
        let bundle = stable_bundle(-6.0, -10.0, 44100);
        let map = recommend_regions(&bundle, MeasureType::Peak, -12.0, PresetId::MixPrepStandard).unwrap();
        assert_eq!(map.recommendations.len(), 1);
        assert!((map.recommendations[0].decision.gain_db - (-6.0)).abs() < 0.001);
    }

    #[test]
    fn rms_target_minus_18_with_rms_minus_20_gives_plus_2_gain() {
        let bundle = stable_bundle(-14.0, -20.0, 44100 * 3);
        let map = recommend_regions(&bundle, MeasureType::Rms, -18.0, PresetId::AnalogConsole).unwrap();
        assert!((map.recommendations[0].decision.gain_db - 2.0).abs() < 0.001);
    }

    #[test]
    fn silence_region_has_is_applicable_false() {
        let bundle = single_region_bundle(-120.0, -120.0, ContentClass::Silence, 44100);
        let map = recommend_regions(&bundle, MeasureType::Peak, -12.0, PresetId::MixPrepStandard).unwrap();
        assert!(!map.recommendations[0].decision.is_applicable);
    }

    #[test]
    fn active_region_has_is_applicable_true() {
        let bundle = single_region_bundle(-12.0, -18.0, ContentClass::Music, 44100);
        let map = recommend_regions(&bundle, MeasureType::Peak, -12.0, PresetId::MixPrepStandard).unwrap();
        assert!(map.recommendations[0].decision.is_applicable);
    }

    #[test]
    fn silence_region_gain_is_zero() {
        let bundle = single_region_bundle(-120.0, -120.0, ContentClass::Silence, 44100);
        let map = recommend_regions(&bundle, MeasureType::Peak, -12.0, PresetId::MixPrepStandard).unwrap();
        assert_eq!(map.recommendations[0].decision.gain_db, 0.0);
    }

    #[test]
    fn multi_region_produces_one_recommendation_per_region() {
        let m1 = placeholder_measurements(-12.0, -18.0);
        let m2 = placeholder_measurements(-120.0, -120.0);
        let r1 = RegionAnalysis::from_classification(0, 22050, m1, ContentClass::Music, 0.8, None);
        let r2 = RegionAnalysis::from_classification(22050, 44100, m2, ContentClass::Silence, 1.0, None);
        let bundle = AnalysisBundle { regions: vec![r1, r2], sample_rate: 44100, total_samples: 44100 };
        let map = recommend_regions(&bundle, MeasureType::Peak, -12.0, PresetId::MixPrepStandard).unwrap();
        assert_eq!(map.recommendations.len(), 2);
        assert!(map.recommendations[0].decision.is_applicable);
        assert!(!map.recommendations[1].decision.is_applicable);
    }

    #[test]
    fn recommendation_times_derived_from_sample_rate() {
        let bundle = stable_bundle(-12.0, -18.0, 44100);
        let map = recommend_regions(&bundle, MeasureType::Peak, -12.0, PresetId::MixPrepStandard).unwrap();
        assert_eq!(map.recommendations[0].start_time, 0.0);
        assert!((map.recommendations[0].end_time - 1.0).abs() < 0.001);
    }

    #[test]
    fn preset_used_is_set_in_map() {
        let bundle = stable_bundle(-14.0, -20.0, 44100);
        let map = recommend_regions(&bundle, MeasureType::Rms, -18.0, PresetId::AnalogConsoleHot).unwrap();
        assert_eq!(map.preset_used, Some(PresetId::AnalogConsoleHot));
    }

    #[test]
    fn schema_version_is_two() {
        let bundle = stable_bundle(-12.0, -18.0, 44100);
        let map = recommend_regions(&bundle, MeasureType::Peak, -12.0, PresetId::MixPrepStandard).unwrap();
        assert_eq!(map.version, 2);
    }

    #[test]
    fn lufs_measure_uses_integrated_lufs_when_verified() {
        let mut m = placeholder_measurements(-12.0, -18.0);
        m.integrated_lufs = MeasurementValue::verified(-16.0);
        let r = RegionAnalysis::from_classification(0, 44100, m, ContentClass::Music, 0.8, None);
        let bundle = AnalysisBundle { regions: vec![r], sample_rate: 44100, total_samples: 44100 };
        let map = recommend_regions(&bundle, MeasureType::Lufs, -23.0, PresetId::PodcastPrep).unwrap();
        assert!((map.recommendations[0].decision.gain_db - (-7.0)).abs() < 0.001);
    }
}
