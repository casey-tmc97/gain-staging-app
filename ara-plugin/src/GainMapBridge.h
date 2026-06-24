#pragma once

#include <cstddef>
#include <string>
#include <vector>

// C++ RAII wrapper around the Rust FFI GainStageMap handle.
// Acquires a map on construction, releases it on destruction.
// No exceptions are thrown — check is_valid() before use.

struct GainRegionCxx {
    double      start_time;
    double      end_time;
    float       gain_db;
    float       confidence;
    uint8_t     region_type;  // 0=Stable 1=Transient 2=Envelope 3=Mixed
    std::string reason;
};

class GainMapBridge {
public:
    // Analyze audio samples. stub: always returns an empty map.
    GainMapBridge(const float* samples, size_t count, uint32_t sample_rate) noexcept;
    ~GainMapBridge() noexcept;

    // Non-copyable — owns the Rust handle
    GainMapBridge(const GainMapBridge&) = delete;
    GainMapBridge& operator=(const GainMapBridge&) = delete;

    bool              is_valid()      const noexcept;
    size_t            region_count()  const noexcept;
    GainRegionCxx     get_region(size_t index) const noexcept;

private:
    struct GainStageMap* map_ = nullptr;
};
