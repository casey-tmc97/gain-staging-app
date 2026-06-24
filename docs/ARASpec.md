# ARA Plugin Specification
## Gain Stage App — `gain-ara`

**Version:** 0.1 (draft)
**Status:** In progress

---

## Overview

The ARA plugin integrates the Gain Stage analysis engine into DAW hosts
that support the ARA 2 protocol. It contains no DSP or analysis logic —
all computation is delegated to the `gain-core` FFI (`gain_stage_ffi`).

## Host Integration Lifecycle

_[Describe the ARA session lifecycle: plugin instantiation, document
controller creation, audio source binding, analysis trigger, teardown.]_

## Audio Source Events

_[Describe which ARA host callbacks trigger analysis: onAudioSourceContentChanged,
onPlaybackRegionChanged, etc. Describe how audio data is passed to the FFI.]_

## Gain Map Display in DAW Timeline

_[Describe how GRM regions are rendered in the DAW timeline: overlay color,
gain annotation, region boundary markers, tooltip on hover.]_

## Session Management

_[Describe how GRM results are persisted across DAW sessions: serialization
format, where data is stored, versioning.]_

## Error Handling

_[Define behavior when analysis fails: silent fallback, user notification
via DAW UI, logging.]_
