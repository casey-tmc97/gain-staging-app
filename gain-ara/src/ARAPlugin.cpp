#include "ARAPlugin.h"

void ARAPlugin::analyzeAudioSource(
    ARA::ARAAudioSourceRef /*source*/,
    const float*           samples,
    size_t                 count,
    uint32_t               sample_rate) noexcept
{
    current_map_ = std::make_unique<GainMapBridge>(samples, count, sample_rate);
}

void ARAPlugin::notifyAudioSourceAnalysisProgress(
    ARA::ARAAudioSourceRef /*audioSource*/, float /*progress*/)
{
    // stub: no-op until real ARA SDK integration
}
