#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Interpret raw bytes as f32 samples; discard odd-length tails.
    let n_floats = data.len() / 4;
    if n_floats == 0 { return; }

    let samples: Vec<f32> = data[..n_floats * 4]
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();

    // Must not panic regardless of input; Err is fine.
    let _ = gain_api::analyze_pcm(&samples, 44100, 1, samples.len() as f64 / 44100.0);
});
