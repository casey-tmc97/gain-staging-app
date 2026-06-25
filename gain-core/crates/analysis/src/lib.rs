use audio_ingestion::AudioBuffer;
use gain_error::GainError;
use gain_map::{MeasurementQuality, MeasurementValue, Measurements};

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

    Ok(Measurements {
        peak_dbfs,
        rms_dbfs,
        crest_factor_db,
        integrated_lufs: MeasurementValue {
            value: None,
            quality: MeasurementQuality::Placeholder,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(samples: Vec<f32>) -> AudioBuffer {
        AudioBuffer { samples, sample_rate: 44100, channels: 1 }
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
        // sine peak = 1.0, rms = 1/sqrt(2) ≈ 0.7071 → rms_dbfs ≈ -3.0103
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
        assert_eq!(result.crest_factor_db, 0.0);
    }

    #[test]
    fn lufs_is_always_placeholder_none() {
        let result = measure(&buf(vec![0.5f32; 1024])).unwrap();
        assert_eq!(result.integrated_lufs.quality, MeasurementQuality::Placeholder);
        assert!(result.integrated_lufs.value.is_none());
    }

    #[test]
    fn empty_buffer_returns_error() {
        let result = measure(&buf(vec![]));
        assert!(matches!(result, Err(GainError::InvalidAudio { .. })));
    }

    #[test]
    fn nan_sample_returns_error() {
        let result = measure(&buf(vec![f32::NAN]));
        assert!(matches!(result, Err(GainError::InvalidAudio { .. })));
    }

    #[test]
    fn out_of_range_sample_returns_error() {
        let result = measure(&buf(vec![1.5f32]));
        assert!(matches!(result, Err(GainError::InvalidAudio { .. })));
    }
}
