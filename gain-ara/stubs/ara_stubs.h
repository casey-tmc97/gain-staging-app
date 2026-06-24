#pragma once

// Placeholder ARA SDK types.
// Replace this entire file with the real ARA SDK headers when integrating.

#include <cstdint>

namespace ARA {

struct ARAAudioSourceRef { uint64_t id = 0; };
struct ARADocumentControllerRef { uint64_t id = 0; };
struct ARAPlaybackRegionRef { uint64_t id = 0; };

enum class ARAContentType : uint32_t {
    kARAContentTypeNotes = 1,
    kARAContentTypeTempoEntries = 2,
};

struct ARADocumentControllerInterface {
    virtual ~ARADocumentControllerInterface() = default;
    virtual void notifyAudioSourceAnalysisProgress(
        ARAAudioSourceRef audioSource, float progress) = 0;
};

} // namespace ARA
