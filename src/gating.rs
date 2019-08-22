
use std::iter::FusedIterator;

use slice_deque::SliceDeque;

use crate::constants::MAX_CHANNELS;
use crate::util::Util;
use crate::stats::Stats;

const GATE_DELTA_MS: u64 = 100;
const GATE_FACTOR: u64 = 4;
const GATE_LENGTH_MS: u64 = GATE_DELTA_MS * GATE_FACTOR;
const ABSOLUTE_LOUDNESS_THRESHOLD: f64 = -70.0;

pub struct GatedPowerIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    sample_iter: I,
    samples_per_delta: usize,
    samples_per_gate: usize,
    ring_buffer: SliceDeque<[f64; MAX_CHANNELS]>,
    initialized: bool,
}

impl<I> GatedPowerIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    pub fn new(sample_iter: I, sample_rate: u32) -> Self {
        let samples_per_delta = Util::ms_to_samples(GATE_DELTA_MS, sample_rate) as usize;
        let samples_per_gate = Util::ms_to_samples(GATE_LENGTH_MS, sample_rate) as usize;

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

impl<I> Iterator for GatedPowerIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
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
        let mut channel_powers = [0.0f64; MAX_CHANNELS];
        for ch in 0..MAX_CHANNELS {
            let mut channel_energy = 0.0;
            for sample in &self.ring_buffer {
                channel_energy += sample[ch] * sample[ch];
            }

            channel_powers[ch] = channel_energy / self.samples_per_gate as f64;
        }

        Some(channel_powers)
    }
}

impl<I> FusedIterator for GatedPowerIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{}

pub struct GatedLoudnessIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    gpi: GatedPowerIter<I>,

    // Denoted as the `G` weights in the tech doc.
    channel_weights: [f64; MAX_CHANNELS],

    averager: Stats,
    absolutely_loud_blocks: Vec<(f64, [f64; MAX_CHANNELS])>,
}

impl<I> GatedLoudnessIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    pub fn new(sample_iter: I, sample_rate: u32, channel_weights: [f64; MAX_CHANNELS]) -> Self {
        let gpi = GatedPowerIter::new(sample_iter, sample_rate);

        Self {
            gpi,
            channel_weights,
            averager: Stats::new(),
            absolutely_loud_blocks: Vec::new(),
        }
    }
}

impl<I> GatedLoudnessIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    pub fn absolute_loudness(&self) -> f64 {
        // This performs the calculation done in equation #5 in the ITU BS.1770 tech spec.
        // This is the loudness of the average of the per-channel power of blocks that were marked as "loud"
        // (i.e. the loudness of that block was above the absolute loudness threshold) during the initial pass.
        Util::block_loudness(&self.averager.mean, &self.channel_weights)
    }

    #[inline]
    pub fn relative_loudness_threshold(&self) -> f64 {
        // This performs the calculation done in equation #6 in the ITU BS.1770 tech spec.
        // The relative loudness threshold is the absolute loudness minus 10.0.
        self.absolute_loudness() - 10.0
    }

    pub fn relative_loudness(&self) -> f64 {
        // This performs the calculation done in equation #7 in the ITU BS.1770 tech spec.
        // From the collected of saved blocks that were marked as "absolutely loud",
        // only those that exceed the relative loudness threshold need to be selected and averaged.
        let mut relative_averager = Stats::new();

        let relative_loudness_threshold = self.relative_loudness_threshold();

        for (block_loudness, channel_powers) in &self.absolutely_loud_blocks {
            // These blocks are already known to be above the absolute loudness threshold.
            // However they also need to be over the relative loudness threshold for this calculation.
            if block_loudness > &relative_loudness_threshold {
                relative_averager.add(channel_powers)
            }
        }

        let relative_loudness = Util::block_loudness(&relative_averager.mean, &self.channel_weights);

        relative_loudness
    }
}

