use audio_ingestion::AudioBuffer;
use gain_error::GainError;
use gain_map::{ContentClass, Measurements, RegionAnalysis};
use segmentation::Segment;

/// Apply 7 deterministic rules to produce a ContentClass and confidence.
///
/// Rules are evaluated in priority order; first match wins.
/// Rule 1 — Silence:    RMS < -60 dBFS
/// Rule 2 — Percussive: crest > 18 dB
/// Rule 3 — Dialogue:   crest in [8, 18] AND RMS >= -40 dBFS
/// Rule 4 — Music:      crest < 8 AND RMS >= -24 dBFS
/// Rule 5 — Ambience:   RMS in (-60, -40) dBFS
/// Rule 6 — Mixed:      crest >= 8 AND RMS >= -40 (did not match Dialogue)
/// Rule 7 — Unknown:    fallback
fn classify_measurements(m: &Measurements) -> (ContentClass, f32) {
    if m.rms_dbfs < -60.0 {
        return (ContentClass::Silence, 1.0);
    }
    if m.crest_factor_db > 18.0 {
        return (ContentClass::Percussive, 0.8);
    }
    if m.crest_factor_db >= 8.0 && m.crest_factor_db <= 18.0 && m.rms_dbfs >= -40.0 {
        return (ContentClass::Dialogue, 0.75);
    }
    if m.crest_factor_db < 8.0 && m.rms_dbfs >= -24.0 {
        return (ContentClass::Music, 0.8);
    }
    if m.rms_dbfs > -60.0 && m.rms_dbfs < -40.0 {
        return (ContentClass::Ambience, 0.7);
    }
    if m.crest_factor_db >= 8.0 && m.rms_dbfs >= -40.0 {
        return (ContentClass::Mixed, 0.6);
    }
    (ContentClass::Unknown, 0.5)
}

pub fn classify_segments(
    buf: &AudioBuffer,
    segments: &[Segment],
) -> Result<Vec<RegionAnalysis>, GainError> {
    segments
        .iter()
        .map(|seg| {
            let measurements =
                analysis::measure_region(buf, seg.start_sample, seg.end_sample)?;
            let (content_class, confidence) = classify_measurements(&measurements);
            Ok(RegionAnalysis::from_classification(
                seg.start_sample,
                seg.end_sample,
                measurements,
                content_class,
                confidence,
                None,
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use audio_ingestion::AudioBuffer;
    use gain_map::ContentClass;

    fn make_buf(samples: Vec<f32>) -> AudioBuffer {
        AudioBuffer { samples, sample_rate: 44100, channels: 1 }
    }

    fn whole_file_segment(n: usize) -> Segment {
        Segment { start_sample: 0, end_sample: n }
    }

    fn classify_one(samples: Vec<f32>) -> ContentClass {
        let n = samples.len();
        let buf = make_buf(samples);
        let seg = whole_file_segment(n);
        classify_segments(&buf, &[seg]).unwrap()[0].content_class
    }

    #[test]
    fn silence_classifies_as_silence() {
        assert_eq!(classify_one(vec![0.0f32; 44100]), ContentClass::Silence);
    }

    #[test]
    fn constant_sine_classifies_as_music() {
        let samples: Vec<f32> = (0..44100)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * i as f32 / 100.0).sin())
            .collect();
        assert_eq!(classify_one(samples), ContentClass::Music);
    }

    #[test]
    fn sparse_impulses_classify_as_percussive() {
        let mut samples = vec![0.0f32; 44100];
        for i in (0..44100).step_by(4410) { samples[i] = 1.0; }
        assert_eq!(classify_one(samples), ContentClass::Percussive);
    }

    #[test]
    fn bursty_signal_classifies_as_dialogue() {
        let mut samples = vec![0.001f32; 44100];
        for i in (0..44100).step_by(10) { samples[i] = 0.3; }
        assert_eq!(classify_one(samples), ContentClass::Dialogue);
    }

    #[test]
    fn quiet_noise_classifies_as_ambience() {
        assert_eq!(classify_one(vec![0.008f32; 44100]), ContentClass::Ambience);
    }

    #[test]
    fn classify_returns_one_region_per_segment() {
        let buf = make_buf(vec![0.5f32; 44100]);
        let segs = vec![
            Segment { start_sample: 0, end_sample: 22050 },
            Segment { start_sample: 22050, end_sample: 44100 },
        ];
        let regions = classify_segments(&buf, &segs).unwrap();
        assert_eq!(regions.len(), 2);
    }

    #[test]
    fn region_start_and_end_sample_match_segment() {
        let buf = make_buf(vec![0.5f32; 44100]);
        let seg = Segment { start_sample: 1000, end_sample: 5000 };
        let regions = classify_segments(&buf, &[seg]).unwrap();
        assert_eq!(regions[0].start_sample, 1000);
        assert_eq!(regions[0].end_sample, 5000);
    }

    #[test]
    fn silence_region_confidence_is_one() {
        let buf = make_buf(vec![0.0f32; 44100]);
        let seg = whole_file_segment(44100);
        let regions = classify_segments(&buf, &[seg]).unwrap();
        assert_eq!(regions[0].content_class, ContentClass::Silence);
        assert_eq!(regions[0].classification_confidence, 1.0);
    }
}
