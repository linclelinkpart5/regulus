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
    pub fn custom(gating: Gating, sample_rate: u32, g_weights: F) -> Self {
        let gated_powers = GatedPowers::custom(gating, sample_rate);
        let loudness = Loudness::new(g_weights);

        Self {
            gated_powers,
            loudness,
        }
    }

    pub fn momentary(sample_rate: u32, g_weights: F) -> Self {
        Self::custom(MOMENTARY_GATING, sample_rate, g_weights)
    }

    pub fn shortterm(sample_rate: u32, g_weights: F) -> Self {
        Self::custom(SHORTTERM_GATING, sample_rate, g_weights)
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
