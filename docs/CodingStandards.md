# Coding Standards
## Gain Stage App

**Version:** 0.1
**Status:** Active

---

## Rust Standards

- Edition: 2021 in all crates
- `unsafe` blocks require a `// SAFETY:` comment explaining the invariant
- No `static mut` — use function-local state or `std::sync` primitives
- All public items in library crates must have doc comments
- Tests live in `#[cfg(test)] mod tests` in the same file as the code
- Use `#[derive(Debug)]` on all public structs and enums

## C++ Standards

- Standard: C++17 minimum
- All functions on the FFI bridge layer must be `noexcept`
- No DSP, analysis, or gain decision logic in `gain-ara`
- RAII for all resources; no raw `new`/`delete` outside of bridge patterns
- No exceptions may propagate across the C ABI boundary

## FFI Rules

- Only POD types and opaque handles cross the FFI boundary
- No Rust objects are shared directly with C++ callers
- Every allocation made in Rust (`Box::into_raw`) must have a corresponding
  free function callable from C++ (`Box::from_raw`)
- All `extern "C"` functions must be `#[no_mangle]`
- The C header (`gain_stage_ffi.h`) is the authoritative FFI contract;
  Rust and C++ implementations must match it exactly

## Public API Boundary (ADR-005)

- `gain-standalone`, `gain-ara`, and any future SDK may ONLY import `gain-api`
- Direct imports of `segmentation`, `analysis`, `classification`,
  `gain_decision`, or `gain_map` from outside `gain-core` are forbidden
- The `ffi` crate is considered an internal crate within `gain-core` and
  must also import via `gain-api`

## Test Requirements

- New logic requires tests before implementation (TDD)
- Tests must assert specific values, not just "no panic"
- Integration tests that cross the FFI boundary must be written in Rust
  (use the `ffi` crate's own test module)
- Do not mock the `gain-api` interface in standalone tests — use the real stub

## Commit Convention

```
<type>: <short description>

Types: feat | fix | refactor | chore | docs | test
```

## Forbidden Patterns

- `unwrap()` in production code (use `?` or explicit error handling)
- `println!` in library crates (use proper logging when added)
- `#[allow(dead_code)]` without a comment explaining why
- Hardcoded absolute file paths
