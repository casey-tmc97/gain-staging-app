#[derive(Debug)]
pub enum GainError {
    FileNotFound    { path: String },
    UnsupportedFormat { format: String },
    DecodeFailure   { details: String },
    InvalidAudio    { details: String },
    AnalysisFailure { details: String },
    InternalError   { details: String },
}

impl std::fmt::Display for GainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GainError::FileNotFound    { path }    => write!(f, "file not found: {path}"),
            GainError::UnsupportedFormat { format } => write!(f, "unsupported format: {format}"),
            GainError::DecodeFailure   { details } => write!(f, "decode failure: {details}"),
            GainError::InvalidAudio    { details } => write!(f, "invalid audio: {details}"),
            GainError::AnalysisFailure { details } => write!(f, "analysis failure: {details}"),
            GainError::InternalError   { details } => write!(f, "internal error: {details}"),
        }
    }
}

impl std::error::Error for GainError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_error_variants_display() {
        assert!(GainError::FileNotFound    { path: "/a.wav".into() }.to_string().contains("/a.wav"));
        assert!(GainError::UnsupportedFormat { format: ".mp3".into() }.to_string().contains(".mp3"));
        assert!(GainError::DecodeFailure   { details: "eof".into() }.to_string().contains("eof"));
        assert!(GainError::InvalidAudio    { details: "nan".into() }.to_string().contains("nan"));
        assert!(GainError::AnalysisFailure { details: "empty".into() }.to_string().contains("empty"));
        assert!(GainError::InternalError   { details: "oops".into() }.to_string().contains("oops"));
    }
}
