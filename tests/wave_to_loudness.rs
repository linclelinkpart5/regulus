#![cfg(test)]

#[macro_use] extern crate approx;

use std::f64::consts::PI;

use regulus::gating::GatedPowerIter;
use regulus::loudness::Loudness;

pub struct SineGen {
    samples_per_period: usize,
    sample_index: usize,
    frequency: f64,
    amplitude: f64,
}

impl SineGen {
    pub fn new(samples_per_period: usize, frequency: f64, amplitude: f64) -> Self {
        Self {
            samples_per_period,
            sample_index: 0,
            frequency,
            amplitude,
        }
    }
}

impl Iterator for SineGen {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        let x = self.sample_index as f64 * self.frequency / self.samples_per_period as f64;
        let y = (2.0 * PI * x).sin() * self.amplitude;
        self.sample_index = (self.sample_index + 1) % self.samples_per_period;

        Some(y)
    }
}

#[test]
fn test_nominal_frequency_reading() {
    // As per the ITU BS.1770 spec:
    // If a 0 dB FS, 997 Hz sine wave is applied to the left, center, or right channel input,
    // the indicated loudness will equal -3.01 LKFS.
    let reference_signal = SineGen::new(44100, 997.0, 1.0).map(|x| [x, 0.0, 0.0, 0.0, 0.0]).take(441000);

    let gated_channel_powers_iter = GatedPowerIter::new(reference_signal, 44100);
    let loudness = Loudness::from_gated_channel_powers(gated_channel_powers_iter, [1.0, 1.0, 1.0, 1.0, 1.0]);

    // assert_abs_diff_eq!(-3.01, loudness);
}
