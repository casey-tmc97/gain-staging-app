#include "GainMapBridge.h"
#include "gain_stage_ffi.h"

#include <cstring>

GainMapBridge::GainMapBridge(
    const float* samples, size_t count, uint32_t sample_rate) noexcept
{
    map_ = gain_stage_analyze(samples, count, sample_rate);
}

GainMapBridge::~GainMapBridge() noexcept
{
    gain_stage_free_map(map_);
}

bool GainMapBridge::is_valid() const noexcept
{
    return map_ != nullptr;
}

size_t GainMapBridge::region_count() const noexcept
{
    return gain_stage_map_region_count(map_);
}

GainRegionCxx GainMapBridge::get_region(size_t index) const noexcept
{
    CGainRegion c = gain_stage_map_get_region(map_, index);
    GainRegionCxx r{};
    r.start_time  = c.start_time;
    r.end_time    = c.end_time;
    r.gain_db     = c.gain_db;
    r.confidence  = c.confidence;
    r.region_type = c.region_type;
    r.reason      = reinterpret_cast<const char*>(c.reason);
    return r;
}
