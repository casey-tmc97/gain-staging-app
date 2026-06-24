# Gain Stage App
## Phase 1A – Architecture Compliance Checklist

Version: 1.0
Applies To: Repository Scaffold Review
Required Before: Core Engine Implementation

# Section 1 — Workspace Architecture

- [ ] Root repository exists
- [ ] gain-core/ exists
- [ ] gain-standalone/ exists
- [ ] gain-ara/ exists
- [ ] docs/ exists
- [ ] test-assets/ exists

- [ ] gain-core has its own Cargo workspace
- [ ] gain-standalone has its own Cargo workspace
- [ ] gain-ara has its own build system
- [ ] No shared workspace combines all products

- [ ] Standalone is not a Cargo workspace member of gain-core
- [ ] ARA is not embedded into gain-core
- [ ] Core does not contain application code

# Section 2 — Public API Boundary

- [ ] gain-core/crates/gain-api exists
- [ ] Standalone imports gain-api
- [ ] ARA imports gain-api through FFI
- [ ] No application imports internal engine crates directly

# Section 3 — Gain Map Contract

- [ ] version: u32 exists
- [ ] Schema version constant exists
- [ ] Default schema version = 1
- [ ] Stable
- [ ] Transient
- [ ] EnvelopeControlled
- [ ] Mixed
- [ ] RegionType::Envelope does NOT exist

# Section 4 — FFI Contract

- [ ] gain_stage_map_version(...) exists in Rust export
- [ ] Present in C header
- [ ] Compiles successfully
- [ ] Allocation strategy documented
- [ ] Deallocation strategy documented
- [ ] No shared ownership
- [ ] No exceptions cross boundary
- [ ] No Rust panics cross boundary
- [ ] Opaque handle pattern documented

# Section 5 — Standalone Scaffold

- [ ] src-tauri/ exists
- [ ] Cargo.toml exists
- [ ] tauri.conf.json exists
- [ ] Frontend placeholder exists
- [ ] import_file()
- [ ] analyze()
- [ ] get_version()
- [ ] Standalone depends on gain-api
- [ ] Standalone does NOT depend on internal crates

# Section 6 — Documentation

- [ ] PRD.md
- [ ] Architecture.md
- [ ] GainDecisionModel.md
- [ ] StandaloneSpec.md
- [ ] ARASpec.md
- [ ] CodingStandards.md
- [ ] ArchitectureDecisionLog.md
- [ ] docs/Validation exists
- [ ] Phase1_Validation_Checklist.md exists

# Section 7 — ADRs

- [ ] ADR-001 Rust Core
- [ ] ADR-002 C ABI FFI
- [ ] ADR-003 Tauri Standalone
- [ ] ADR-004 ARA Integration
- [ ] ADR-005 Recommendation First
- [ ] ADR-006 Gain Map Schema Versioning
- [ ] ADR-007 Public Core API Boundary

# Section 8 — Design Drift Detection

- [ ] Standalone directly calls analysis crate
- [ ] ARA contains DSP logic
- [ ] Gain Map lacks version field
- [ ] RegionType::Envelope still exists
- [ ] Multiple public APIs exist
- [ ] Applications bypass gain-api

# Exit Criteria

- [ ] Workspace architecture validated
- [ ] Public API boundary validated
- [ ] Gain Map schema validated
- [ ] FFI contract validated
- [ ] Tauri scaffold validated
- [ ] Documentation validated
- [ ] ADRs validated
- [ ] No architecture drift detected
