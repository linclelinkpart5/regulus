use sampara::{Frame, Calculator};

use crate::filter::KWeightFilter;
use crate::gated_loudness::{GatingKind, GatedPowers, Loudness};

pub struct Pipeline<F, const N: usize, const P: usize>
where
    F: Frame<N, Sample = f64>,
{
    k_filter: KWeightFilter<F, N>,
    gated_power_pipelines: [GatedPowers<F, N>; P],
    loudness_pipelines: [Loudness<F, N>; P],
}

impl<F, const N: usize, const P: usize> Pipeline<F, N, P>
where
    F: Frame<N, Sample = f64>,
{
    pub fn new(sample_rate: u32, gating_kinds: [GatingKind; P], g_weights: F) -> Self {
        let k_filter = KWeightFilter::new(sample_rate);
        let gated_power_pipelines = gating_kinds.map(|k| GatedPowers::new(sample_rate, k));
        let loudness_pipelines = gating_kinds.map(|_| Loudness::new(g_weights));

        Self {
            k_filter,
            gated_power_pipelines,
            loudness_pipelines,
        }
    }
}

impl<F, const N: usize, const P: usize> Calculator for Pipeline<F, N, P>
where
    F: Frame<N, Sample = f64>,
{
    type Input = F;
    type Output = [Option<f64>; P];

    fn push(&mut self, input: Self::Input) {
        let filtered_frame = self.k_filter.process(input);

        let opt_powers = self.gated_power_pipelines.each_mut().map(|gpp| {
            gpp.process(filtered_frame)
        });

        self.loudness_pipelines.each_mut().zip(opt_powers).map(|(lp, opt_power)| {
            opt_power.map(|p| lp.push(p))
        });
    }

    fn calculate(self) -> Self::Output {
        self.loudness_pipelines.map(|lp| lp.calculate())
    }
}
