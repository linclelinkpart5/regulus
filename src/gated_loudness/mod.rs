pub mod gating;
pub mod loudness;

pub use gating::*;
pub use loudness::*;

use sampara::{Frame, Calculator};

pub struct GatedLoudness<F, const N: usize>
where
    F: Frame<N, Sample = f64>,
{
    gated_powers: GatedPowers<F, N>,
    loudness: Loudness<F, N>,
}

impl<F, const N: usize> GatedLoudness<F, N>
where
    F: Frame<N, Sample = f64>,
{
    pub fn new(sample_rate: u32, g_weights: F, gating: Gating) -> Self {
        let gated_powers = GatedPowers::new(sample_rate, gating);
        let loudness = Loudness::new(g_weights);

        Self {
            gated_powers,
            loudness,
        }
    }

    pub fn reset(&mut self) {
        self.gated_powers.reset();
        self.loudness.reset();
    }

    pub fn momentary(sample_rate: u32, g_weights: F) -> Self {
        Self::new(sample_rate, g_weights, Gating::Momentary)
    }

    pub fn shortterm(sample_rate: u32, g_weights: F) -> Self {
        Self::new(sample_rate, g_weights, Gating::Shortterm)
    }

    pub fn custom(sample_rate: u32, g_weights: F, gate_len_ms: u64, delta_len_ms: u64) -> Self {
        Self::new(sample_rate, g_weights, Gating::Custom { gate_len_ms, delta_len_ms })
    }
}

impl<F, const N: usize> Calculator for GatedLoudness<F, N>
where
    F: Frame<N, Sample = f64>,
{
    type Input = F;
    type Output = Option<f64>;

    fn push(&mut self, input: Self::Input) {
        if let Some(gp) = self.gated_powers.process(input) {
            self.loudness.push(gp)
        }
    }

    fn calculate(self) -> Self::Output {
        self.loudness.calculate()
    }
}
