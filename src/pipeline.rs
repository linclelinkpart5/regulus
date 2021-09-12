use std::collections::{HashMap, HashSet};

use sampara::{Frame, Calculator, Signal};

use crate::filter::KWeightFilter;
use crate::gated_loudness::{Gating, GatedLoudness};

pub struct Output {
    averages: HashMap<Gating, Option<f64>>,
    maximas: HashMap<Gating, Option<f64>>,
}

pub struct PipelineLayer<F, const N: usize>
where
    F: Frame<N, Sample = f64>,
{
    k_filter: KWeightFilter<F, N>,
    gl_average_map: HashMap<Gating, GatedLoudness<F, N>>,
    gl_maxima_map: HashMap<Gating, GatedLoudness<F, N>>,
}

impl<F, const N: usize> PipelineLayer<F, N>
where
    F: Frame<N, Sample = f64>,
{
    pub fn is_noop(&self) -> bool {
        self.gl_average_map.is_empty() && self.gl_maxima_map.is_empty()
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

        for gated_loudness in self.gl_average_map.values_mut() {
            gated_loudness.push(filtered_frame);
        }

        for gated_loudness in self.gl_maxima_map.values_mut() {
            gated_loudness.push(filtered_frame);
        }
    }

    pub fn calculate(self) -> Output {
        let averages = self.gl_average_map.into_iter()
            .map(|(gating, gl)| {
                (gating, gl.calculate())
            })
            .collect();

        let maximas = self.gl_maxima_map.into_iter()
            .map(|(gating, gl)| {
                (gating, gl.calculate())
            })
            .collect();

        Output {
            averages,
            maximas,
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

    // Gatings to calculate for both averages and maximas.
    average_gatings: HashSet<Gating>,
    maxima_gatings: HashSet<Gating>,

    root_layer: PipelineLayer<F, N>,

    // The stack of child layers, from closest to furthest from the root layer.
    child_layers: Vec<PipelineLayer<F, N>>,
}

impl<F, const N: usize> Pipeline<F, N>
where
    F: Frame<N, Sample = f64>,
{
    pub fn push(&mut self, frame: F) {
        self.root_layer.push(frame);
        for layer in self.child_layers.iter_mut() {
            layer.push(frame);
        }
    }

    pub fn feed<I>(&mut self, frames: I)
    where
        I: IntoIterator<Item = F>,
    {
        for frame in frames {
            self.push(frame);
        }
    }

    pub fn process_track<I>(&mut self, frames: I) -> Output
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

    fn create_layer(&self) -> PipelineLayer<F, N> {
        let sample_rate = self.sample_rate;
        let g_weights = self.g_weights;

        let k_filter = KWeightFilter::new(sample_rate);

        let gl_average_map = self.average_gatings
            .iter()
            .map(|g| (*g, GatedLoudness::new(sample_rate, g_weights, *g)))
            .collect::<HashMap<_, _>>();
        let gl_maxima_map = self.maxima_gatings
            .iter()
            .map(|g| (*g, GatedLoudness::new(sample_rate, g_weights, *g)))
            .collect::<HashMap<_, _>>();

        PipelineLayer {
            k_filter,
            gl_average_map,
            gl_maxima_map,
        }
    }

    pub fn add_layer(&mut self) {
        let new_layer = self.create_layer();
        self.child_layers.push(new_layer);
    }

    pub fn calculate(mut self) -> (Output, Option<Self>) {
        if let Some(child_layer) = self.child_layers.pop() {
            (child_layer.calculate(), Some(self))
        }
        else {
            (self.root_layer.calculate(), None)
        }
    }
}

impl<F, const N: usize> Calculator for Pipeline<F, N>
where
    F: Frame<N, Sample = f64>,
{
    type Input = F;
    type Output = (Output, Option<Self>);

    fn push(&mut self, input: Self::Input) {
        self.push(input)
    }

    fn calculate(self) -> Self::Output {
        self.calculate()
    }
}
