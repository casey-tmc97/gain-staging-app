# Test Assets

Reference audio files used by integration tests.

Do not commit large binary files here. Add file names to `.gitignore`
and fetch via a test fixture script (to be defined in Phase 2).

## Required files (to be added before Phase 2 integration tests)

| File | Purpose |
|------|---------|
| `sine_440hz_-18dBFS_5s.wav` | Stable tone, known loudness for loudness analysis tests |
| `kick_transient.wav` | Sharp transient for transient detection tests |
| `envelope_swell.wav` | Slow amplitude swell for envelope detection tests |
| `mixed_content.wav` | Composite content for classification tests |

## Format

WAV, 44100 Hz, 32-bit float, mono.
Files must be royalty-free or synthetically generated.
