pub struct AudioBuffer {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

pub fn load_file(_path: &std::path::Path) -> Result<AudioBuffer, String> {
    Err("not implemented".to_string())
}
