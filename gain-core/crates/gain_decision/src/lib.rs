use gain_map::GainRecommendationMap;

pub fn decide(_samples: &[f32], _sample_rate: u32) -> GainRecommendationMap {
    GainRecommendationMap::default()
}
