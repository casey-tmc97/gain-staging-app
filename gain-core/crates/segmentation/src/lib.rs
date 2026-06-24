pub struct Segment {
    pub start_sample: usize,
    pub end_sample: usize,
}

pub fn segment(_samples: &[f32], _sample_rate: u32) -> Vec<Segment> {
    vec![]
}
