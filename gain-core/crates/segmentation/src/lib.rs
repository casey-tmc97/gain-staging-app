const SILENCE_THRESHOLD_DBFS: f32 = -60.0;
/// Minimum silent run (in frames) to be kept as a separate segment.
/// At 10 ms/frame: 25 frames = 250 ms.
const MIN_SILENCE_FRAMES: usize = 25;

#[derive(Debug, Clone)]
pub struct Segment {
    pub start_sample: usize,
    pub end_sample: usize,
}

pub fn segment(samples: &[f32], sample_rate: u32) -> Vec<Segment> {
    if samples.is_empty() {
        return vec![];
    }

    // 10 ms frame size; clamp to at least 1 sample.
    let frame_size = (sample_rate as usize / 100).max(1);

    // Classify each 10 ms frame as silent or active.
    let silence_flags: Vec<bool> = samples
        .chunks(frame_size)
        .map(|frame| {
            let sum_sq: f32 = frame.iter().map(|s| s * s).sum();
            let rms = (sum_sq / frame.len() as f32).sqrt();
            let rms_db = if rms == 0.0 { -120.0f32 } else { 20.0 * rms.log10() };
            rms_db < SILENCE_THRESHOLD_DBFS
        })
        .collect();

    // Build runs of consecutive same-label frames.
    let mut runs: Vec<(bool, usize, usize)> = vec![]; // (is_silent, start_frame, end_frame)
    let mut i = 0;
    while i < silence_flags.len() {
        let label = silence_flags[i];
        let start = i;
        while i < silence_flags.len() && silence_flags[i] == label {
            i += 1;
        }
        runs.push((label, start, i));
    }

    // Merge silent runs shorter than MIN_SILENCE_FRAMES into adjacent active material.
    let mut merged: Vec<(bool, usize, usize)> = vec![];
    for run in runs {
        let (is_silent, start, end) = run;
        let too_short = is_silent && (end - start) < MIN_SILENCE_FRAMES;
        if too_short {
            if let Some(prev) = merged.last_mut() {
                prev.2 = end; // extend previous run to absorb this short gap
                continue;
            }
        }
        merged.push((is_silent, start, end));
    }

    // Merge any now-adjacent active runs created by the above step.
    let mut coalesced: Vec<(bool, usize, usize)> = vec![];
    for run in merged {
        if let Some(prev) = coalesced.last_mut() {
            if prev.0 == run.0 {
                prev.2 = run.2;
                continue;
            }
        }
        coalesced.push(run);
    }

    // Convert frame indices to sample indices.
    let total = samples.len();
    coalesced
        .into_iter()
        .map(|(_, start_frame, end_frame)| Segment {
            start_sample: start_frame * frame_size,
            end_sample: (end_frame * frame_size).min(total),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn silence(n: usize) -> Vec<f32> { vec![0.0f32; n] }
    fn tone(n: usize) -> Vec<f32>    { vec![0.5f32; n] }

    #[test]
    fn empty_input_returns_empty() {
        assert!(segment(&[], 44100).is_empty());
    }

    #[test]
    fn pure_silence_returns_single_silent_segment() {
        let segs = segment(&silence(44100), 44100);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].start_sample, 0);
        assert_eq!(segs[0].end_sample, 44100);
    }

    #[test]
    fn pure_tone_returns_single_active_segment() {
        let segs = segment(&tone(44100), 44100);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].start_sample, 0);
        assert_eq!(segs[0].end_sample, 44100);
    }

    #[test]
    fn tone_silence_tone_returns_three_segments() {
        let mut samples = tone(22050);
        samples.extend(silence(22050));
        samples.extend(tone(22050));
        let segs = segment(&samples, 44100);
        assert_eq!(segs.len(), 3, "expected tone|silence|tone, got {} segments", segs.len());
        assert_eq!(segs[1].start_sample, segs[0].end_sample);
        assert_eq!(segs[2].start_sample, segs[1].end_sample);
        assert_eq!(segs[2].end_sample, samples.len());
    }

    #[test]
    fn short_silence_under_250ms_is_merged() {
        let mut samples = tone(22050);
        samples.extend(silence(4410)); // 100 ms
        samples.extend(tone(22050));
        let segs = segment(&samples, 44100);
        assert_eq!(segs.len(), 1, "short gap should be merged into single segment");
    }

    #[test]
    fn segments_cover_full_sample_range() {
        let mut samples = silence(11025);
        samples.extend(tone(22050));
        samples.extend(silence(11025));
        let segs = segment(&samples, 44100);
        assert_eq!(segs[0].start_sample, 0);
        assert_eq!(segs.last().unwrap().end_sample, samples.len());
    }
}
