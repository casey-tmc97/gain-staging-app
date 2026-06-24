pub enum SegmentClass {
    Stable,
    Transient,
    Envelope,
    Mixed,
}

pub struct ClassifiedSegment {
    pub start_sample: usize,
    pub end_sample: usize,
    pub class: SegmentClass,
    pub confidence: f32,
}

pub fn classify(_samples: &[f32], _sample_rate: u32) -> Vec<ClassifiedSegment> {
    vec![]
}
