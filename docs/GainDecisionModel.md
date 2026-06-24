# Gain Decision Model
## Gain Stage App

**Version:** 0.1 (draft)
**Status:** In progress

---

## Overview

_[Describe the overall approach: analysis pipeline feeds a decision engine that
produces per-region gain recommendations with confidence scores.]_

## Region Classification Rules

_[Define the rules for classifying a segment as Stable, Transient,
EnvelopeControlled, or Mixed. Include signal characteristics that distinguish
each type (e.g., attack time, RMS variance, spectral flatness).]_

## Gain Recommendation Formula

_[Define how gain_db is computed for each region type. Reference target
levels (e.g., peak, RMS) and describe the formula or lookup logic.]_

## Confidence Scoring

_[Describe how the 0.0–1.0 confidence score is derived. What makes a
recommendation high-confidence vs. uncertain?]_

## Edge Cases

_[Document known difficult cases: silence, full-scale clipping, very short
transients, DC offset, etc., and how the model handles them.]_

## References

_[Cite relevant standards and literature: ITU-R BS.1770, AES papers on
loudness, transient detection algorithms, etc.]_
