#pragma once
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handle to a GainRecommendationMap allocated in Rust. */
typedef struct GainStageMap GainStageMap;

/* region_type values */
#define GAIN_STAGE_REGION_STABLE              0
#define GAIN_STAGE_REGION_TRANSIENT           1
#define GAIN_STAGE_REGION_ENVELOPE_CONTROLLED 2
#define GAIN_STAGE_REGION_MIXED               3

/* Preset codes for gain_stage_analyze_file */
#define GAIN_STAGE_PRESET_MIX_PREP_CONSERVATIVE  0
#define GAIN_STAGE_PRESET_MIX_PREP_STANDARD      1
#define GAIN_STAGE_PRESET_MIX_PREP_AGGRESSIVE    2
#define GAIN_STAGE_PRESET_ANALOG_CONSOLE         3
#define GAIN_STAGE_PRESET_ANALOG_CONSOLE_HOT     4
#define GAIN_STAGE_PRESET_DIALOGUE_PREP          5

/* Error codes returned by gain_stage_last_error_code */
#define GAIN_STAGE_ERR_NONE             0
#define GAIN_STAGE_ERR_FILE_NOT_FOUND   1
#define GAIN_STAGE_ERR_UNSUPPORTED_FMT  2
#define GAIN_STAGE_ERR_DECODE_FAILURE   3
#define GAIN_STAGE_ERR_INVALID_AUDIO    4
#define GAIN_STAGE_ERR_ANALYSIS_FAILURE 5
#define GAIN_STAGE_ERR_INTERNAL         6

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
 * Analyze raw interleaved f32 PCM (assumed mono) and return a GainStageMap.
 * Uses MixPrepStandard preset (-12 dBFS Peak).
 * Returns NULL on error; call gain_stage_last_error_code() for details.
 * Caller must release with gain_stage_free_map().
 */
GainStageMap* gain_stage_analyze(
    const float*  samples,
    size_t        count,
    uint32_t      sample_rate
);

/*
 * Analyze an audio file and return a GainStageMap.
 * path   - null-terminated UTF-8 file path (WAV or AIFF)
 * preset - GAIN_STAGE_PRESET_* constant (unknown values return NULL)
 * Returns NULL on error; call gain_stage_last_error_code() for details.
 * Caller must release with gain_stage_free_map().
 *
 * WARNING: Not re-entrant. Treat as single-threaded at the call site.
 * Thread-local error state may produce incorrect results if called
 * concurrently from the same host thread pool.
 */
GainStageMap* gain_stage_analyze_file(const char* path, uint8_t preset);

/* Release a GainStageMap. Passing NULL is a no-op. */
void gain_stage_free_map(GainStageMap* map);

/* Return the number of regions. Returns 0 if map is NULL. */
size_t gain_stage_map_region_count(const GainStageMap* map);

/*
 * Return a copy of the region at index.
 * Returns a zeroed CGainRegion if map is NULL or index is out of range.
 */
CGainRegion gain_stage_map_get_region(const GainStageMap* map, size_t index);

/* Return the schema version of the map. Returns 0 if map is NULL. */
uint32_t gain_stage_map_version(const GainStageMap* map);

/* Return the error code from the most recent failed call on this thread (0 = no error). */
uint8_t gain_stage_last_error_code(void);

/*
 * Return a null-terminated error message for the last failed call on this thread.
 * The pointer is valid only until the next gain_stage_* call on this thread.
 */
const char* gain_stage_last_error_message(void);

#ifdef __cplusplus
} /* extern "C" */
#endif
