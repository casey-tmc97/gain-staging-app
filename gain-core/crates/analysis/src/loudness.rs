use audio_ingestion::AudioBuffer;
use ebur128::{EbuR128, Mode};
use gain_error::GainError;
use gain_map::MeasurementValue;

pub struct LoudnessResult {
    pub integrated_lufs: MeasurementValue,
    pub short_term_lufs_peak: MeasurementValue,
    pub momentary_lufs_peak: MeasurementValue,
    pub true_peak_dbtp: MeasurementValue,
}

/// Compute BS.1770-4 LUFS and True Peak for the given buffer.
/// Returns placeholder values for buffers shorter than 400 ms.
pub fn measure_loudness(buf: &AudioBuffer) -> Result<LoudnessResult, GainError> {
    let min_samples = (buf.sample_rate as usize * 4 / 10) * buf.channels as usize;
    if buf.samples.len() < min_samples {
        return Ok(LoudnessResult {
            integrated_lufs: MeasurementValue::placeholder(),
            short_term_lufs_peak: MeasurementValue::placeholder(),
            momentary_lufs_peak: MeasurementValue::placeholder(),
            true_peak_dbtp: MeasurementValue::placeholder(),
        });
    }

    let mode = Mode::M | Mode::S | Mode::I | Mode::TRUE_PEAK;
    let mut ebu = EbuR128::new(buf.channels as u32, buf.sample_rate, mode)
        .map_err(|e| GainError::AnalysisFailure { details: format!("ebur128 init: {e}") })?;

    let mut max_short_term = f64::NEG_INFINITY;
    let mut max_momentary  = f64::NEG_INFINITY;

    // Process in 100 ms blocks to capture per-block short-term/momentary peaks.
    let block = ((buf.sample_rate as usize / 10) * buf.channels as usize).max(1);
    for chunk in buf.samples.chunks(block) {
        ebu.add_frames_f32(chunk)
            .map_err(|e| GainError::AnalysisFailure { details: format!("ebur128 feed: {e}") })?;

        if let Ok(st) = ebu.loudness_shortterm() {
            if st.is_finite() { max_short_term = max_short_term.max(st); }
        }
        if let Ok(m) = ebu.loudness_momentary() {
            if m.is_finite() { max_momentary = max_momentary.max(m); }
        }
    }

    let integrated = ebu.loudness_global()
        .map_err(|e| GainError::AnalysisFailure { details: format!("ebur128 global: {e}") })?;

    // True peak: maximum across all channels (linear scale → dBTP).
    let true_peak_linear = (0..buf.channels as u32)
        .filter_map(|ch| ebu.true_peak(ch).ok())
        .fold(0.0f64, f64::max);
    let true_peak_dbtp = if true_peak_linear > 0.0 {
        MeasurementValue::verified((20.0 * true_peak_linear.log10()) as f32)
    } else {
        MeasurementValue::placeholder()
    };

    Ok(LoudnessResult {
        integrated_lufs: if integrated.is_finite() {
            MeasurementValue::verified(integrated as f32)
        } else {
            MeasurementValue::placeholder()
        },
        short_term_lufs_peak: if max_short_term.is_finite() {
            MeasurementValue::verified(max_short_term as f32)
        } else {
            MeasurementValue::placeholder()
        },
        momentary_lufs_peak: if max_momentary.is_finite() {
            MeasurementValue::verified(max_momentary as f32)
        } else {
            MeasurementValue::placeholder()
        },
        true_peak_dbtp,
    })
}
