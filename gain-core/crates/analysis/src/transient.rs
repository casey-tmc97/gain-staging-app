pub struct TransientResult {
    pub onset_times_sec: Vec<f64>,
}

pub fn detect_transients(_samples: &[f32], _sample_rate: u32) -> TransientResult {
    TransientResult { onset_times_sec: vec![] }
}
