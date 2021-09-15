use std::collections::{HashMap, HashSet};

use sampara::{Frame, Calculator};

use crate::filter::KWeightFilter;
use crate::gated_loudness::{Gating, GatedLoudness};

pub struct Output {
    pub averages: HashMap<Gating, Option<f64>>,
    pub maximums: HashMap<Gating, Option<f64>>,
}

pub struct PipelineLayer<F, const N: usize>
where
    F: Frame<N, Sample = f64>,
{
    k_filter: KWeightFilter<F, N>,
    avg_gl_map: HashMap<Gating, GatedLoudness<F, N>>,
    max_gl_map: HashMap<Gating, GatedLoudness<F, N>>,
}

impl<F, const N: usize> PipelineLayer<F, N>
where
    F: Frame<N, Sample = f64>,
{
    fn new(sample_rate: u32, g_weights: F, avg_gatings: &HashSet<Gating>, max_gatings: &HashSet<Gating>) -> Self {
        let k_filter = KWeightFilter::new(sample_rate);

        let avg_gl_map = avg_gatings.iter()
            .map(|&g| (g, GatedLoudness::new(sample_rate, g_weights, g)))
            .collect();
        let max_gl_map = max_gatings.iter()
            .map(|&g| (g, GatedLoudness::new(sample_rate, g_weights, g)))
            .collect();

        Self {
            k_filter,
            avg_gl_map,
            max_gl_map,
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

impl<F, const N: usize> Calculator for PipelineLayer<F, N>
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

pub struct Pipeline<F, const N: usize>
where
    F: Frame<N, Sample = f64>,
{
    pub sample_rate: u32,
    pub g_weights: F,

    // Gatings to calculate for both averages and maximums.
    avg_gatings: HashSet<Gating>,
    max_gatings: HashSet<Gating>,

    // The stack of layers, starting with the root layer.
    layers: Vec<PipelineLayer<F, N>>,
}

impl<F, const N: usize> Pipeline<F, N>
where
    F: Frame<N, Sample = f64>,
{
    pub fn push(&mut self, frame: F) {
        for layer in self.layers.iter_mut() {
            layer.push(frame);
        }
    }

    pub fn calculate(mut self) -> Option<(Output, Self)> {
        self.layers.pop()
            .map(|layer| (layer.calculate(), self))
    }

    fn create_layer(&self) -> PipelineLayer<F, N> {
        let Self { sample_rate, g_weights, avg_gatings, max_gatings, .. } = self;

        PipelineLayer::new(*sample_rate, *g_weights, avg_gatings, max_gatings)
    }

    pub fn push_layer(&mut self) {
        let new_layer = self.create_layer();
        self.layers.push(new_layer);
    }

    pub fn pop_layer(&mut self) -> Option<Output> {
        self.layers.pop().map(|l| l.calculate())
    }

    pub fn oneshot_sublayer<I>(&mut self, frames: I) -> Output
    where
        I: IntoIterator<Item = F>,
    {
        let mut oneshot_layer = self.create_layer();

        for frame in frames {
            self.push(frame);
            oneshot_layer.push(frame);
        }

        oneshot_layer.calculate()
    }
}

impl<F, const N: usize> Calculator for Pipeline<F, N>
where
    F: Frame<N, Sample = f64>,
{
    type Input = F;
    type Output = Option<(Output, Self)>;

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
    num_layers: usize,
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
            num_layers: 0,
        }
    }

    #[inline]
    pub fn average(&mut self, gating: Gating) -> &mut Self {
        self.avg_gatings.insert(gating);
        self
    }

    #[inline]
    pub fn maximum(&mut self, gating: Gating) -> &mut Self {
        self.max_gatings.insert(gating);
        self
    }

    #[inline]
    pub fn num_layers(&mut self, n: usize) -> &mut Self {
        self.num_layers = n;
        self
    }

    pub fn build(self) -> Pipeline<F, N> {
        let Self { sample_rate, g_weights, avg_gatings, max_gatings, num_layers } = self;

        let mut layers = Vec::new();
        layers.resize_with(num_layers, || {
            PipelineLayer::new(sample_rate, g_weights, &avg_gatings, &max_gatings)
        });

        Pipeline {
            sample_rate,
            g_weights,
            avg_gatings,
            max_gatings,
            layers,
        }
    }
}
