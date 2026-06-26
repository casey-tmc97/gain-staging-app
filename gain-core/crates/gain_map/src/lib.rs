pub const GAIN_MAP_SCHEMA_VERSION: u32 = 2;

// === Measurement quality ===

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

impl MeasurementValue {
    pub fn placeholder() -> Self {
        Self { value: None, quality: MeasurementQuality::Placeholder }
    }
    pub fn verified(v: f32) -> Self {
        Self { value: Some(v), quality: MeasurementQuality::Verified }
    }
}

// === Measurements (Phase 3: adds LUFS + True Peak) ===

#[derive(Debug, Clone)]
pub struct Measurements {
    pub peak_dbfs: f32,
    pub rms_dbfs: f32,
    pub crest_factor_db: f32,
    pub integrated_lufs: MeasurementValue,
    pub short_term_lufs_peak: MeasurementValue,   // max 3-second window
    pub momentary_lufs_peak: MeasurementValue,    // max 400ms window
    pub true_peak_dbtp: MeasurementValue,
}

// === Content classification ===

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContentClass {
    Silence,
    Dialogue,
    Music,
    Ambience,
    Percussive,
    Mixed,
    Unknown,
}

// === Region type (output-facing label) ===
// Stable: Phase 2 compat path only — produced exclusively by whole_file_stable().
// All other variants are produced via From<ContentClass>.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RegionType {
    Stable,
    Silence,
    Dialogue,
    Music,
    Ambience,
    Percussive,
    Mixed,
    Unknown,
}

impl From<ContentClass> for RegionType {
    fn from(c: ContentClass) -> Self {
        match c {
            ContentClass::Silence    => RegionType::Silence,
            ContentClass::Dialogue   => RegionType::Dialogue,
            ContentClass::Music      => RegionType::Music,
            ContentClass::Ambience   => RegionType::Ambience,
            ContentClass::Percussive => RegionType::Percussive,
            ContentClass::Mixed      => RegionType::Mixed,
            ContentClass::Unknown    => RegionType::Unknown,
        }
    }
}

// === RegionAnalysis ===
// `region_type` is private to prevent (content_class, region_type) mismatches.
// Constructors enforce the invariant: from_classification derives region_type
// via From<ContentClass>; whole_file_stable forces RegionType::Stable.

#[derive(Debug, Clone)]
pub struct RegionAnalysis {
    pub start_sample: usize,
    pub end_sample: usize,
    pub measurements: Measurements,
    pub content_class: ContentClass,
    pub classification_confidence: f32,
    pub classification_vector: Option<Vec<f32>>,
    region_type: RegionType,
}

impl RegionAnalysis {
    pub fn region_type(&self) -> RegionType {
        self.region_type
    }

    pub fn from_classification(
        start_sample: usize,
        end_sample: usize,
        measurements: Measurements,
        content_class: ContentClass,
        classification_confidence: f32,
        classification_vector: Option<Vec<f32>>,
    ) -> Self {
        let region_type = RegionType::from(content_class);
        Self {
            start_sample,
            end_sample,
            measurements,
            content_class,
            classification_confidence,
            classification_vector,
            region_type,
        }
    }

    // Phase 2 compat path: treats the whole file as a single stable region.
    pub fn whole_file_stable(measurements: Measurements, total_samples: usize) -> Self {
        Self {
            start_sample: 0,
            end_sample: total_samples,
            measurements,
            content_class: ContentClass::Unknown,
            classification_confidence: 1.0,
            classification_vector: None,
            region_type: RegionType::Stable,
        }
    }
}

// === Decision output ===

#[derive(Debug, Clone)]
pub struct RegionDecision {
    pub is_applicable: bool,
    pub gain_db: f32,
    pub confidence: f32,
    pub reason: String,
}

// === Combined output bundle ===

#[derive(Debug, Clone)]
pub struct GainRegion {
    pub start_time: f64,
    pub end_time: f64,
    pub analysis: RegionAnalysis,
    pub decision: RegionDecision,
}

// === Analysis bundle (output of analyze_regions) ===

#[derive(Debug)]
pub struct AnalysisBundle {
    pub regions: Vec<RegionAnalysis>,
    pub sample_rate: u32,
    pub total_samples: usize,
}

// === Album consistency ===

#[derive(Debug, Clone)]
pub enum AlbumAnchorMethod {
    Median,
    Maximum,
    Custom(f32),
}

#[derive(Debug, Clone)]
pub struct AlbumAnchor {
    pub target_lufs: f32,
    pub method: AlbumAnchorMethod,
}

// === MeasureType (moved here from gain_decision) ===

#[derive(Debug, Clone, PartialEq)]
pub enum MeasureType {
    Peak,
    Rms,
    Lufs,
}

