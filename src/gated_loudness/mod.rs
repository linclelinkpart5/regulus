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
    loudness_calc: Loudness<F, N>,
}

impl<F, const N: usize> Calculator for GatedLoudness<F, N>
where
    F: Frame<N, Sample = f64>,
{
    type Input = F;
    type Output = Option<f64>;

    fn push(&mut self, input: Self::Input) {
        if let Some(gp) = self.gated_powers.process(input) {
            self.loudness_calc.push(gp)
        }
    }

    fn calculate(self) -> Self::Output {
        self.loudness_calc.calculate()
    }
}
