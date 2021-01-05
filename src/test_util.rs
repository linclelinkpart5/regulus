#![cfg(test)]

use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use byteorder::{ByteOrder, LittleEndian};
use claxon::FlacReader;
// use hound::WavReader;
use itertools::Itertools;

use crate::MAX_CHANNELS;

pub(crate) enum WaveKind {
    Sine,
    Square,
    Triangle,
    Sawtooth,
}

impl WaveKind {
    const fn name(&self) -> &'static str {
        match self {
            Self::Sine => "sine",
            Self::Square => "square",
            Self::Triangle => "triangle",
            Self::Sawtooth => "sawtooth",
        }
    }
}

pub(crate) struct TestUtil;

impl TestUtil {
    pub fn check_sox() -> bool {
        Command::new("sox").arg("--version")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub fn load_flac_data(path: &Path) -> (Vec<[f64; MAX_CHANNELS]>, u32, u32) {
        let file = match File::open(path) {
            Err(e) => panic!("could not open file: {}", e),
            Ok(f) => f,
        };

        let mut reader = match FlacReader::new(file) {
            Err(e) => panic!("could not read FLAC data: {}", e),
            Ok(r) => r,
        };

        // Get stream info.
        let info = reader.streaminfo();
        let sample_rate = info.sample_rate;
        let num_channels = info.channels;
        let bits_per_sample = info.bits_per_sample;

        assert!(
            num_channels as usize <= MAX_CHANNELS,
            "too many channels (max {}): {}", MAX_CHANNELS, num_channels,
        );

        // Since the samples are signed integers (one of 16/24/32-bit), need to
        // normalize them to the range [-1.0, 1.0).
        let a = match bits_per_sample {
            0 => 0u32,
            b => {
                let shift = b - 1;
                match 1u32.checked_shl(shift) {
                    None => panic!("too many bits per sample (max 32): {}", b),
                    Some(a) => a,
                }
            },
        };
        let amplitude = a as f64;

        let samples = reader.samples()
            .map(|res| {
                match res {
                    Err(e) => panic!("error while reading FLAC data: {}", e),
                    Ok(s) => s,
                }
            })
            .batching(|it| {
                let mut s = [0.0f64; MAX_CHANNELS];

                for i in 0..num_channels {
                    let x = match it.next() {
                        Some(x) => x,
                        None if i == 0 => return None,
                        None => panic!("incomplete frame at end of stream"),
                    };

                    s[i as usize] = x as f64 / amplitude;
                }

                Some(s)
            })
            .collect::<Vec<_>>()
        ;

        (samples, sample_rate, num_channels)
    }

    pub fn load_custom_audio_paths(dir_path: &Path) -> Vec<PathBuf> {
        let read_dir = match std::fs::read_dir(dir_path) {
            Ok(rd) => rd,
            Err(e) => panic!("could not read dir: {}", e),
        };

        let mut entries = read_dir
            .filter_map(|res| {
                let dir_entry = match res {
                    Ok(de) => de,
                    Err(e) => panic!("could not read dir entry: {}", e),
                };

                let path = dir_entry.path();

                match path.extension() {
                    Some(ext) => {
                        if ext == "flac" || ext == "wav" {
                            Some(path)
                        } else {
                            None
                        }
                    },
                    None => None,
                }
            })
            .collect::<Vec<_>>()
        ;

        entries.sort_by(|ea, eb| ea.file_name().cmp(&eb.file_name()));

        entries
    }

    pub fn sox_eval(cmd: &mut Command) -> Vec<u8> {
        let output = match cmd.output() {
            Err(e) => panic!("failed to execute command: {}", e),
            Ok(o) => o,
        };

        let Output { status, stdout, stderr } = output;

        assert!(status.success(), "command returned with non-zero code: {}", status);
        assert!(
            stderr.len() == 0,
            "non-empty stderr from running sox as subprocess: {}",
            std::str::from_utf8(&stderr).unwrap(),
        );

        stdout
    }

    pub fn sox_eval_string(cmd: &mut Command) -> String {
        let raw_stdout = Self::sox_eval(cmd);

        match String::from_utf8(raw_stdout) {
            Err(e) => panic!("cannot convert stdout bytes into string: {}", e),
            Ok(s) => s,
        }
    }

    pub fn sox_eval_samples(cmd: &mut Command) -> Vec<f64> {
        let raw_stdout = Self::sox_eval(cmd);

        let mut res = vec![0.0f64; raw_stdout.len() / std::mem::size_of::<f64>()];
        LittleEndian::read_f64_into(&raw_stdout, &mut res);

        res
    }

    pub fn sox_gen_wave_cmd(sample_rate: u32, kind: &WaveKind, frequency: u32) -> Command {
        let mut cmd = Command::new("sox");

        cmd
            // No input file name.
            .arg("--null")

            // Set sample rate.
            .arg("--rate").arg(sample_rate.to_string())

            // Set output data format params.
            .arg("--endian").arg("little")
            .arg("--channels").arg("1")
            .arg("--type").arg("f64")

            // Output to stdout.
            .arg("-")

            // Wave to generate/synthesize.
            .arg("synth").arg("3").arg(kind.name()).arg(frequency.to_string())

            // Insert some headroom to prevent clipping.
            .arg("gain").arg("-2")
        ;

        cmd
    }

    fn sox_gen_wave(sample_rate: u32, kind: &WaveKind, frequency: u32) -> Vec<f64> {
        Self::sox_eval_samples(&mut Self::sox_gen_wave_cmd(sample_rate, kind, frequency))
    }

    pub fn load_audio_data(path: &Path) -> (Vec<f64>, u32, u8) {
        // Get sample rate.
        let stdout_str = Self::sox_eval_string(
            Command::new("soxi").arg("-r").arg(path)
        );
        let sample_rate = str::parse::<u32>(&stdout_str).unwrap();

        // Get num channels.
        let stdout_str = Self::sox_eval_string(
            Command::new("soxi").arg("-c").arg(path)
        );
        let num_channels = str::parse::<u8>(&stdout_str).unwrap();

        // Read the audio data.
        let flat_samples = Self::sox_eval_samples(
            Command::new("sox")
                .arg(path)

                // Set output data format params.
                .arg("--endian").arg("little")
                .arg("--type").arg("f64")

                // Output to stdout.
                .arg("-")
        );

        (flat_samples, sample_rate, num_channels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::File;

    use tempfile::{TempDir, Builder};

    fn create_dir_with_files<I, S>(files: I) -> TempDir
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let temp_dir = Builder::new().tempdir().unwrap();

        let path = temp_dir.path();

        for file_name in files {
            File::create(path.join(file_name.as_ref())).unwrap();
        }

        temp_dir
    }

    #[test]
    fn load_custom_audio_paths() {
        let temp_dir = create_dir_with_files(&[
            "03.flac", "02.txt", "01.wav", "04"
        ]);
        let temp_dir_path = temp_dir.path();

        assert_eq!(
            TestUtil::load_custom_audio_paths(&temp_dir_path),
            vec![
                temp_dir_path.join("01.wav"),
                temp_dir_path.join("03.flac"),
            ]
        );
    }
}