// === PresetId ===

#[derive(Debug, Clone, PartialEq)]
pub enum PresetId {
    // Phase 2 presets
    MixPrepConservative,
    MixPrepStandard,
    MixPrepAggressive,
    AnalogConsole,
    AnalogConsoleHot,
    DialoguePrep,
    // Phase 3 presets
    DialogueBroadcast,
    PodcastPrep,
    VoiceoverPrep,
    MusicStemPrep,
    FilmDialogue,
    AlbumConsistency,
    Custom,
}

// === Recommendation map ===

#[derive(Debug)]
pub struct GainRecommendationMap {
    pub version: u32,
    pub preset_used: Option<PresetId>,
    pub recommendations: Vec<GainRegion>,
}

impl Default for GainRecommendationMap {
    fn default() -> Self {
        Self {
            version: GAIN_MAP_SCHEMA_VERSION,
            preset_used: None,
            recommendations: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placeholder_measurements() -> Measurements {
        Measurements {
            peak_dbfs: -12.0, rms_dbfs: -18.0, crest_factor_db: 6.0,
            integrated_lufs: MeasurementValue::placeholder(),
            short_term_lufs_peak: MeasurementValue::placeholder(),
            momentary_lufs_peak: MeasurementValue::placeholder(),
            true_peak_dbtp: MeasurementValue::placeholder(),
        }
    }

    #[test]
    fn schema_version_is_two() {
        assert_eq!(GAIN_MAP_SCHEMA_VERSION, 2);
    }

    #[test]
    fn content_class_to_region_type_maps_all_variants() {
        assert_eq!(RegionType::from(ContentClass::Silence),    RegionType::Silence);
        assert_eq!(RegionType::from(ContentClass::Dialogue),   RegionType::Dialogue);
        assert_eq!(RegionType::from(ContentClass::Music),      RegionType::Music);
        assert_eq!(RegionType::from(ContentClass::Ambience),   RegionType::Ambience);
        assert_eq!(RegionType::from(ContentClass::Percussive), RegionType::Percussive);
        assert_eq!(RegionType::from(ContentClass::Mixed),      RegionType::Mixed);
        assert_eq!(RegionType::from(ContentClass::Unknown),    RegionType::Unknown);
    }

    #[test]
    fn from_classification_region_type_matches_content_class() {
        let r = RegionAnalysis::from_classification(
            0, 1000, placeholder_measurements(),
            ContentClass::Dialogue, 0.8, None,
        );
        assert_eq!(r.region_type(), RegionType::Dialogue);
        assert_eq!(r.content_class, ContentClass::Dialogue);
    }

    #[test]
    fn whole_file_stable_has_stable_region_type() {
        let r = RegionAnalysis::whole_file_stable(placeholder_measurements(), 44100);
        assert_eq!(r.region_type(), RegionType::Stable);
        assert_eq!(r.start_sample, 0);
        assert_eq!(r.end_sample, 44100);
    }

    #[test]
    fn measurement_value_placeholder_has_no_value() {
        let v = MeasurementValue::placeholder();
        assert!(v.value.is_none());
        assert_eq!(v.quality, MeasurementQuality::Placeholder);
    }

    #[test]
    fn measurement_value_verified_holds_value() {
        let v = MeasurementValue::verified(-14.0);
        assert_eq!(v.value, Some(-14.0));
        assert_eq!(v.quality, MeasurementQuality::Verified);
    }

    #[test]
    fn gain_recommendation_map_default_uses_recommendations_field() {
        let map = GainRecommendationMap::default();
        assert!(map.recommendations.is_empty());
        assert_eq!(map.version, 2);
    }

    #[test]
    fn measure_type_lufs_variant_exists() {
        assert_eq!(MeasureType::Lufs, MeasureType::Lufs);
    }

    #[test]
    fn new_preset_id_variants_exist() {
        let _ = PresetId::DialogueBroadcast;
        let _ = PresetId::PodcastPrep;
        let _ = PresetId::VoiceoverPrep;
        let _ = PresetId::MusicStemPrep;
        let _ = PresetId::FilmDialogue;
        let _ = PresetId::AlbumConsistency;
    }

    #[test]
    fn region_decision_is_applicable_false_for_silence() {
        let d = RegionDecision {
            is_applicable: false, gain_db: 0.0,
            confidence: 1.0, reason: "silence".to_string(),
        };
        assert!(!d.is_applicable);
    }

    #[test]
    fn analysis_bundle_holds_sample_rate() {
        let bundle = AnalysisBundle {
            regions: vec![],
            sample_rate: 48000,
            total_samples: 0,
        };
        assert_eq!(bundle.sample_rate, 48000);
    }
}
