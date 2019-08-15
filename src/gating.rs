
use std::iter::FusedIterator;

use slice_deque::SliceDeque;

use crate::mean_sq::MeanSquare;
use crate::constants::MAX_CHANNELS;
use crate::util::Util;

const GATE_DELTA_MS: u64 = 100;
const GATE_FACTOR: u64 = 4;
const GATE_LENGTH_MS: u64 = GATE_DELTA_MS * GATE_FACTOR;

pub struct GatedMeanSquareIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    sample_iter: I,
    // sample_rate: u32,
    samples_per_delta: usize,
    samples_per_gate: usize,
    ring_buffer: SliceDeque<[f64; MAX_CHANNELS]>,
    initialized: bool,
}

impl<I> GatedMeanSquareIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    pub fn new(sample_iter: I, sample_rate: u32) -> Self {
        let samples_per_delta = Util::ms_to_samples(GATE_DELTA_MS, sample_rate, false) as usize;
        let samples_per_gate = Util::ms_to_samples(GATE_LENGTH_MS, sample_rate, false) as usize;

        let ring_buffer = SliceDeque::with_capacity(samples_per_gate);

        Self {
            sample_iter,
            samples_per_delta,
            samples_per_gate,
            ring_buffer,
            initialized: false,
        }
    }
}

impl<I> Iterator for GatedMeanSquareIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let mut msq = MeanSquare::new();

        let samples_to_take = if !self.initialized {
            // Set the initialized flag and take an entire gate's worth of samples.
            self.initialized = true;
            self.samples_per_gate
        } else {
            // Take a delta's worth of samples.
            self.samples_per_delta
        };

        for _ in 0..samples_to_take {
            // If this returns `None`, return `None` for this entire call.
            let sample = self.sample_iter.next()?;

            // Push the new sample into the ring buffer, cycling out old samples if needed.
            while self.ring_buffer.len() >= self.samples_per_gate {
                self.ring_buffer.pop_front();
            }
            self.ring_buffer.push_back(sample);
        }

        // At this point the buffer should be filled to capacity.
        assert_eq!(self.samples_per_gate, self.ring_buffer.len());

        // Calculate the mean squares of the current ring buffer.
        msq.add_samples(self.ring_buffer.as_slice());
        let result = msq.mean_sqs();

        Some(result)
    }
}

impl<I> FusedIterator for GatedMeanSquareIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{}

pub struct GatedLoudnessIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    gmsi: GatedMeanSquareIter<I>,

    // Denoted as the `G` weights in the tech doc.
    channel_weights: [f64; MAX_CHANNELS],
}

impl<I> Iterator for GatedLoudnessIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        let mean_sq = self.gmsi.next()?;

        let mut sum = 0.0;
        for ch in 0..MAX_CHANNELS {
            sum += self.channel_weights[ch] * mean_sq[ch];
        }

        let loudness = -0.691 + 10.0 * sum.log10();

        Some(loudness)
    }
}
