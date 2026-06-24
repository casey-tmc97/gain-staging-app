# Gain Recommendation Map Schema

## GainRegion

| Field | Type | Description |
|-------|------|-------------|
| start_time | f64 (seconds) | Region start in seconds |
| end_time | f64 (seconds) | Region end in seconds |
| gain_db | f32 | Recommended gain adjustment in dB |
| confidence | f32 | Confidence score 0.0–1.0 |
| region_type | RegionType | Stable / Transient / Envelope / Mixed |
| reason | String | Human-readable reasoning tag |

## RegionType

- `Stable` — sustained, steady-level content
- `Transient` — sharp attack/impact content
- `Envelope` — shaping-driven dynamic content
- `Mixed` — overlapping characteristics

## GainRecommendationMap

A time-ordered list of non-overlapping `GainRegion` entries covering the analyzed audio.
