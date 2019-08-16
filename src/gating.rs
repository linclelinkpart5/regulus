
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

        // println!("{}, {:?}", msq.num_samples(), result);

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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::wave::WaveKind;
    use crate::wave::WaveGen;

    const CYCLE_LEN: usize = 128;

    #[derive(Default)]
    struct SampleIter(usize);

    impl SampleIter {
        fn new() -> Self {
            Default::default()
        }
    }

    impl Iterator for SampleIter {
        type Item = [f64; MAX_CHANNELS];

        fn next(&mut self) -> Option<Self::Item> {
            let x = (2.0 * self.0 as f64 / CYCLE_LEN as f64) - 1.0;
            self.0 = (self.0 + 1) % CYCLE_LEN;
            Some([-x, -x / 2.0, 0.0, x / 2.0, x])
        }
    }

    #[test]
    fn gated_mean_square_iter() {
        const FREQUENCIES: [u32; MAX_CHANNELS] = [440, 480, 520, 560, 600];

        let sample_iter = WaveGen::new(WaveKind::Sawtooth, 48000, FREQUENCIES);
        let mut gmsi = GatedMeanSquareIter::new(sample_iter, 48000);

        let expected_results = [
            [0.33333379629629456, 0.3334000000000141, 0.3333337962962951, 0.3333351851851806, 0.33343750000000755],
            [0.33333379629629417, 0.3334000000000152, 0.33333379629629406, 0.33333518518517913, 0.33343750000000566],
            [0.33333379629629306, 0.33340000000000763, 0.33333379629629295, 0.33333518518517596, 0.33343750000000544],
            [0.33333379629629234, 0.33340000000000103, 0.3333337962962912, 0.33333518518517324, 0.3334375000000025],
            [0.33333379629629073, 0.3334000000000033, 0.3333337962962899, 0.33333518518517247, 0.33343749999999983],
            [0.3333337962962897, 0.33340000000000347, 0.3333337962962904, 0.3333351851851727, 0.33343749999999456],
            [0.3333337962962901, 0.3334000000000038, 0.3333337962962902, 0.333335185185172, 0.3334374999999834],
            [0.33333379629629045, 0.33340000000000836, 0.3333337962962904, 0.3333351851851737, 0.33343749999998906],
        ];

        for expected in expected_results.iter() {
            let produced = gmsi.next().unwrap();

            for ch in 0..expected.len().max(produced.len()) {
                let e = expected[ch];
                let p = produced[ch];
                assert_abs_diff_eq!(e, p);
            }
        }

        let sample_iter = WaveGen::new(WaveKind::Square, 48000, FREQUENCIES);
        let mut gmsi = GatedMeanSquareIter::new(sample_iter, 48000);

        let expected_results = [
            [1.0, 1.0, 1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0, 1.0],
        ];

        for expected in expected_results.iter() {
            let produced = gmsi.next().unwrap();

            for ch in 0..expected.len().max(produced.len()) {
                let e = expected[ch];
                let p = produced[ch];
                assert_abs_diff_eq!(e, p);
            }
        }

        let sample_iter = WaveGen::new(WaveKind::Triangle, 48000, FREQUENCIES);
        let mut gmsi = GatedMeanSquareIter::new(sample_iter, 48000);

        let expected_results = [
            [0.3333351851851775, 0.33360000000000145, 0.33333518518517663, 0.33334074074073505, 0.33375000000000676],
            [0.33333518518517413, 0.33360000000000267, 0.33333518518517297, 0.33334074074073333, 0.33374999999999966],
            [0.33333518518517236, 0.3336000000000016, 0.3333351851851718, 0.33334074074072767, 0.3337499999999865],
            [0.33333518518517175, 0.33359999999999884, 0.33333518518517324, 0.3333407407407277, 0.3337499999999755],
            [0.33333518518517335, 0.33359999999998974, 0.3333351851851759, 0.33334074074073, 0.3337499999999757],
            [0.3333351851851765, 0.3335999999999949, 0.3333351851851782, 0.3333407407407311, 0.3337499999999655],
            [0.33333518518517824, 0.333599999999996, 0.33333518518517896, 0.33334074074074416, 0.33374999999994337],
            [0.33333518518517974, 0.3336000000000051, 0.3333351851851805, 0.33334074074075304, 0.33374999999997745],
        ];

        for expected in expected_results.iter() {
            let produced = gmsi.next().unwrap();

            for ch in 0..expected.len().max(produced.len()) {
                let e = expected[ch];
                let p = produced[ch];
                assert_abs_diff_eq!(e, p);
            }
        }

        // for _ in 0..8 {
        //     println!("{:?}", gmsi.next());
        // }
    }
}
