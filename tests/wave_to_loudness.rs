#![cfg(test)]

#[macro_use] extern crate approx;

use std::f64::consts::PI;

use regulus::filter::FilteredSampleIter;
use regulus::gating::GatedPowerIter;
use regulus::loudness::Loudness;

pub struct SineGen {
    sample_rate: u32,
    sample_index: u32,
    frequency: f64,
    amplitude: f64,
}

impl SineGen {
    pub fn new(sample_rate: u32, frequency: f64, amplitude: f64) -> Self {
        Self {
            sample_rate,
            sample_index: 0,
            frequency,
            amplitude,
        }
    }
}

impl Iterator for SineGen {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        let x = self.sample_index as f64 * self.frequency / self.sample_rate as f64;
        let y = (2.0 * PI * x).sin() * self.amplitude;
        self.sample_index = (self.sample_index + 1) % self.sample_rate;

        Some(y)
    }
}

#[test]
fn test_nominal_frequency_reading() {
    // As per the ITU BS.1770 spec:
    // If a 0 dB FS, 997 Hz sine wave is applied to the left, center, or right channel input,
    // the indicated loudness will equal -3.01 LKFS.
    let sample_rate: u32 = 48000;
    let raw_signal = SineGen::new(sample_rate, 997.0, 1.0).map(|x| [x, 0.0, 0.0, 0.0, 0.0]).take(sample_rate as usize * 10);
    let filtered_signal = FilteredSampleIter::new(raw_signal, sample_rate);
    let gated_channel_powers_iter = GatedPowerIter::new(filtered_signal, sample_rate);
    let loudness = Loudness::from_gated_channel_powers(gated_channel_powers_iter, [1.0, 1.0, 1.0, 1.0, 1.0]);

    // assert_abs_diff_eq!(-3.01, loudness);
    assert_abs_diff_eq!(-3.010279921396327, loudness);
}
