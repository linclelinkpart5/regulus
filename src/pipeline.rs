use std::collections::{HashMap, HashSet};

use sampara::{Frame, Calculator};

use crate::filter::KWeightFilter;
use crate::gated_loudness::{Gating, GatedLoudness};

#[derive(Debug, Clone)]
pub struct Output {
    pub averages: HashMap<Gating, Option<f64>>,
    pub maximums: HashMap<Gating, Option<f64>>,
}

pub struct Pipeline<F, const N: usize>
where
    F: Frame<N, Sample = f64>,
{
    k_filter: KWeightFilter<F, N>,
    avg_gl_map: HashMap<Gating, GatedLoudness<F, N>>,
    max_gl_map: HashMap<Gating, GatedLoudness<F, N>>,
}

impl<F, const N: usize> Pipeline<F, N>
where
    F: Frame<N, Sample = f64>,
{
    pub fn reset(&mut self) {
        self.k_filter.reset();

        for avg_gl in self.avg_gl_map.values_mut() {
            avg_gl.reset();
        }

        for max_gl in self.max_gl_map.values_mut() {
            max_gl.reset();
        }
    }

    pub fn is_noop(&self) -> bool {
        self.avg_gl_map.is_empty() && self.max_gl_map.is_empty()
    }

    pub fn feed<I>(&mut self, frames: I)
    where
        I: IntoIterator<Item = F>,
    {
        for frame in frames.into_iter() {
            self.push(frame);
        }
    }

    pub fn push(&mut self, input: F) {
        let filtered_frame = self.k_filter.process(input);

        for gated_loudness in self.avg_gl_map.values_mut() {
            gated_loudness.push(filtered_frame);
        }

        for gated_loudness in self.max_gl_map.values_mut() {
            gated_loudness.push(filtered_frame);
        }
    }

    pub fn calculate(self) -> Output {
        let averages = self.avg_gl_map.into_iter()
            .map(|(gating, gl)| {
                (gating, gl.calculate())
            })
            .collect();

        let maximums = self.max_gl_map.into_iter()
            .map(|(gating, gl)| {
                (gating, gl.calculate())
            })
            .collect();

        Output {
            averages,
            maximums,
        }
    }
}

impl<F, const N: usize> Calculator for Pipeline<F, N>
where
    F: Frame<N, Sample = f64>,
{
    type Input = F;
    type Output = Output;

    fn push(&mut self, input: Self::Input) {
        self.push(input)
    }

    fn calculate(self) -> Self::Output {
        self.calculate()
    }
}

#[derive(Clone, Debug)]
pub struct PipelineBuilder<F, const N: usize>
where
    F: Frame<N, Sample = f64>,
{
    sample_rate: u32,
    g_weights: F,
    avg_gatings: HashSet<Gating>,
    max_gatings: HashSet<Gating>,
}

impl<F, const N: usize> PipelineBuilder<F, N>
where
    F: Frame<N, Sample = f64>,
{
    pub fn new(sample_rate: u32, g_weights: F) -> Self {
        Self {
            sample_rate,
            g_weights,
            avg_gatings: HashSet::new(),
            max_gatings: HashSet::new(),
        }
    }

    #[inline]
    pub fn average(&mut self, gating: Gating) -> &mut Self {
        self.avg_gatings.insert(gating);
        self
    }

    #[inline]
    pub fn averages<I>(&mut self, gatings: I) -> &mut Self
    where
        I: IntoIterator<Item = Gating>,
    {
        for gating in gatings {
            self.avg_gatings.insert(gating);
        }
        self
    }

    #[inline]
    pub fn maximum(&mut self, gating: Gating) -> &mut Self {
        self.max_gatings.insert(gating);
        self
    }

    #[inline]
    pub fn maximums<I>(&mut self, gatings: I) -> &mut Self
    where
        I: IntoIterator<Item = Gating>,
    {
        for gating in gatings {
            self.max_gatings.insert(gating);
        }
        self
    }

    pub fn build(&self) -> Pipeline<F, N> {
        let Self { sample_rate, g_weights, avg_gatings, max_gatings } = self;

        let k_filter = KWeightFilter::new(*sample_rate);

        let avg_gl_map = avg_gatings.iter()
            .map(|&g| (g, GatedLoudness::new(*sample_rate, *g_weights, g)))
            .collect();
        let max_gl_map = max_gatings.iter()
            .map(|&g| (g, GatedLoudness::new(*sample_rate, *g_weights, g)))
            .collect();

        Pipeline {
            k_filter,
            avg_gl_map,
            max_gl_map,
        }
    }
}
