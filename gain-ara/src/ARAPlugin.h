#pragma once

#include "../stubs/ara_stubs.h"
#include "GainMapBridge.h"

#include <memory>

// ARA plugin session stub.
// Receives audio from the DAW host and delegates analysis to GainMapBridge.
// No DSP logic here — transport and session management only.

class ARAPlugin : public ARA::ARADocumentControllerInterface {
public:
    ARAPlugin() = default;
    ~ARAPlugin() override = default;

    // Called by the host when audio data is available for analysis.
    // stub: creates a GainMapBridge and discards the result.
    void analyzeAudioSource(
        ARA::ARAAudioSourceRef source,
        const float*           samples,
        size_t                 count,
        uint32_t               sample_rate) noexcept;

    // ARADocumentControllerInterface stubs
    void notifyAudioSourceAnalysisProgress(
        ARA::ARAAudioSourceRef audioSource, float progress) override;

private:
    std::unique_ptr<GainMapBridge> current_map_;
};
