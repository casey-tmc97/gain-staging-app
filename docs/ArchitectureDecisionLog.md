# Architecture Decision Log
## Gain Stage App

---

## ADR Template

```
### ADR-NNN: [Title]

**Date:** YYYY-MM-DD
**Status:** Accepted | Superseded by ADR-NNN | Deprecated

**Context:** [What situation led to this decision?]

**Decision:** [What was decided?]

**Consequences:** [What are the trade-offs and implications?]
```

---

### ADR-001: Rust for the core analysis engine

**Date:** 2026-06-23
**Status:** Accepted

**Context:** The analysis engine requires deterministic, high-performance
batch processing of audio data. Memory safety is critical to avoid subtle
bugs in signal processing code.

**Decision:** Implement the entire analysis engine in Rust as a Cargo
workspace (`gain-core`). No other language may contain analysis logic.

**Consequences:** C++ callers (ARA plugin) must go through a C ABI bridge.
Tauri (Rust) callers use the Rust API directly via `gain-api`.

---

### ADR-002: C ABI FFI with opaque handle pattern

**Date:** 2026-06-23
**Status:** Accepted

**Context:** The ARA plugin is written in C++ and must call into the Rust
core. Rust objects cannot be shared directly across a language boundary.

**Decision:** Expose a C ABI from the `ffi` crate. Use opaque handles
(`GainStageMap*`) allocated on the Rust heap via `Box::into_raw`. Pass
only POD types across the boundary. Provide explicit free functions.

**Consequences:** No exceptions may cross the boundary. Callers are
responsible for calling the free function. The C header
(`gain_stage_ffi.h`) is the authoritative contract.

---

### ADR-003: Tauri for the standalone desktop app

**Date:** 2026-06-23
**Status:** Accepted

**Context:** A desktop app is needed for file-based offline analysis.
Electron was considered but carries a large runtime overhead.

**Decision:** Use Tauri 2 for the standalone app. The Tauri backend
(Rust) imports `gain-api` directly. The frontend (HTML/JS) communicates
via Tauri commands.

**Consequences:** The Tauri backend is pure Rust and shares types with
`gain-core` at zero cost. The frontend build pipeline (if any) is
separate from the Cargo build.

---

### ADR-004: ARA protocol for DAW integration

**Date:** 2026-06-23
**Status:** Accepted

**Context:** DAW integration requires access to audio content in a
non-destructive, host-managed way. Standard VST/AU plugin APIs do not
provide offline access to audio buffers.

**Decision:** Implement the DAW plugin using the ARA 2 SDK (Celemony).
The plugin receives audio via ARA host callbacks and passes it to the
Rust FFI. No DSP runs in the C++ layer.

**Consequences:** Requires the ARA SDK (separate license from Celemony).
The plugin is only compatible with ARA-capable hosts (Pro Tools, Logic,
Studio One, Reaper with ARA extension).

---

### ADR-005: Recommendation-first workflow

**Date:** 2026-06-23
**Status:** Accepted

**Context:** Gain staging tools that modify audio automatically create trust
problems: engineers cannot tell what changed, cannot revert selectively, and
cannot apply professional judgment before committing. Automatic gain
application is a destructive operation that bypasses human decision-making.

**Decision:** The engine never modifies audio. It produces recommendations
only. All gain adjustments require explicit user action. The
`GainRecommendationMap` is a read-only output; no write-back path exists
in the core engine.

**Consequences:** The core engine has no "apply" function. Consumers
(standalone app, ARA plugin) are responsible for presenting recommendations
and gating any application behind user confirmation. This eliminates an
entire class of data-loss bugs.

---

### ADR-006: Gain Map schema versioning

**Date:** 2026-06-23
**Status:** Accepted

**Context:** The `GainRecommendationMap` structure will evolve as DSP
analysis is added in later phases. Without a version field, consumers
cannot detect stale or incompatible map data, and serialized maps have
no migration path.

**Decision:** Every `GainRecommendationMap` carries a `version: u32` field.
The current schema version is defined as `GAIN_MAP_SCHEMA_VERSION = 1` in
`gain_map` and re-exported through `gain-api`. The `Default` impl always
stamps the current version. Consumers must reject maps with an unrecognized
version rather than silently misinterpreting them.

**Consequences:** Adding fields to `GainRecommendationMap` requires bumping
`GAIN_MAP_SCHEMA_VERSION`. Deserialization code must check the version field
before interpreting the rest of the struct.

---

### ADR-007: `gain-api` as sole public surface of `gain-core`

**Date:** 2026-06-23
**Status:** Accepted

**Context:** Without an explicit public API boundary, consumers
(`gain-standalone`, `gain-ara`, future SDKs) would import internal
implementation crates directly. This creates tight coupling and makes
internal refactoring impossible without breaking consumers.

**Decision:** Introduce `gain-core/crates/gain-api` as the sole public
façade. It re-exports stable types and exposes `analyze_file()`.
All code outside `gain-core` must import only `gain-api`. The `ffi`
crate, despite being inside `gain-core`, also imports via `gain-api` to
enforce the same discipline.

**Consequences:** Internal crates (`segmentation`, `analysis`,
`classification`, `gain_decision`, `gain_map`) may be reorganized,
split, or renamed freely. `gain-api` is the only contract that must
remain stable across internal refactors.
