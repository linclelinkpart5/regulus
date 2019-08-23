#![cfg(test)]

use std::f64::consts::PI;

use crate::constants::MAX_CHANNELS;

#[derive(Clone, Copy)]
pub enum WaveKind {
    Sine,
    Square,
    Triangle,
    Sawtooth,
}

impl WaveKind {
    pub fn val(&self, sample_index: usize, samples_per_period: usize, frequency: u32) -> f64 {
        let x = sample_index as f64 * frequency as f64 / samples_per_period as f64;
        match self {
            &WaveKind::Sine => (2.0 * PI * x).sin(),
            &WaveKind::Square => (-1.0f64).powf((2.0 * x).floor()),
            &WaveKind::Triangle => 1.0 - 4.0 * (0.5 - (x + 0.25).fract()).abs(),
            &WaveKind::Sawtooth => 2.0 * x.fract() - 1.0,
        }
    }
}

pub struct WaveGen {
    kind: WaveKind,
    samples_per_period: usize,
    sample_index: usize,
    frequencies: [u32; MAX_CHANNELS],
}

impl WaveGen {
    pub fn new(kind: WaveKind, samples_per_period: usize, frequencies: [u32; MAX_CHANNELS]) -> Self {
        Self {
            kind,
            samples_per_period,
            sample_index: 0,
            frequencies,
        }
    }
}

impl Iterator for WaveGen {
    type Item = [f64; MAX_CHANNELS];

    fn next(&mut self) -> Option<Self::Item> {
        let mut o = [0.0f64; MAX_CHANNELS];

        for ch in 0..MAX_CHANNELS {
            o[ch] = self.kind.val(self.sample_index, self.samples_per_period, self.frequencies[ch]);
        }

        self.sample_index = (self.sample_index + 1) % self.samples_per_period;
        Some(o)
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_gen() {
        let mut sine_gen = SineGen::new(48000, 997.0, 1.0);

        let expected = [
            0.0,
            0.13013684267905243,
            0.25806032898427383,
            0.38159474711916214,
            0.4986390341860973,
            0.6072025108857589,
            0.7054387388216735,
            0.7916769245677292,
            0.8644503363809454,
            0.9225212502504557,
            0.9649020010024225,
            0.9908717804254776,
            0.999988896715596,
            0.992098286732807,
            0.9673341533019163,
            0.9261176827022533,
            0.8691498811671841,
            0.7973996522295976,
            0.712087317692863,
            0.6146638625011827,
            0.5067862565108243,
            0.39028927288745724,
            0.2671542824398906,
            0.1394755546335431,
            0.009424638433143987,
            -0.12078657121773094,
            -0.24894345311810595,
            -0.3728663258990093,
            -0.49044751985917323,
            -0.5996872240324127,
            -0.6987274988027844,
            -0.7858838755838653,
        ];

        for e in expected.iter() {
            let e = *e;
            let p = sine_gen.next().unwrap();
            assert_abs_diff_eq!(e, p);
        }
    }
}
