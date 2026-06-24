You are building a professional audio software system called **Gain Stage App**.

This is NOT a plugin-first project.

This is a **perceptual audio analysis engine** that generates gain staging recommendations.

---

# 1. SYSTEM OVERVIEW

Gain Stage App analyzes audio and produces a:

> Gain Recommendation Map (GRM)

It does NOT modify audio unless explicitly instructed outside the core engine.

---

# 2. ARCHITECTURE (MANDATORY)

You MUST follow this architecture exactly:

## Core Engine (Rust)
- All audio analysis
- segmentation
- envelope detection
- transient detection
- gain decision engine
- Gain Map generation

NO UI logic
NO DAW logic
NO plugin logic

---

## Standalone App (Tauri + Rust)
- file import/export
- waveform display
- batch processing
- visualization of Gain Map
- user review + export

Uses Core Engine ONLY through Rust library

---

## ARA Plugin (C++)
- DAW integration layer
- receives audio from host
- sends audio to Rust Core via C ABI
- receives Gain Map
- displays results in DAW timeline

NO DSP logic

---

## FFI Layer (C ABI)
- Rust ↔ C++ bridge
- minimal surface area
- no object sharing across boundary
- no exceptions across boundary

---

# 3. CORE PRINCIPLE

The system is:

> Recommendation-first, non-destructive by default

It must never:
- normalize audio automatically
- apply gain silently
- alter audio without explicit action

---

# 4. CORE OUTPUT FORMAT

The only important product of the system is:

## Gain Recommendation Map

Example structure:

- region start/end
- recommended gain (dB)
- confidence score
- region type
- reasoning tag

---

# 5. RUST CORE REQUIREMENTS

Create a Rust workspace with:

## Modules:

- audio_ingestion
- analysis
  - loudness
  - spectral
  - transient
  - envelope
- segmentation
- classification
- gain_decision
- gain_map
- ffi

---

## Rules:

- no unsafe unless justified
- no global mutable state
- deterministic outputs required
- all processing is batch-oriented (NOT real-time DSP)

---

# 6. ARA C++ LAYER REQUIREMENTS

Create a C++ project with:

- ARA SDK integration layer
- bridge to Rust via C ABI
- session management
- gain map display hooks

Rules:
- no DSP
- no analysis
- no audio processing logic
- only transport + UI + host integration

---

# 7. DATA MODEL (MANDATORY)

Define a shared Gain Map schema:

- regions:
  - start_time
  - end_time
  - gain_db
  - confidence
  - type (stable/transient/envelope/mixed)
  - reason

---

# 8. OUTPUT OF THIS TASK

Generate:

## Repository structure:
- full folder tree
- Rust workspace scaffold
- C++ plugin scaffold
- FFI interface files
- shared docs folder
- placeholder implementations

## Must include:
- build system setup (Cargo + CMake or equivalent)
- basic compileable skeleton
- no pseudo-code pretending to be functional DSP

---

# 9. HARD CONSTRAINTS

- DO NOT implement full DSP algorithms yet
- DO NOT build UI logic beyond placeholders
- DO NOT mix responsibilities between layers
- DO NOT skip FFI boundary design

---

# 10. SUCCESS CRITERIA

The output must:

- compile (even if minimal functionality)
- clearly separate Rust core vs C++ plugin
- expose a working FFI stub
- include Gain Map struct definitions
- reflect architecture exactly

---

# 11. CHECKPOINT — RETURN TO THIS CONVERSATION

When this phase is complete:

- Full repo scaffold is generated
- Rust core builds successfully
- C++ ARA layer compiles (even if minimal)
- FFI boundary is functional (stub-level is acceptable)
- Gain Map data structures exist in both layers

👉 STOP further development work.

👉 Do NOT proceed into DSP algorithm implementation yet.

👉 Return to the original planning conversation with the user and report completion of:
   - repository scaffold
   - architecture implementation
   - build system setup

Then wait for next instructions before continuing.

---

BEGIN GENERATION NOW
