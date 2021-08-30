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
use hound::{Error as HoundError, WavReader, WavIntoSamples, SampleFormat, Result as HoundResult};
use serde::Deserialize;

use crate::filter::KWeightFilter;
use crate::gated_loudness::{GatedPowers, Loudness, Gating};

const MAX_CHANNELS: usize = 5;
const G_WEIGHTS: [f64; MAX_CHANNELS] = [1.0, 1.0, 1.0, 1.41, 1.41];

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
        let power_gater = GatedPowers::momentary(sample_rate);

        let filtered_signal = signal.process(k_weighter);
        let gated_signal = filtered_signal.process_lazy(power_gater);

        let loudness = gated_signal.calculate(Loudness::new(G_WEIGHTS)).unwrap();

        println!("Loudness: {}", loudness)
    }
}

type ReaderFunc = fn(&Path) -> Result<TestReader<File>, ReaderError>;

impl TestReader<File> {
    pub fn get_reader_func(track_path: &Path) -> Result<ReaderFunc, ReaderError> {
        let ext = track_path
            .extension()
            .ok_or(ReaderError::NoExt)?;

        if ext == "flac" {
            Ok(|p| Ok(Self::Flac(TestUtil::load_flac_data(p)?)))
        } else if ext == "wav" {
            Ok(|p| Ok(Self::Wav(TestUtil::load_wav_data(p)?)))
        } else {
            Err(ReaderError::BadExt)
        }
    }

