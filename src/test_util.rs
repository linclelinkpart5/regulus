#![cfg(test)]

use std::f64::consts::PI;
use std::fs::File;
use std::io::{Error as IoError, Read, Result as IoResult};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use byteorder::{ByteOrder, LittleEndian};
use claxon::{Error as ClaxonError, FlacReader, FlacIntoSamples, Result as ClaxonResult};
use claxon::input::BufferedReader;
use sampara::{Frame, Signal};
use sampara::signal::FromFrames as SignalFromFrames;
use hound::{Error as HoundError, WavReader, WavIntoSamples, SampleFormat, Result as HoundResult};
use serde::Deserialize;

use crate::filter::KWeightFilter;
use crate::gating::GatedPowers;
use crate::loudness::Loudness;

const MAX_CHANNELS: usize = 5;

#[derive(Debug)]
pub enum ReaderError {
    NoExt,
    BadExt,
    Io(IoError),
    Flac(ClaxonError),
    Wav(HoundError),
}

impl From<LoadFlacError> for ReaderError {
    fn from(err: LoadFlacError) -> Self {
        match err {
            LoadFlacError::Io(err) => Self::Io(err),
            LoadFlacError::Flac(err) => Self::Flac(err),
        }
    }
}

impl From<LoadWavError> for ReaderError {
    fn from(err: LoadWavError) -> Self {
        match err {
            LoadWavError::Io(err) => Self::Io(err),
            LoadWavError::Wav(err) => Self::Wav(err),
        }
    }
}

impl From<ClaxonError> for ReaderError {
    fn from(err: ClaxonError) -> Self {
        ReaderError::Flac(err)
    }
}

impl From<HoundError> for ReaderError {
    fn from(err: HoundError) -> Self {
        ReaderError::Wav(err)
    }
}

#[derive(Debug)]
pub enum LoadFlacError {
    Io(IoError),
    Flac(ClaxonError),
}

#[derive(Debug)]
pub enum LoadWavError {
    Io(IoError),
    Wav(HoundError),
}

fn amplitude(bits_per_sample: u32) -> f64 {
    // Since the samples are signed integers (one of 16/24/32-bit), need to
    // normalize them to the range [-1.0, 1.0).
    let a = match bits_per_sample {
        0 => panic!("bits per sample is 0"),
        b => {
            let shift = b - 1;
            1u32.checked_shl(shift)
                .unwrap_or_else(|| panic!("too many bits per sample (max 32): {}", b))
        },
    };

    a as f64
}

pub(crate) struct FlacFrames<R: Read> {
    samples: FlacIntoSamples<BufferedReader<R>>,
    pub num_channels: u32,
    pub sample_rate: u32,
    amplitude: f64,
}

impl<R: Read> FlacFrames<R> {
    pub fn new(reader: FlacReader<R>) -> Self {
        // Get stream info.
        let info = reader.streaminfo();
        let num_channels = info.channels;
        let bits_per_sample = info.bits_per_sample;
        let sample_rate = info.sample_rate;

        assert!(
            num_channels as usize <= MAX_CHANNELS,
            "too many channels (max {}): {}", MAX_CHANNELS, num_channels,
        );

        let amplitude = amplitude(bits_per_sample);

        let samples = reader.into_samples();

        Self {
            samples,
            num_channels,
            sample_rate,
            amplitude,
        }
    }
}

impl<R: Read> Iterator for FlacFrames<R> {
    type Item = ClaxonResult<[f64; MAX_CHANNELS]>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut frame = [0.0f64; MAX_CHANNELS];

        for (i, f) in frame.channels_mut().enumerate().take(self.num_channels as usize) {
            let raw_sample = match self.samples.next() {
                Some(Ok(x)) => x,
                Some(Err(e)) => return Some(Err(e)),
                None if i == 0 => return None,
                None => return Some(Err(ClaxonError::FormatError("incomplete frame at end of stream"))),
            };

            let normalized_sample = raw_sample as f64 / self.amplitude;

            *f = normalized_sample
        }

        Some(Ok(frame))
    }
}

enum WavNormedSamples<R: Read> {
    // Use i32, which will accommodate i8, i16, and i32.
    // Also include the normalization factor (aka "amplitude").
    Int(WavIntoSamples<R, i32>, f64),

    // Use f32, as it is the only supported float type.
    Float(WavIntoSamples<R, f32>),
}

