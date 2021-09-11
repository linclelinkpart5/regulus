use std::collections::{HashMap, HashSet};

use sampara::{Frame, Calculator, Signal};

use crate::filter::KWeightFilter;
use crate::gated_loudness::{Gating, GatedLoudness};

pub struct Output {
    momentary_mean: Option<f64>,
    shortterm_mean: Option<f64>,
    custom_gating_means: HashMap<Gating, Option<f64>>
}

pub struct Config<F, const N: usize>
where
    F: Frame<N, Sample = f64>,
{
    pub sample_rate: u32,
    pub g_weights: F
}

enum Upstream<F, const N: usize>
where
    F: Frame<N, Sample = f64>,
{
    Parent(Box<Pipeline<F, N>>),
    Source(Config<F, N>),
}

impl<F, const N: usize> Upstream<F, N>
where
    F: Frame<N, Sample = f64>,
{
    #[inline(always)]
    fn config(&self) -> &Config<F, N> {
        match self {
            Self::Parent(parent_pipeline) => parent_pipeline.upstream.config(),
            Self::Source(config) => config,
        }
    }

    #[inline(always)]
    fn push(&mut self, input: F) {
        match self {
            Self::Parent(parent_pipeline) => parent_pipeline.push(input),
            Self::Source(..) => {},
        }
    }

    fn into_parent(self) -> Option<Pipeline<F, N>> {
        match self {
            Self::Parent(parent_pipeline) => Some(Box::into_inner(parent_pipeline)),
            Self::Source(..) => None,
        }
    }
}

pub struct PipelineLayer<F, const N: usize>
where
    F: Frame<N, Sample = f64>,
{
    k_filter: KWeightFilter<F, N>,
    momentary_gl: Option<GatedLoudness<F, N>>,
    shortterm_gl: Option<GatedLoudness<F, N>>,
    custom_gl_map: HashMap<Gating, GatedLoudness<F, N>>,
}

