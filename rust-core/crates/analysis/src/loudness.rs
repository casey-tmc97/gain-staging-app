pub struct LoudnessResult {
    pub integrated_lufs: f32,
    pub short_term_lufs: Vec<f32>,
}

pub fn analyze_loudness(_samples: &[f32], _sample_rate: u32) -> LoudnessResult {
    LoudnessResult { integrated_lufs: 0.0, short_term_lufs: vec![] }
}