    pub fn read_track(track_path: &Path) -> Result<Self, ReaderError> {
        let loader = Self::get_reader_func(track_path)?;

        (loader)(track_path)
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

#[derive(Deserialize, Default)]
pub(crate) struct Analysis {
    momentary_mean: f64,
    momentary_maximum: f64,
    momentary_range: f64,
    shortterm_mean: f64,
    shortterm_maximum: f64,
    shortterm_range: f64,
}

#[derive(Deserialize, Default)]
pub(crate) struct AlbumAnalysis {
    #[serde(flatten)]
    album: Analysis,
    tracks: Vec<Analysis>,
}

pub(crate) struct TestUtil;

impl TestUtil {
    pub fn load_analysis(analysis_path: &Path) -> AlbumAnalysis {
        let analysis_str = std::fs::read_to_string(analysis_path).expect("unable to read analysis file");

        serde_json::from_str(&analysis_str).expect("unable to deserialize analysis")
    }

    /// Collects album testcases from the testcase root directory. A testcase
    /// consists of an expected album analysis result, along with the album
    /// dir path.
    pub fn collect_album_testcases(testcase_root_dir: &Path) -> Vec<(AlbumAnalysis, PathBuf)> {
        let read_dir = std::fs::read_dir(testcase_root_dir).expect("cannot read root dir");

        let mut album_dir_paths = read_dir.filter_map(|res| {
            let dir_entry = res.expect("cannot read subentry in root dir");

            let metadata = dir_entry.metadata().expect("cannot read subentry metadata");

            // Only keep directories.
            if !metadata.is_dir() {
                return None;
            }

            // Try and find the expected analysis result file for this album
            // root dir.
            let exp_analysis_name = {
                let mut n = dir_entry.file_name();
                n.push(".json");
                n
            };
            let exp_analysis_path = testcase_root_dir.join(&exp_analysis_name);
            let exp_analysis = Self::load_analysis(&exp_analysis_path);

            Some((exp_analysis, dir_entry.path()))
        })
        .collect::<Vec<_>>();

        album_dir_paths.sort_by(|(_, dir_a), (_, dir_b)| dir_a.file_name().cmp(&dir_b.file_name()));

        album_dir_paths
    }

    pub fn run_track_analysis<R, F>(track_reader: TestReader<R>, frame_callback: F) -> Analysis
    where
        R: Read,
        F: FnMut([f64; MAX_CHANNELS]) -> (),
    {
        let mut frame_callback = frame_callback;

        let sample_rate = track_reader.sample_rate();

        let mut k_weighter = KWeightFilter::new(sample_rate);

        let mut momentary_gater = GatedPowers::momentary(sample_rate);
        let mut shortterm_gater = GatedPowers::shortterm(sample_rate);

        let mut momentary_loudness_calc = Loudness::new(G_WEIGHTS);
        let mut shortterm_loudness_calc = Loudness::new(G_WEIGHTS);

        for res_frame in track_reader {
            let frame = res_frame.expect("unable to read frame");

            // The K-weighting step is done before any momentary or
            // shortterm calculations.
            let filtered_frame = k_weighter.process(frame);

            if let Some(momentary_gated_frame) = momentary_gater.process(filtered_frame) {
                momentary_loudness_calc.push(momentary_gated_frame);
            }

            if let Some(shortterm_gated_frame) = shortterm_gater.process(filtered_frame) {
                shortterm_loudness_calc.push(shortterm_gated_frame);
            }

            // Also feed the original frame to the callback function.
            frame_callback(frame);
        }

        let momentary_mean = momentary_loudness_calc.calculate()
            .expect("unable to calculate momentary loudness for track");
        let shortterm_mean = shortterm_loudness_calc.calculate()
            .expect("unable to calculate shortterm loudness for track");

        let track_analysis = Analysis {
            momentary_mean,
            momentary_maximum: 0.0,
            momentary_range: 0.0,
            shortterm_mean,
            shortterm_maximum: 0.0,
            shortterm_range: 0.0,
        };

        track_analysis
    }

    pub fn run_album_analysis(album_dir: &Path) -> AlbumAnalysis {
        let track_bundles = Self::collect_track_bundles(album_dir);

        let mut track_analyses = Vec::with_capacity(track_bundles.len());

        // Keep track of the sample rate across tracks.
        // TODO: Should support be added for albums with tracks with different
        //       sample rates?
        let mut expected_sample_rate = None;

        for (track_path, load_func) in track_bundles {
            let track_reader = (load_func)(&track_path).expect("unable to read track file");

            let sample_rate = track_reader.sample_rate();

            if let Some(r) = expected_sample_rate {
                assert_eq!(r, sample_rate, "different sample rate for track: expected {}, got {}", r, sample_rate);
            }
            else {
                expected_sample_rate = Some(sample_rate);
            }

            let mut k_weighter = KWeightFilter::new(sample_rate);

            let mut momentary_gater = GatedPowers::momentary(sample_rate);
            let mut shortterm_gater = GatedPowers::shortterm(sample_rate);

            let mut momentary_loudness_calc = Loudness::new(G_WEIGHTS);
            let mut shortterm_loudness_calc = Loudness::new(G_WEIGHTS);

            for res_frame in track_reader {
                let frame = res_frame.expect("unable to read frame");

                // The K-weighting step is done before any momentary or
                // shortterm calculations.
                let filtered_frame = k_weighter.process(frame);

                if let Some(momentary_gated_frame) = momentary_gater.process(filtered_frame) {
                    momentary_loudness_calc.push(momentary_gated_frame);
                }

                if let Some(shortterm_gated_frame) = shortterm_gater.process(filtered_frame) {
                    shortterm_loudness_calc.push(shortterm_gated_frame);
                }
            }

            let momentary_mean = momentary_loudness_calc.calculate().expect("unable to calculate momentary loudness for track");
            let shortterm_mean = shortterm_loudness_calc.calculate().expect("unable to calculate shortterm loudness for track");

            let track_analysis = Analysis {
                momentary_mean,
                momentary_maximum: 0.0,
                momentary_range: 0.0,
                shortterm_mean,
                shortterm_maximum: 0.0,
                shortterm_range: 0.0,
            };

            track_analyses.push(track_analysis);
        }

        let album_analysis = AlbumAnalysis {
            album: Analysis::default(),
            tracks: track_analyses,
        };

        album_analysis
    }

    // pub fn collect_testcase_paths(testcase_root_dir: &Path) -> Vec<PathBuf> {
    //     let read_dir = std::fs::read_dir(testcase_root_dir).expect("cannot read root dir");

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

    pub fn collect_track_bundles(album_dir: &Path) -> Vec<(PathBuf, ReaderFunc)> {
        let read_dir = std::fs::read_dir(album_dir).expect("cannot read album dir");

        let mut track_paths = read_dir
            .map(|res| {
                let dir_entry = res.expect("cannot read subentry in album dir");

                let metadata = dir_entry.metadata().expect("cannot read subentry metadata");

                assert!(metadata.is_file(), "subentry is not a file");

                let track_path = dir_entry.path();

                let load_func = TestReader::get_reader_func(&track_path).expect("unknown track format");

                (track_path, load_func)
            })
            .collect::<Vec<_>>();

        track_paths.sort_by(|(tp_a, _), (tp_b, _)| tp_a.file_name().cmp(&tp_b.file_name()));

        track_paths
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
