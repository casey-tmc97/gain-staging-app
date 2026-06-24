#pragma once
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handle to a GainRecommendationMap allocated in Rust. */
typedef struct GainStageMap GainStageMap;

/* region_type values */
#define GAIN_STAGE_REGION_STABLE    0
#define GAIN_STAGE_REGION_TRANSIENT 1
#define GAIN_STAGE_REGION_ENVELOPE  2
#define GAIN_STAGE_REGION_MIXED     3

/* POD struct safe to copy across the FFI boundary. */
typedef struct {
    double  start_time;
    double  end_time;
    float   gain_db;
    float   confidence;
    uint8_t region_type;  /* see GAIN_STAGE_REGION_* constants */
    uint8_t reason[64];   /* null-terminated UTF-8 */
} CGainRegion;

/*
 * Analyze raw audio and return an opaque GainStageMap handle.
 * samples     - interleaved f32 PCM samples (mono expected at scaffold level)
 * count       - total number of samples
 * sample_rate - samples per second (e.g. 44100)
 * Returns NULL only on allocation failure.
 * Caller must release with gain_stage_free_map().
 */
GainStageMap* gain_stage_analyze(
    const float*  samples,
    size_t        count,
    uint32_t      sample_rate
);

/* Release a GainStageMap. Passing NULL is a no-op. */
void gain_stage_free_map(GainStageMap* map);

/* Return the number of regions. Returns 0 if map is NULL. */
size_t gain_stage_map_region_count(const GainStageMap* map);

/*
 * Return a copy of the region at index.
 * Returns a zeroed CGainRegion if map is NULL or index is out of range.
 */
CGainRegion gain_stage_map_get_region(const GainStageMap* map, size_t index);

#ifdef __cplusplus
} /* extern "C" */
#endif