impl<R: Read> Iterator for WavNormedSamples<R> {
    type Item = HoundResult<f64>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Int(i_samples, amp) => Some(i_samples.next()?.map(|i| i as f64 / *amp)),
            Self::Float(f_samples) => Some(f_samples.next()?.map(|f| f as f64)),
        }
    }
}

pub(crate) struct WavFrames<R: Read> {
    samples: WavNormedSamples<R>,
    pub num_channels: u32,
    pub sample_rate: u32,
}

impl<R: Read> WavFrames<R> {
    pub fn new(reader: WavReader<R>) -> Self {
        // Get stream info.
        let info = reader.spec();
        let num_channels = info.channels as u32;
        let bits_per_sample = info.bits_per_sample as u32;
        let sample_rate = info.sample_rate;

        assert!(
            num_channels as usize <= MAX_CHANNELS,
            "too many channels (max {}): {}", MAX_CHANNELS, num_channels,
        );

        let samples = match info.sample_format {
            SampleFormat::Int => WavNormedSamples::Int(
                reader.into_samples(),
                amplitude(bits_per_sample),
            ),
            SampleFormat::Float => WavNormedSamples::Float(reader.into_samples()),
        };

        Self {
            samples,
            num_channels,
            sample_rate,
        }
    }
}

impl<R: Read> Iterator for WavFrames<R> {
    type Item = HoundResult<[f64; MAX_CHANNELS]>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut frame = [0.0f64; MAX_CHANNELS];

        for (i, f) in frame.channels_mut().enumerate().take(self.num_channels as usize) {
            let normed_sample = match self.samples.next() {
                Some(Ok(x)) => x,
                Some(Err(e)) => return Some(Err(e)),
                None if i == 0 => return None,
                None => return Some(Err(HoundError::FormatError("incomplete frame at end of stream"))),
            };

            *f = normed_sample
        }

        Some(Ok(frame))
    }
}

pub(crate) enum TestReader<R: Read> {
    Flac(FlacFrames<R>),
    Wav(WavFrames<R>),
}

impl<R: Read> TestReader<R> {
    pub fn into_signal(self) -> impl Signal<MAX_CHANNELS, Frame = [f64; MAX_CHANNELS]> {
        sampara::signal::from_frames(self.map(Result::unwrap))
    }

    pub fn num_channels(&self) -> u32 {
        match self {
            Self::Flac(s) => s.num_channels,
            Self::Wav(s) => s.num_channels,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        match self {
            Self::Flac(s) => s.sample_rate,
            Self::Wav(s) => s.sample_rate,
        }
    }

    pub fn process_frames(self) {
        let sample_rate = self.sample_rate();

        let signal = self.into_signal();

        let k_weighter = KWeightFilter::new(sample_rate);
        let power_gater = GatedPowers::new(sample_rate);

        let filtered_signal = signal.process(k_weighter);
        let gated_signal = filtered_signal.blocking_process(power_gater);

        let mut loudness_calc = Loudness::new([1.0, 1.0, 1.0, 1.41, 1.41]);

        for frame in gated_signal.into_iter() {
            loudness_calc.push(frame);
        }

        let loudness = loudness_calc.calculate().unwrap();

        println!("Loudness: {}", loudness)
    }
}

impl TestReader<File> {
    pub fn read_path(path: &Path) -> Result<Self, ReaderError> {
        let ext = path
            .extension()
            .ok_or(ReaderError::NoExt)?;

        if ext == "flac" {
            Ok(Self::Flac(TestUtil::load_flac_data(path)?))
        } else if ext == "wav" {
            Ok(Self::Wav(TestUtil::load_wav_data(path)?))
        } else {
            Err(ReaderError::BadExt)
        }
    }
}

impl<R: Read> Iterator for TestReader<R> {
    type Item = Result<[f64; MAX_CHANNELS], ReaderError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Flac(fs) => fs.next().map(|r| r.map_err(Into::into)),
            Self::Wav(fs) => fs.next().map(|r| r.map_err(Into::into)),
        }
    }
}

#[derive(Deserialize)]
pub(crate) struct Analysis {
    momentary_mean: f64,
    momentary_maximum: f64,
    momentary_range: f64,
    shortterm_mean: f64,
    shortterm_maximum: f64,
    shortterm_range: f64,
}

#[derive(Deserialize)]
pub(crate) struct AlbumTestcase {
    name: String,
    #[serde(flatten)]
    album: Analysis,
    tracks: Vec<Analysis>,
}

pub(crate) struct TestUtil;

