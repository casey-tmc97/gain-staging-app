pub use gain_map::{GainRecommendationMap, GainRegion, RegionType};

#[derive(Debug)]
pub enum GainError {
    FileNotFound(String),
    UnsupportedFormat(String),
    AnalysisFailed(String),
}

/// Analyze an audio file and return a GainRecommendationMap.
/// Stub: returns an empty map with version = 1 regardless of input.
pub fn analyze_file(path: &std::path::Path) -> Result<GainRecommendationMap, GainError> {
    let _ = path;
    Ok(GainRecommendationMap::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_file_stub_returns_default_map() {
        let result = analyze_file(std::path::Path::new("/fake/path.wav"));
        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(map.version, 1);
        assert!(map.regions.is_empty());
    }

    #[test]
    fn gain_error_variants_exist() {
        let _ = GainError::FileNotFound("x".to_string());
        let _ = GainError::UnsupportedFormat("x".to_string());
        let _ = GainError::AnalysisFailed("x".to_string());
    }
}
