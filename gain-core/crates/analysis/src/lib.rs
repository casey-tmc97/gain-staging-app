mod loudness;

use audio_ingestion::AudioBuffer;
use gain_error::GainError;
use gain_map::Measurements;

const SILENCE_FLOOR_DBFS: f32 = -120.0;

pub fn measure(buf: &AudioBuffer) -> Result<Measurements, GainError> {
    if buf.samples.is_empty() {
        return Err(GainError::InvalidAudio { details: "empty audio buffer".to_string() });
    }

    for s in &buf.samples {
        if !s.is_finite() {
            return Err(GainError::InvalidAudio { details: format!("non-finite sample: {s}") });
        }
        if s.abs() > 1.0 {
            return Err(GainError::InvalidAudio { details: format!("sample out of [-1,1] range: {s}") });
        }
    }

    let max_amplitude = buf.samples.iter().map(|s| s.abs()).fold(0f32, f32::max);
    let peak_dbfs = if max_amplitude == 0.0 {
        SILENCE_FLOOR_DBFS
    } else {
        20.0 * max_amplitude.log10()
    };

    let n = buf.samples.len() as f32;
    let sum_sq: f32 = buf.samples.iter().map(|s| s * s).sum();
    let rms = (sum_sq / n).sqrt();
    let rms_dbfs = if rms == 0.0 {
        SILENCE_FLOOR_DBFS
    } else {
        20.0 * rms.log10()
    };

    let crest_factor_db = peak_dbfs - rms_dbfs;
    let loudness = loudness::measure_loudness(buf)?;

    Ok(Measurements {
        peak_dbfs,
        rms_dbfs,
        crest_factor_db,
        integrated_lufs:    loudness.integrated_lufs,
        short_term_lufs_peak: loudness.short_term_lufs_peak,
        momentary_lufs_peak:  loudness.momentary_lufs_peak,
        true_peak_dbtp:       loudness.true_peak_dbtp,
    })
}

/// Measure a sub-region of `buf` between `start_sample` and `end_sample` (exclusive).
pub fn measure_region(
    buf: &AudioBuffer,
    start_sample: usize,
    end_sample: usize,
) -> Result<Measurements, GainError> {
    let end = end_sample.min(buf.samples.len());
    if start_sample >= end {
        return Err(GainError::InvalidAudio {
            details: format!("empty region [{start_sample}, {end})"),
        });
    }
    let region_buf = AudioBuffer {
        samples: buf.samples[start_sample..end].to_vec(),
        sample_rate: buf.sample_rate,
        channels: buf.channels,
    };
    measure(&region_buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gain_map::MeasurementQuality;

    fn buf(samples: Vec<f32>) -> AudioBuffer {
        AudioBuffer { samples, sample_rate: 44100, channels: 1 }
    }

    fn sine_buf(amplitude: f32, n_samples: usize) -> AudioBuffer {
        let samples: Vec<f32> = (0..n_samples)
            .map(|i| amplitude * (2.0 * std::f32::consts::PI * i as f32 / 100.0).sin())
            .collect();
        buf(samples)
    }

    #[test]
    fn full_scale_peak_is_zero_dbfs() {
        let result = measure(&buf(vec![1.0f32; 1024])).unwrap();
        assert!((result.peak_dbfs - 0.0).abs() < 0.001);
    }

    #[test]
    fn half_amplitude_peak_is_approx_minus_6() {
        let result = measure(&buf(vec![0.5f32; 1024])).unwrap();
        assert!((result.peak_dbfs - (-6.0206)).abs() < 0.001);
    }

    #[test]
    fn constant_signal_crest_factor_is_zero() {
        let result = measure(&buf(vec![0.5f32; 1024])).unwrap();
        assert!((result.crest_factor_db - 0.0).abs() < 0.001);
    }

    #[test]
    fn sine_wave_crest_factor_is_approx_3() {
        let samples: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f32::consts::PI * i as f32 / 100.0).sin())
            .collect();
        let result = measure(&buf(samples)).unwrap();
        assert!((result.peak_dbfs - 0.0).abs() < 0.01);
        assert!((result.rms_dbfs - (-3.0103)).abs() < 0.05);
        assert!((result.crest_factor_db - 3.0103).abs() < 0.05);
    }

    #[test]
    fn silent_audio_returns_silence_floor() {
        let result = measure(&buf(vec![0.0f32; 1024])).unwrap();
        assert_eq!(result.peak_dbfs, -120.0);
        assert_eq!(result.rms_dbfs, -120.0);
    }

    #[test]
    fn empty_buffer_returns_error() {
        let result = measure(&buf(vec![]));
        assert!(matches!(result, Err(gain_error::GainError::InvalidAudio { .. })));
    }

    #[test]
    fn nan_sample_returns_error() {
        let result = measure(&buf(vec![f32::NAN]));
        assert!(matches!(result, Err(gain_error::GainError::InvalidAudio { .. })));
    }

    #[test]
    fn out_of_range_sample_returns_error() {
        let result = measure(&buf(vec![1.5f32]));
        assert!(matches!(result, Err(gain_error::GainError::InvalidAudio { .. })));
    }

    #[test]
    fn long_sine_has_verified_integrated_lufs() {
        let b = sine_buf(0.5, 44100 * 5);
        let result = measure(&b).unwrap();
        assert_eq!(result.integrated_lufs.quality, MeasurementQuality::Verified);
        assert!(result.integrated_lufs.value.is_some());
    }

    #[test]
    fn long_sine_integrated_lufs_in_expected_range() {
        let b = sine_buf(0.5, 44100 * 5);
        let result = measure(&b).unwrap();
        let lufs = result.integrated_lufs.value.unwrap();
        assert!(lufs > -15.0 && lufs < -5.0,
            "expected LUFS in (-15, -5), got {lufs}");
    }

    #[test]
    fn long_sine_has_verified_true_peak() {
        let b = sine_buf(0.5, 44100 * 5);
        let result = measure(&b).unwrap();
        assert_eq!(result.true_peak_dbtp.quality, MeasurementQuality::Verified);
        assert!(result.true_peak_dbtp.value.is_some());
    }

    #[test]
    fn short_buffer_lufs_is_placeholder() {
        let b = buf(vec![0.5f32; 100]);
        let result = measure(&b).unwrap();
        assert_eq!(result.integrated_lufs.quality, MeasurementQuality::Placeholder);
    }

    #[test]
    fn measure_region_gives_correct_peak() {
        let mut samples = vec![0.0f32; 2000];
        for s in &mut samples[500..1000] { *s = 0.5; }
        let full_buf = AudioBuffer { samples, sample_rate: 44100, channels: 1 };
        let result = measure_region(&full_buf, 500, 1000).unwrap();
        assert!((result.peak_dbfs - (-6.0206)).abs() < 0.01,
            "expected peak ≈ -6.02, got {}", result.peak_dbfs);
    }

    #[test]
    fn measure_region_outside_bounds_returns_error() {
        let b = buf(vec![0.5f32; 100]);
        let result = measure_region(&b, 200, 300);
        assert!(matches!(result, Err(gain_error::GainError::InvalidAudio { .. })));
    }
}
