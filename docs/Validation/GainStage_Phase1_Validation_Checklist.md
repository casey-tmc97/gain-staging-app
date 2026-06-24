# Phase 1 Validation Checklist
## Gain Stage App — Repository Scaffold Validation

**Version:** 1.0
**Phase:** Core Architecture & Repository Foundation

---

# Pass / Fail Criteria

A phase is only considered complete when all critical items pass.

---

# Section 1 — Repository Structure

## Critical

### Repository exists as a monorepo

- [ ] Single root repository
- [ ] Rust Core project present
- [ ] Standalone project present
- [ ] ARA project present
- [ ] Documentation folder present

### Required directories exist

- [ ] /gain-core
- [ ] /gain-standalone
- [ ] /gain-ara
- [ ] /docs
- [ ] /test-assets

### Build files exist

- [ ] Root README
- [ ] Cargo workspace file
- [ ] CMake configuration
- [ ] Build instructions

---

# Section 2 — Architecture Validation

## Critical

### Rust Core contains

- [ ] Audio ingestion module
- [ ] Analysis module
- [ ] Segmentation module
- [ ] Classification module
- [ ] Gain Decision module
- [ ] Gain Map module
- [ ] FFI module

### Rust Core does NOT contain

- [ ] DAW integration code
- [ ] Plugin code
- [ ] GUI code
- [ ] ARA SDK references

### ARA project contains

- [ ] Host integration layer
- [ ] FFI bridge layer
- [ ] Session management

### ARA project does NOT contain

- [ ] Loudness analysis
- [ ] DSP logic
- [ ] Gain decision logic
- [ ] Segmentation code

### Standalone contains

- [ ] Application shell
- [ ] File import framework
- [ ] Core integration hooks

### Standalone does NOT contain

- [ ] Duplicate analysis code
- [ ] Duplicate Gain Map logic

---

# Section 3 — Gain Map Contract Validation

## Critical

### Gain Map structure exists

- [ ] Version field
- [ ] Region list
- [ ] Gain recommendation field
- [ ] Confidence field
- [ ] Region classification field
- [ ] Reason field

### Region types exist

- [ ] Stable
- [ ] Transient
- [ ] Envelope Controlled
- [ ] Mixed

### Gain Map location

- [ ] Defined in Rust Core
- [ ] Accessible via FFI
- [ ] Not duplicated independently

---

# Section 4 — FFI Validation

## Critical

### Boundary exists

- [ ] Rust exports symbols
- [ ] C++ imports symbols
- [ ] Build succeeds

### Ownership rules documented

- [ ] Allocation strategy documented
- [ ] Deallocation strategy documented

### Safety rules followed

- [ ] No exceptions cross boundary
- [ ] No Rust panics cross boundary
- [ ] No shared complex object ownership

---

# Section 5 — Build Validation

## Critical

### Rust

- [ ] Cargo build succeeds
- [ ] Unit test framework configured

### C++

- [ ] Plugin project compiles
- [ ] CMake build succeeds

### Integrated Build

- [ ] Rust library generated
- [ ] C++ links correctly
- [ ] Example FFI call succeeds

---

# Section 6 — Documentation Validation

## Required Documents

- [ ] PRD
- [ ] Architecture Document
- [ ] Core Specification
- [ ] Gain Decision Model
- [ ] FFI Contract
- [ ] Standalone Specification
- [ ] ARA Specification
- [ ] Coding Standards
- [ ] Architecture Decision Log

---

# Section 7 — Design Drift Detection

- [ ] No automatic gain application
- [ ] Core does not modify audio
- [ ] Core outputs recommendations
- [ ] All products consume Gain Map
- [ ] Envelope-aware processing referenced

---

# Section 8 — Explicitly Deferred Work

If present, mark FAIL.

- [ ] LUFS implementation
- [ ] Spectral analysis implementation
- [ ] Segmentation implementation
- [ ] Gain recommendation implementation
- [ ] Full waveform editor
- [ ] Production UI
- [ ] DAW visual integration
- [ ] Destructive rendering engine
- [ ] Export processing engine

---

# Phase 1 Exit Criteria

- [ ] Repository structure validated
- [ ] Build systems validated
- [ ] Rust Core scaffold validated
- [ ] C++ ARA scaffold validated
- [ ] FFI validated
- [ ] Documentation validated
- [ ] No architecture drift detected

## Next Authorized Phase

Proceed to Phase 2 — Core Engine Data Structures & Gain Map Implementation.

DO NOT begin DSP algorithm development until Phase 2 is formally approved.