impl TestUtil {
    pub fn collect_album_dirs(root_dir: &Path) -> Vec<PathBuf> {
        let read_dir = std::fs::read_dir(root_dir).expect("cannot read root dir");

        let mut album_dir_paths = read_dir.filter_map(|res| {
            let dir_entry = res.expect("cannot read subentry in root dir");

            let metadata = dir_entry.metadata().expect("cannot read subentry metadata");

            // Only keep directories.
            if metadata.is_dir() {
                Some(dir_entry.path())
            }
            else {
                None
            }
        })
        .collect::<Vec<_>>();

        album_dir_paths.sort_by(|ea, eb| ea.file_name().cmp(&eb.file_name()));

        album_dir_paths
    }

    // pub fn collect_testcase_paths(root_dir: &Path) -> Vec<PathBuf> {
    //     let read_dir = std::fs::read_dir(root_dir).expect("cannot read root dir");

    //     let mut testcase_paths = read_dir.filter_map(|res| {
    //         let dir_entry = res.expect("cannot read subentry in root dir");

    //         let metadata = dir_entry.metadata().expect("cannot read subentry metadata");

    //         // Skip any entries that are not files with a JSON extension.
    //         if metadata.is_file() {
    //             let path = dir_entry.path();
    //             path.extension().contains(&"json").then(|| path)
    //         }
    //         else {
    //             None
    //         }
    //     })
    //     .collect::<Vec<_>>();

    //     testcase_paths.sort_by(|ea, eb| ea.file_name().cmp(&eb.file_name()));

    //     testcase_paths
    // }

    pub fn collect_album_dir_items(album_dir: &Path) -> Vec<PathBuf> {
        let read_dir = std::fs::read_dir(album_dir).expect("cannot read album dir");

        let mut track_paths = read_dir
            .map(|res| {
                let dir_entry = res.expect("cannot read subentry in album dir");

                let metadata = dir_entry.metadata().expect("cannot read subentry metadata");

                assert!(metadata.is_file(), "subentry is not a file");

                let track_path = dir_entry.path();

                track_path
            })
            .collect::<Vec<_>>();

        track_paths.sort_by(|ea, eb| ea.file_name().cmp(&eb.file_name()));

        track_paths
    }

    pub fn load_testcase(testcase_path: &Path) -> AlbumTestcase {
        let testcase_str = std::fs::read_to_string(testcase_path).expect("unable to read testcase file");

        serde_json::from_str(&testcase_str).expect("unable to deserialize testcase")
    }

    pub fn analyze_albums(root_dir: &Path) {
        let album_dirs = Self::collect_album_dirs(root_dir);

        let mut n = 0;
        for album_dir in album_dirs {
            n += 1;

            let album_dir_items = Self::collect_album_dir_items(&album_dir);

            for album_dir_item in album_dir_items {
                match TestReader::read_path(&album_dir_item) {
                    Err(ReaderError::BadExt | ReaderError::NoExt) => continue,
                    res => {
                        let reader = res.expect("unable to read track");
                    },
                }
            }

            let mut album_dir = album_dir;
            let testcase_path = {
                album_dir.push(".json");
                album_dir
            };

            let testcase = Self::load_testcase(&testcase_path);

            println!("Analyzing testcase #{}: '{}'", n, testcase.name);
        }
    }

    pub fn check_sox() -> bool {
        Command::new("sox").arg("--version")
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub fn load_flac_data(path: &Path) -> Result<FlacFrames<File>, LoadFlacError> {
        let file = File::open(path).map_err(LoadFlacError::Io)?;

        let reader = FlacReader::new(file).map_err(LoadFlacError::Flac)?;

        Ok(FlacFrames::new(reader))
    }

    pub fn load_wav_data(path: &Path) -> Result<WavFrames<File>, LoadWavError> {
        let file = File::open(path).map_err(LoadWavError::Io)?;

        let reader = WavReader::new(file).map_err(LoadWavError::Wav)?;

        Ok(WavFrames::new(reader))
    }

    pub fn sox_eval(cmd: &mut Command) -> Vec<u8> {
        let output = cmd.output()
            .unwrap_or_else(|e| panic!("failed to execute command: {}", e));

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

        String::from_utf8(raw_stdout)
            .unwrap_or_else(|e| panic!("cannot convert stdout bytes into string: {}", e))
    }

    pub fn sox_eval_samples(cmd: &mut Command) -> Vec<f64> {
        let raw_stdout = Self::sox_eval(cmd);

        let mut res = vec![0.0f64; raw_stdout.len() / std::mem::size_of::<f64>()];
        LittleEndian::read_f64_into(&raw_stdout, &mut res);

        res
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
