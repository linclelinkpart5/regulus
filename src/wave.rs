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
