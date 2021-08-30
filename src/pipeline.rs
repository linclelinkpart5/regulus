// use std::collections::HashMap;

// use sampara::{Frame, Calculator};

// use crate::filter::KWeightFilter;
// use crate::gated_loudness::{Gating, GatedLoudness};

// pub struct PipelineBuilder

// pub struct Pipeline<F, const N: usize>
// where
//     F: Frame<N, Sample = f64>,
// {
//     k_filter: KWeightFilter<F, N>,
//     gated_loudness_specs: HashMap<Gating, GatedLoudness<F, N>>,
// }

// impl<F, const N: usize> Pipeline<F, N>
// where
//     F: Frame<N, Sample = f64>,
// {
//     pub fn new(sample_rate: u32, g_weights: F) -> Self {
//         let k_filter = KWeightFilter::new(sample_rate);
//         let gated_loudness_specs = HashMap::new();

//         Self {
//             k_filter,
//             gated_loudness_specs,
//         }
//     }

//     pub fn is_noop(&self) -> bool {
//         self.gated_loudness_specs.is_empty()
//     }
// }

// impl<F, const N: usize> Calculator for Pipeline<F, N>
// where
//     F: Frame<N, Sample = f64>,
// {
//     type Input = F;
//     type Output = [Option<f64>; P];

//     fn push(&mut self, input: Self::Input) {
//         let filtered_frame = self.k_filter.process(input);

//         let opt_powers = self.gated_power_pipelines.each_mut().map(|gpp| {
//             gpp.process(filtered_frame)
//         });

//         self.loudness_pipelines.each_mut().zip(opt_powers).map(|(lp, opt_power)| {
//             opt_power.map(|p| lp.push(p))
//         });
//     }

//     fn calculate(self) -> Self::Output {
//         self.loudness_pipelines.map(|lp| lp.calculate())
//     }
// }
