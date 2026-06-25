use gain_error::GainError;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct AudioBuffer {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone)]
pub enum ContainerFormat {
    Wav,
    Aiff,
}

pub struct AudioMetadata {
    pub duration_secs: f64,
    pub sample_rate: u32,
    pub channels: u16,
    pub format: ContainerFormat,
}

pub fn load_file(path: &std::path::Path) -> Result<(AudioBuffer, AudioMetadata), GainError> {
    if !path.exists() {
        return Err(GainError::FileNotFound { path: path.to_string_lossy().into_owned() });
    }

    // Detect container format: first try extension, then sniff magic bytes.
    let ext = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase());
    let container = match ext.as_deref() {
        Some("wav")                => ContainerFormat::Wav,
        Some("aif") | Some("aiff") => ContainerFormat::Aiff,
        _ => {
            // No recognized extension — sniff the first 4 bytes for RIFF / FORM magic.
            let mut magic = [0u8; 4];
            let mut peek = std::fs::File::open(path)
                .map_err(|e| GainError::DecodeFailure { details: e.to_string() })?;
            use std::io::Read;
            peek.read_exact(&mut magic)
                .map_err(|_| GainError::UnsupportedFormat {
                    format: ext.as_deref().unwrap_or("(no extension)").to_string(),
                })?;
            match &magic {
                b"RIFF"             => ContainerFormat::Wav,
                b"FORM"             => ContainerFormat::Aiff,
                _ => return Err(GainError::UnsupportedFormat {
                    format: ext.as_deref().unwrap_or("(no extension)").to_string(),
                }),
            }
        }
    };

    let file = std::fs::File::open(path)
        .map_err(|e| GainError::DecodeFailure { details: e.to_string() })?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext_str) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext_str);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| GainError::UnsupportedFormat { format: e.to_string() })?;

    let mut format = probed.format;
    let track = format.default_track()
        .ok_or_else(|| GainError::DecodeFailure { details: "no default track".to_string() })?;

    let track_id     = track.id;
    let sample_rate  = track.codec_params.sample_rate
        .ok_or_else(|| GainError::DecodeFailure { details: "missing sample rate".to_string() })?;
    let channels     = track.codec_params.channels
        .ok_or_else(|| GainError::DecodeFailure { details: "missing channel info".to_string() })?;
    let channel_count = channels.count() as u16;
    let n_frames     = track.codec_params.n_frames;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| GainError::DecodeFailure { details: e.to_string() })?;

    let mut all_samples: Vec<f32> = Vec::new();
    if let Some(frames) = n_frames {
        all_samples.reserve(frames as usize * channel_count as usize);
    }

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(GainError::DecodeFailure { details: e.to_string() }),
        };

        if packet.track_id() != track_id { continue; }

        let decoded = decoder.decode(&packet)
            .map_err(|e| GainError::DecodeFailure { details: e.to_string() })?;

        let spec = *decoded.spec();
        let mut sample_buf = SampleBuffer::<f32>::new(decoded.frames() as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);
        all_samples.extend_from_slice(sample_buf.samples());
    }

    let total_frames = all_samples.len() as f64 / channel_count as f64;
    let duration_secs = total_frames / sample_rate as f64;

    let buf  = AudioBuffer   { samples: all_samples, sample_rate, channels: channel_count };
    let meta = AudioMetadata { duration_secs, sample_rate, channels: channel_count, format: container };
    Ok((buf, meta))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_wav(samples_i16: &[i16], sample_rate: u32) -> Vec<u8> {
        let data_len = samples_i16.len() * 2;
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36u32 + data_len as u32).to_le_bytes());
        b.extend_from_slice(b"WAVE");
        b.extend_from_slice(b"fmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());       // PCM
        b.extend_from_slice(&1u16.to_le_bytes());       // mono
        b.extend_from_slice(&sample_rate.to_le_bytes());
        b.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());       // block align
        b.extend_from_slice(&16u16.to_le_bytes());      // bits per sample
        b.extend_from_slice(b"data");
        b.extend_from_slice(&(data_len as u32).to_le_bytes());
        for s in samples_i16 { b.extend_from_slice(&s.to_le_bytes()); }
        b
    }

    fn make_aiff(samples_i16: &[i16], sample_rate: u32) -> Vec<u8> {
        let n_frames = samples_i16.len() as u32;
        let data_len = samples_i16.len() * 2;
        let ssnd_size = 8u32 + data_len as u32;
        let form_size = 4 + 26 + 8 + ssnd_size;
        let sr_bytes: [u8; 10] = match sample_rate {
            44100 => [0x40, 0x0E, 0xAC, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            48000 => [0x40, 0x0E, 0xBB, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            _ => panic!("unsupported sample rate in test helper"),
        };
        let mut b = Vec::new();
        b.extend_from_slice(b"FORM");
        b.extend_from_slice(&form_size.to_be_bytes());
        b.extend_from_slice(b"AIFF");
        b.extend_from_slice(b"COMM");
        b.extend_from_slice(&18u32.to_be_bytes());
        b.extend_from_slice(&1u16.to_be_bytes());       // numChannels
        b.extend_from_slice(&n_frames.to_be_bytes());
        b.extend_from_slice(&16u16.to_be_bytes());      // sampleSize
        b.extend_from_slice(&sr_bytes);
        b.extend_from_slice(b"SSND");
        b.extend_from_slice(&ssnd_size.to_be_bytes());
        b.extend_from_slice(&0u32.to_be_bytes());       // offset
        b.extend_from_slice(&0u32.to_be_bytes());       // blockSize
        for s in samples_i16 { b.extend_from_slice(&s.to_be_bytes()); }
        b
    }

    #[test]
    fn load_wav_returns_samples_and_metadata() {
        let samples_i16: Vec<i16> = vec![16383; 100]; // ≈ 0.5 amplitude
        let bytes = make_wav(&samples_i16, 44100);
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(&bytes).unwrap();
        let (buf, meta) = load_file(f.path()).unwrap();
        assert_eq!(meta.sample_rate, 44100);
        assert_eq!(meta.channels, 1);
        assert_eq!(buf.samples.len(), 100);
        for s in &buf.samples {
            assert!((s - 0.5).abs() < 0.002, "sample {s} not near 0.5");
        }
    }

    #[test]
    fn load_aiff_returns_samples_and_metadata() {
        let samples_i16: Vec<i16> = vec![16383; 50];
        let bytes = make_aiff(&samples_i16, 44100);
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(&bytes).unwrap();
        let (buf, meta) = load_file(f.path()).unwrap();
        assert_eq!(meta.sample_rate, 44100);
        assert_eq!(buf.samples.len(), 50);
        for s in &buf.samples {
            assert!((s - 0.5).abs() < 0.002, "sample {s} not near 0.5");
        }
    }

    #[test]
    fn load_missing_file_returns_file_not_found() {
        let result = load_file(std::path::Path::new("/no/such/file.wav"));
        assert!(matches!(result, Err(gain_error::GainError::FileNotFound { .. })));
    }

    #[test]
    fn load_unsupported_extension_returns_error() {
        let mut f = tempfile::NamedTempFile::with_suffix(".mp3").unwrap();
        f.write_all(b"not audio").unwrap();
        let result = load_file(f.path());
        assert!(matches!(result, Err(gain_error::GainError::UnsupportedFormat { .. })));
    }
}
