#pragma once
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handle to a GainRecommendationMap allocated in Rust. */
typedef struct GainStageMap GainStageMap;

/* Opaque handle to a classified AnalysisBundle allocated in Rust. */
typedef struct GainStageAnalysis GainStageAnalysis;

/* region_type values */
#define GAIN_STAGE_REGION_TYPE_STABLE     0
#define GAIN_STAGE_REGION_TYPE_SILENCE    1
#define GAIN_STAGE_REGION_TYPE_DIALOGUE   2
#define GAIN_STAGE_REGION_TYPE_MUSIC      3
#define GAIN_STAGE_REGION_TYPE_AMBIENCE   4
#define GAIN_STAGE_REGION_TYPE_PERCUSSIVE 5
#define GAIN_STAGE_REGION_TYPE_MIXED      6
#define GAIN_STAGE_REGION_TYPE_UNKNOWN    7

/* content_class values */
#define GAIN_STAGE_CLASS_SILENCE    0
#define GAIN_STAGE_CLASS_DIALOGUE   1
#define GAIN_STAGE_CLASS_MUSIC      2
#define GAIN_STAGE_CLASS_AMBIENCE   3
#define GAIN_STAGE_CLASS_PERCUSSIVE 4
#define GAIN_STAGE_CLASS_MIXED      5
#define GAIN_STAGE_CLASS_UNKNOWN    6

/* Preset codes for gain_stage_analyze_file / gain_stage_begin_analysis */
#define GAIN_STAGE_PRESET_MIX_PREP_CONSERVATIVE  0
#define GAIN_STAGE_PRESET_MIX_PREP_STANDARD      1
#define GAIN_STAGE_PRESET_MIX_PREP_AGGRESSIVE    2
#define GAIN_STAGE_PRESET_ANALOG_CONSOLE         3
#define GAIN_STAGE_PRESET_ANALOG_CONSOLE_HOT     4
#define GAIN_STAGE_PRESET_DIALOGUE_PREP          5
#define GAIN_STAGE_PRESET_FILM_DIALOGUE          6
#define GAIN_STAGE_PRESET_ALBUM_CONSISTENCY      7
#define GAIN_STAGE_PRESET_MUSIC_STEM_PREP        8
#define GAIN_STAGE_PRESET_ANALOGUE_CONSOLE       9
#define GAIN_STAGE_PRESET_ANALOGUE_HOT          10
#define GAIN_STAGE_PRESET_DIALOGUE_BROADCAST    11

/* Error codes returned by gain_stage_last_error_code */
#define GAIN_STAGE_ERR_NONE             0
#define GAIN_STAGE_ERR_FILE_NOT_FOUND   1
#define GAIN_STAGE_ERR_UNSUPPORTED_FMT  2
#define GAIN_STAGE_ERR_DECODE_FAILURE   3
#define GAIN_STAGE_ERR_INVALID_AUDIO    4
#define GAIN_STAGE_ERR_ANALYSIS_FAILURE 5
#define GAIN_STAGE_ERR_INTERNAL         6

/* POD struct safe to copy across the FFI boundary (Phase 2 / legacy). */
typedef struct {
    double  start_time;
    double  end_time;
    float   gain_db;
    float   confidence;
    uint8_t region_type;  /* see GAIN_STAGE_REGION_TYPE_* constants */
    uint8_t reason[64];   /* null-terminated UTF-8 */
} CGainRegion;

/* Phase 3 recommendation struct. */
typedef struct {
    double  start_time;
    double  end_time;
    uint8_t region_type;   /* GAIN_STAGE_REGION_TYPE_* */
    uint8_t content_class; /* GAIN_STAGE_CLASS_* */
    float   gain_db;
    uint8_t is_applicable; /* 0 = false, 1 = true */
    float   confidence;
} GainStageRegionV2;

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

/* ── Phase 3 two-step API ────────────────────────────────────────────────── */

/*
 * Step 1: decode, segment, and classify an audio file.
 * path     - UTF-8 file path, path_len bytes (need not be null-terminated)
 * preset_id - GAIN_STAGE_PRESET_* constant
 * Returns NULL on error; call gain_stage_last_error_code() for details.
 * Caller must release with gain_stage_free_analysis().
 */
GainStageAnalysis* gain_stage_begin_analysis(const char* path, uint8_t preset_id);

/*
 * Step 2: apply the preset stored in the analysis handle and produce a map.
 * Returns NULL on error. Caller must release with gain_stage_free_map().
 */
GainStageMap*      gain_stage_generate_recommendation(const GainStageAnalysis* analysis);

/* Release a GainStageAnalysis. Passing NULL is a no-op. */
void               gain_stage_free_analysis(GainStageAnalysis* analysis);

/* Return the number of Phase 3 recommendations. Returns 0 if map is NULL. */
uintptr_t          gain_stage_map_recommendation_count(const GainStageMap* map);

/*
 * Return a copy of the Phase 3 recommendation at index.
 * Returns a zeroed GainStageRegionV2 if map is NULL or index is out of range.
 */
GainStageRegionV2  gain_stage_map_get_recommendation(const GainStageMap* map, uintptr_t index);

/* ── Error accessors ─────────────────────────────────────────────────────── */

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