impl<I> Iterator for GatedLoudnessIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        let channel_powers = self.gpi.next()?;
        let block_loudness = Util::block_loudness(&channel_powers, &self.channel_weights);

        // If the block loudness is greater than the absolute loudness threshold, save the channel powers.
        if block_loudness > ABSOLUTE_LOUDNESS_THRESHOLD {
            self.averager.add(&channel_powers);
            self.absolutely_loud_blocks.push((block_loudness, channel_powers))
        }

        Some(block_loudness)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::wave::WaveKind;
    use crate::wave::WaveGen;

    #[test]
    fn gated_power_iter() {
        const FREQUENCIES: [u32; MAX_CHANNELS] = [440, 480, 520, 560, 600];

        let sample_iter = WaveGen::new(WaveKind::Sawtooth, 48000, FREQUENCIES);
        let mut gpi = GatedPowerIter::new(sample_iter, 48000);

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
            let produced = gpi.next().unwrap();

            for ch in 0..expected.len().max(produced.len()) {
                let e = expected[ch];
                let p = produced[ch];
                assert_abs_diff_eq!(e, p);
            }
        }

        let sample_iter = WaveGen::new(WaveKind::Square, 48000, FREQUENCIES);
        let mut gpi = GatedPowerIter::new(sample_iter, 48000);

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
            let produced = gpi.next().unwrap();

            for ch in 0..expected.len().max(produced.len()) {
                let e = expected[ch];
                let p = produced[ch];
                assert_abs_diff_eq!(e, p);
            }
        }

        let sample_iter = WaveGen::new(WaveKind::Triangle, 48000, FREQUENCIES);
        let mut gpi = GatedPowerIter::new(sample_iter, 48000);

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
            let produced = gpi.next().unwrap();

            for ch in 0..expected.len().max(produced.len()) {
                let e = expected[ch];
                let p = produced[ch];
                assert_abs_diff_eq!(e, p);
            }
        }

        let sample_iter = WaveGen::new(WaveKind::Sine, 48000, FREQUENCIES);
        let mut gpi = GatedPowerIter::new(sample_iter, 48000);

        let expected_results = [
            [0.4999999999999992, 0.5, 0.4999999999999989, 0.5000000000000002, 0.5000000000000003],
            [0.49999999999999906, 0.4999999999999999, 0.4999999999999981, 0.5000000000000001, 0.4999999999999999],
            [0.4999999999999995, 0.4999999999999995, 0.5000000000000027, 0.49999999999999983, 0.5000000000000002],
            [0.49999999999999917, 0.49999999999999983, 0.5000000000000017, 0.5000000000000006, 0.5000000000000001],
            [0.49999999999999944, 0.4999999999999999, 0.5000000000000011, 0.5000000000000016, 0.5000000000000002],
            [0.49999999999999944, 0.49999999999999983, 0.5000000000000019, 0.5000000000000004, 0.5000000000000003],
            [0.49999999999999917, 0.5000000000000001, 0.5000000000000012, 0.5000000000000002, 0.5000000000000003],
            [0.49999999999999917, 0.49999999999999944, 0.4999999999999999, 0.5000000000000003, 0.4999999999999996],
        ];

        for expected in expected_results.iter() {
            let produced = gpi.next().unwrap();

            for ch in 0..expected.len().max(produced.len()) {
                let e = expected[ch];
                let p = produced[ch];
                assert_abs_diff_eq!(e, p);
            }
        }

        // for _ in 0..8 {
        //     println!("{:?}", gpi.next());
        // }

        // return;
    }

    #[test]
    fn gated_loudness_iter() {
        const FREQUENCIES: [u32; MAX_CHANNELS] = [440, 480, 520, 560, 600];
        const CHANNEL_WEIGHTS: [f64; MAX_CHANNELS] = [0.8, 0.9, 1.0, 1.1, 1.2];

        let sample_iter = WaveGen::new(WaveKind::Sawtooth, 48000, FREQUENCIES);
        let mut gli = GatedLoudnessIter::new(sample_iter, 48000, CHANNEL_WEIGHTS);

        let expected_results = [
            1.527977014965285,
            1.5279770149652738,
            1.5279770149652414,
            1.5279770149652028,
            1.5279770149651917,
            1.5279770149651748,
            1.5279770149651393,
            1.5279770149651726,
        ];

        for expected in expected_results.iter() {
            let produced = gli.next().unwrap();
            assert_abs_diff_eq!(*expected, produced);
        }

        let sample_iter = WaveGen::new(WaveKind::Square, 48000, FREQUENCIES);
        let mut gli = GatedLoudnessIter::new(sample_iter, 48000, CHANNEL_WEIGHTS);

        let expected_results = [
            6.2987000433601885,
            6.2987000433601885,
            6.2987000433601885,
            6.2987000433601885,
            6.2987000433601885,
            6.2987000433601885,
            6.2987000433601885,
            6.2987000433601885,
        ];

        for expected in expected_results.iter() {
            let produced = gli.next().unwrap();
            assert_abs_diff_eq!(*expected, produced);
        }

        let sample_iter = WaveGen::new(WaveKind::Triangle, 48000, FREQUENCIES);
        let mut gli = GatedLoudnessIter::new(sample_iter, 48000, CHANNEL_WEIGHTS);

        let expected_results = [
            1.5294452403980916,
            1.5294452403980507,
            1.5294452403979846,
            1.5294452403979464,
            1.5294452403979424,
            1.529445240397938,
            1.5294452403979144,
            1.5294452403980756,
        ];

        for expected in expected_results.iter() {
            let produced = gli.next().unwrap();
            assert_abs_diff_eq!(*expected, produced);
        }

        let sample_iter = WaveGen::new(WaveKind::Sine, 48000, FREQUENCIES);
        let mut gli = GatedLoudnessIter::new(sample_iter, 48000, CHANNEL_WEIGHTS);

        let expected_results = [
            3.2884000867203746,
            3.2884000867203715,
            3.2884000867203795,
            3.2884000867203795,
            3.28840008672038,
            3.28840008672038,
            3.2884000867203786,
            3.288400086720374,
        ];

        for expected in expected_results.iter() {
            let produced = gli.next().unwrap();
            assert_abs_diff_eq!(*expected, produced);
        }

        // for _ in 0..8 {
        //     println!("{:?}", gli.next());
        // }

        // return;
    }
}
