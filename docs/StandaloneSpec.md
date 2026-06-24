# Standalone App Specification
## Gain Stage App — `gain-standalone`

**Version:** 0.1 (draft)
**Status:** In progress

---

## Overview

The standalone app is a Tauri desktop application that wraps the `gain-core`
engine for offline, file-based analysis workflows. It exposes no analysis
logic of its own — all computation is delegated to `gain-api`.

## File Import Flow

_[Describe how users import audio files: drag-and-drop, file picker, batch
import. Supported formats. Error states.]_

## Analysis Flow

_[Describe the UX from file import to GRM display: progress indication,
cancellation, result presentation.]_

## Waveform Display

_[Describe the waveform visualization: zoom, scroll, time ruler, channel
display, overlay of GRM regions.]_

## Gain Map Visualization

_[Describe how GRM regions are shown: color coding per RegionType,
gain_db annotation, confidence indicator, reason tooltip.]_

## Export

_[Describe export formats: JSON GRM export, CSV, optional rendered audio
with gain applied (explicit user action only).]_

## Error Handling

_[Define error states and user-facing messages for: file not found,
unsupported format, analysis failure, out of memory.]_