impl<F, const N: usize> PipelineLayer<F, N>
where
    F: Frame<N, Sample = f64>,
{
    pub fn is_noop(&self) -> bool {
        self.momentary_gl.is_none() && self.shortterm_gl.is_none() && self.custom_gl_map.is_empty()
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

        self.momentary_gl.as_mut().map(|gl| gl.push(filtered_frame));
        self.shortterm_gl.as_mut().map(|gl| gl.push(filtered_frame));

        for gated_loudness in self.custom_gl_map.values_mut() {
            gated_loudness.push(filtered_frame);
        }
    }

    pub fn calculate(self) -> Output {
        Output {
            momentary_mean: self.momentary_gl.and_then(|gl| gl.calculate()),
            shortterm_mean: self.shortterm_gl.and_then(|gl| gl.calculate()),
            custom_gating_means: self.custom_gl_map.into_iter()
                .map(|(gating, gl)| {
                    (gating, gl.calculate())
                })
                .collect(),
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

pub struct LayeredPipeline<F, const N: usize>
where
    F: Frame<N, Sample = f64>,
{
    pub sample_rate: u32,
    pub g_weights: F,

    // Flags for calculating preset/common gatings.
    calc_momentary: bool,
    calc_shortterm: bool,

    // Custom gatings to calculate, usually will be empty.
    custom_gatings: HashSet<Gating>,

    // The stack of layers, starting with the root, and ending with the current child.
    layers: Vec<PipelineLayer<F, N>>,
}

impl<F, const N: usize> LayeredPipeline<F, N>
where
    F: Frame<N, Sample = f64>,
{
    pub fn push_frame(&mut self, frame: F) {
        for layer in self.layers.iter_mut() {
            layer.push(frame);
        }
    }

    pub fn push_layer(&mut self) {
        let sample_rate = self.sample_rate;
        let g_weights = self.g_weights;

        let k_filter = KWeightFilter::new(sample_rate);
        let momentary_gl = self.calc_momentary.then(|| GatedLoudness::momentary(sample_rate, g_weights));
        let shortterm_gl = self.calc_shortterm.then(|| GatedLoudness::shortterm(sample_rate, g_weights));
        let custom_gl_map = self.custom_gatings
            .iter()
            .map(|g| (*g, GatedLoudness::new(sample_rate, g_weights, *g)))
            .collect::<HashMap<_, _>>();

        let new_layer = PipelineLayer {
            k_filter,
            momentary_gl,
            shortterm_gl,
            custom_gl_map,
        };

        self.layers.push(new_layer);
    }

    pub fn pop_layer(&mut self) -> Option<Output> {
        self.layers.pop().map(|l| l.calculate())
    }
}

pub struct Pipeline<F, const N: usize>
where
    F: Frame<N, Sample = f64>,
{
    upstream: Upstream<F, N>,

    k_filter: KWeightFilter<F, N>,
    momentary_gl: Option<GatedLoudness<F, N>>,
    shortterm_gl: Option<GatedLoudness<F, N>>,
    custom_gl_map: HashMap<Gating, GatedLoudness<F, N>>,
}

impl<F, const N: usize> Pipeline<F, N>
where
    F: Frame<N, Sample = f64>,
{
    pub fn is_noop(&self) -> bool {
        self.momentary_gl.is_none() && self.shortterm_gl.is_none() && self.custom_gl_map.is_empty()
    }

    pub fn feed<S>(&mut self, signal: S)
    where
        S: Signal<N, Frame = F>,
    {
        for frame in signal.into_iter() {
            self.push(frame);
        }
    }

    pub fn spawn_child(self) -> Self {
        let &Config { sample_rate, g_weights } = self.upstream.config();

        let k_filter = KWeightFilter::new(sample_rate);
        let momentary_gl = self.momentary_gl.is_some().then(|| {
            GatedLoudness::momentary(sample_rate, g_weights)
        });
        let shortterm_gl = self.shortterm_gl.is_some().then(|| {
            GatedLoudness::shortterm(sample_rate, g_weights)
        });

        let custom_gl_map = self.custom_gl_map.keys()
            .copied()
            .map(|gating| {
                (gating, GatedLoudness::new(sample_rate, g_weights, gating))
            })
            .collect()
        ;

        Self {
            k_filter,
            momentary_gl,
            shortterm_gl,
            custom_gl_map,

            upstream: Upstream::Parent(Box::new(self)),
        }
    }
}

impl<F, const N: usize> Calculator for Pipeline<F, N>
where
    F: Frame<N, Sample = f64>,
{
    type Input = F;
    type Output = (Output, Option<Pipeline<F, N>>);

    fn push(&mut self, input: Self::Input) {
        let filtered_frame = self.k_filter.process(input);

        self.momentary_gl.as_mut().map(|gl| gl.push(filtered_frame));
        self.shortterm_gl.as_mut().map(|gl| gl.push(filtered_frame));

        for gated_loudness in self.custom_gl_map.values_mut() {
            gated_loudness.push(filtered_frame);
        }

        // Push the frame upstream as well.
        self.upstream.push(input);
    }

    fn calculate(self) -> Self::Output {
        let output = Output {
            momentary_mean: self.momentary_gl.and_then(|gl| gl.calculate()),
            shortterm_mean: self.shortterm_gl.and_then(|gl| gl.calculate()),
            custom_gating_means: self.custom_gl_map.into_iter()
                .map(|(gating, gl)| {
                    (gating, gl.calculate())
                })
                .collect(),
        };

        let parent = self.upstream.into_parent();

        (output, parent)
    }
}

pub struct PipelineBuilder<F, const N: usize>
where
    F: Frame<N, Sample = f64>,
{
    sample_rate: u32,
    g_weights: F,
    calc_momentary: bool,
    calc_shortterm: bool,
    custom_gatings: HashSet<Gating>,
}

impl<F, const N: usize> PipelineBuilder<F, N>
where
    F: Frame<N, Sample = f64>,
{
    pub fn new(sample_rate: u32, g_weights: F) -> Self {
        Self {
            sample_rate,
            g_weights,
            calc_momentary: false,
            calc_shortterm: false,
            custom_gatings: HashSet::new(),
        }
    }

    #[inline]
    pub fn momentary(&mut self) -> &mut Self {
        self.calc_momentary = true;
        self
    }

    #[inline]
    pub fn shortterm(&mut self) -> &mut Self {
        self.calc_shortterm = true;
        self
    }

    #[inline]
    pub fn custom(&mut self, gate_len_ms: u64, delta_len_ms: u64) -> &mut Self {
        self.custom_gatings.insert(Gating::Custom { gate_len_ms, delta_len_ms });
        self
    }

    pub fn build(&mut self) -> Pipeline<F, N> {
        let sample_rate = self.sample_rate;
        let g_weights = self.g_weights;

        let upstream = Upstream::Source(Config { sample_rate, g_weights });
        let k_filter = KWeightFilter::new(sample_rate);
        let momentary_gl = self.calc_momentary.then(|| GatedLoudness::momentary(sample_rate, g_weights));
        let shortterm_gl = self.calc_shortterm.then(|| GatedLoudness::shortterm(sample_rate, g_weights));
        let custom_gl_map = self.custom_gatings.drain()
            .map(|g| (g, GatedLoudness::new(sample_rate, g_weights, g)))
            .collect::<HashMap<_, _>>();

        Pipeline {
            upstream,

            k_filter,
            momentary_gl,
            shortterm_gl,
            custom_gl_map,
        }
    }
}
