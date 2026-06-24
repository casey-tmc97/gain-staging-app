cmake_minimum_required(VERSION 3.20)
project(gain_stage_ara VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# ── Paths ──────────────────────────────────────────────────────────────────────

# Path to the Rust workspace root (relative to this CMakeLists.txt)
set(RUST_CORE_DIR "${CMAKE_SOURCE_DIR}/../gain-core")

# The Rust FFI header
set(FFI_INCLUDE_DIR "${RUST_CORE_DIR}/crates/ffi/include")

# The Rust import library (Windows: .dll.lib)
# Build the Rust FFI crate first: cd gain-core && cargo build -p ffi --release
set(RUST_LIB "${RUST_CORE_DIR}/target/release/gain_stage_ffi.dll.lib")

# ── Verify prerequisites ───────────────────────────────────────────────────────

if(NOT EXISTS "${FFI_INCLUDE_DIR}/gain_stage_ffi.h")
    message(FATAL_ERROR
        "FFI header not found at ${FFI_INCLUDE_DIR}/gain_stage_ffi.h\n"
        "Ensure gain-core/crates/ffi/include/gain_stage_ffi.h exists.")
endif()

if(NOT EXISTS "${RUST_LIB}")
    message(FATAL_ERROR
        "Rust import library not found at ${RUST_LIB}\n"
        "Run: cd gain-core && cargo build -p ffi --release")
endif()

# ── Copy header to local include dir ─────────────────────────────────────────

set(LOCAL_FFI_INCLUDE "${CMAKE_SOURCE_DIR}/gain-core-include")
file(MAKE_DIRECTORY "${LOCAL_FFI_INCLUDE}")
configure_file(
    "${FFI_INCLUDE_DIR}/gain_stage_ffi.h"
    "${LOCAL_FFI_INCLUDE}/gain_stage_ffi.h"
    COPYONLY
)

# ── Target ────────────────────────────────────────────────────────────────────

add_library(gain_stage_ara SHARED
    src/GainMapBridge.cpp
    src/ARAPlugin.cpp
)

target_include_directories(gain_stage_ara PRIVATE
    "${LOCAL_FFI_INCLUDE}"
    stubs/
    src/
)

target_link_libraries(gain_stage_ara PRIVATE "${RUST_LIB}")

# On Windows: copy the Rust DLL next to the built plugin DLL
if(WIN32)
    set(RUST_DLL "${RUST_CORE_DIR}/target/release/gain_stage_ffi.dll")
    add_custom_command(TARGET gain_stage_ara POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
            "${RUST_DLL}"
            "$<TARGET_FILE_DIR:gain_stage_ara>/gain_stage_ffi.dll"
        COMMENT "Copying Rust FFI DLL alongside plugin"
    )
endif()
