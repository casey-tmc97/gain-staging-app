# Product Requirements Document
## Gain Stage App

**Version:** 0.1 (draft)
**Status:** In progress

---

## Problem Statement

_[Describe the core problem: mixing engineers lack a reliable, non-destructive
tool for analyzing and staging gain across a session. Manual gain staging is
time-consuming and inconsistent.]_

## Target Users

_[Primary: mix engineers working in DAW environments (Pro Tools, Logic, Reaper).
Secondary: mastering engineers verifying pre-master levels.]_

## Goals

- Analyze audio and produce a time-segmented Gain Recommendation Map (GRM)
- Provide recommendations without modifying audio
- Integrate into DAW workflows via ARA and into offline workflows via a standalone app

## Non-Goals

- Automatic gain application without explicit user action
- Loudness normalization to delivery targets (LUFS delivery is out of scope)
- Real-time DSP processing

## Success Metrics

_[Define measurable outcomes: e.g., recommendation accuracy vs. manual engineer
judgment, time saved per session, user retention after trial.]_

## Product Layers

### Gain Core (`gain-core`)

_[Describe the Rust analysis engine: inputs, outputs, crate structure, public API surface via gain-api.]_

### Standalone App (`gain-standalone`)

_[Describe the Tauri desktop app: file import, waveform display, GRM visualization, export.]_

### ARA Plugin (`gain-ara`)

_[Describe the DAW plugin: host integration, timeline overlay, session management.]_

## Open Questions

_[List unresolved product decisions.]_
