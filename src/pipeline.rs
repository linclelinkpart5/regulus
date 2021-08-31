use std::collections::{HashMap, HashSet};

use sampara::{Frame, Calculator};

use crate::filter::KWeightFilter;
use crate::gated_loudness::{Gating, GatedLoudness};

pub struct PipelineBuilder<F, const N: usize>
where
    F: Frame<N, Sample = f64>,
{
    sample_rate: u32,
    g_weights: F,
    gatings: HashSet<Gating>,
}

impl<F, const N: usize> PipelineBuilder<F, N>
where
    F: Frame<N, Sample = f64>,
{
    pub fn new(sample_rate: u32, g_weights: F) -> Self {
        Self {
            sample_rate,
            g_weights,
            gatings: HashSet::new(),
        }
    }

    #[inline]
    pub fn gating(&mut self, gating: Gating) -> &mut Self {
        self.gatings.insert(gating);
        self
    }

    #[inline]
    pub fn momentary(&mut self) -> &mut Self {
        self.gating(Gating::Momentary)
    }

    #[inline]
    pub fn shortterm(&mut self) -> &mut Self {
        self.gating(Gating::Shortterm)
    }

    #[inline]
    pub fn custom(&mut self, gate_len_ms: u64, delta_len_ms: u64) -> &mut Self {
        self.gating(Gating::Custom { gate_len_ms, delta_len_ms })
    }

    pub fn build(&mut self) -> Pipeline<F, N> {
        let sample_rate = self.sample_rate;
        let g_weights = self.g_weights;

        let k_filter = KWeightFilter::new(sample_rate);
        let gated_loudness_map = self.gatings.drain()
            .map(|g| (g, GatedLoudness::new(sample_rate, g_weights, g)))
            .collect::<HashMap<_, _>>();

        Pipeline {
            k_filter,
            gated_loudness_map,
        }
    }
}

pub struct Pipeline<F, const N: usize>
where
    F: Frame<N, Sample = f64>,
{
    k_filter: KWeightFilter<F, N>,
    gated_loudness_map: HashMap<Gating, GatedLoudness<F, N>>,
}

impl<F, const N: usize> Pipeline<F, N>
where
    F: Frame<N, Sample = f64>,
{
    pub fn is_noop(&self) -> bool {
        self.gated_loudness_map.is_empty()
    }
}

impl<F, const N: usize> Calculator for Pipeline<F, N>
where
    F: Frame<N, Sample = f64>,
{
    type Input = F;
    type Output = HashMap<Gating, Option<f64>>;

    fn push(&mut self, input: Self::Input) {
        let filtered_frame = self.k_filter.process(input);

        for gated_loudness in self.gated_loudness_map.values_mut() {
            gated_loudness.push(filtered_frame);
        }
    }

    fn calculate(self) -> Self::Output {
        self.gated_loudness_map.into_iter()
            .map(|(gating, gated_loudness)| {
                (gating, gated_loudness.calculate())
            })
            .collect()
    }
}
