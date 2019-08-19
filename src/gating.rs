
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

impl<I> GatedLoudnessIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    pub fn new(sample_iter: I, sample_rate: u32, channel_weights: [f64; MAX_CHANNELS]) -> Self {
        let gmsi = GatedMeanSquareIter::new(sample_iter, sample_rate);

        Self {
            gmsi,
            channel_weights,
        }
    }

    fn next_unsummed(&mut self) -> Option<[f64; MAX_CHANNELS]> {
        let mean_sq = self.gmsi.next()?;
        let mut result = [0.0; MAX_CHANNELS];

        for ch in 0..MAX_CHANNELS {
            result[ch] = mean_sq[ch] * self.channel_weights[ch];
        }

        Some(result)
    }
}

impl<I> Iterator for GatedLoudnessIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        let weighted = self.next_unsummed()?;

        let mut sum = 0.0;
        for ch in 0..MAX_CHANNELS {
            sum += weighted[ch];
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

    // const CYCLE_LEN: usize = 128;

    // #[derive(Default)]
    // struct SampleIter(usize);

    // impl SampleIter {
    //     fn new() -> Self {
    //         Default::default()
    //     }
    // }

    // impl Iterator for SampleIter {
    //     type Item = [f64; MAX_CHANNELS];

    //     fn next(&mut self) -> Option<Self::Item> {
    //         let x = (2.0 * self.0 as f64 / CYCLE_LEN as f64) - 1.0;
    //         self.0 = (self.0 + 1) % CYCLE_LEN;
    //         Some([-x, -x / 2.0, 0.0, x / 2.0, x])
    //     }
    // }

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

        let sample_iter = WaveGen::new(WaveKind::Sine, 48000, FREQUENCIES);
        let mut gmsi = GatedMeanSquareIter::new(sample_iter, 48000);

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

        // return;
    }

    #[test]
    fn gated_loudness_iter_next_unsummed() {
        const FREQUENCIES: [u32; MAX_CHANNELS] = [440, 480, 520, 560, 600];
        const CHANNEL_WEIGHTS: [f64; MAX_CHANNELS] = [0.8, 0.9, 1.0, 1.1, 1.2];

        let sample_iter = WaveGen::new(WaveKind::Sawtooth, 48000, FREQUENCIES);
        let mut gli = GatedLoudnessIter::new(sample_iter, 48000, CHANNEL_WEIGHTS);

        let expected_results = [
            [0.26666703703703565, 0.30006000000001265, 0.3333337962962951, 0.3666687037036987, 0.40012500000000906],
            [0.26666703703703537, 0.30006000000001365, 0.33333379629629406, 0.3666687037036971, 0.4001250000000068],
            [0.2666670370370345, 0.3000600000000069, 0.33333379629629295, 0.3666687037036936, 0.4001250000000065],
            [0.26666703703703387, 0.30006000000000094, 0.3333337962962912, 0.3666687037036906, 0.400125000000003],
            [0.2666670370370326, 0.300060000000003, 0.3333337962962899, 0.36666870370368976, 0.4001249999999998],
            [0.26666703703703176, 0.30006000000000316, 0.3333337962962904, 0.36666870370369, 0.40012499999999346],
            [0.2666670370370321, 0.30006000000000344, 0.3333337962962902, 0.36666870370368926, 0.4001249999999801],
            [0.2666670370370324, 0.30006000000000754, 0.3333337962962904, 0.3666687037036911, 0.40012499999998685],
        ];

        for expected in expected_results.iter() {
            let produced = gli.next_unsummed().unwrap();
            for ch in 0..expected.len().max(produced.len()) {
                let e = expected[ch];
                let p = produced[ch];
                assert_abs_diff_eq!(e, p);
            }
        }

        let sample_iter = WaveGen::new(WaveKind::Square, 48000, FREQUENCIES);
        let mut gli = GatedLoudnessIter::new(sample_iter, 48000, CHANNEL_WEIGHTS);

        let expected_results = [
            [0.8, 0.9, 1.0, 1.1, 1.2],
            [0.8, 0.9, 1.0, 1.1, 1.2],
            [0.8, 0.9, 1.0, 1.1, 1.2],
            [0.8, 0.9, 1.0, 1.1, 1.2],
            [0.8, 0.9, 1.0, 1.1, 1.2],
            [0.8, 0.9, 1.0, 1.1, 1.2],
            [0.8, 0.9, 1.0, 1.1, 1.2],
            [0.8, 0.9, 1.0, 1.1, 1.2],
        ];

        for expected in expected_results.iter() {
            let produced = gli.next_unsummed().unwrap();
            for ch in 0..expected.len().max(produced.len()) {
                let e = expected[ch];
                let p = produced[ch];
                assert_abs_diff_eq!(e, p);
            }
        }

        let sample_iter = WaveGen::new(WaveKind::Triangle, 48000, FREQUENCIES);
        let mut gli = GatedLoudnessIter::new(sample_iter, 48000, CHANNEL_WEIGHTS);

        let expected_results = [
            [0.26666814814814205, 0.30024000000000134, 0.33333518518517663, 0.36667481481480857, 0.4005000000000081],
            [0.26666814814813933, 0.3002400000000024, 0.33333518518517297, 0.3666748148148067, 0.4004999999999996],
            [0.2666681481481379, 0.30024000000000145, 0.3333351851851718, 0.36667481481480046, 0.4004999999999838],
            [0.2666681481481374, 0.30023999999999895, 0.33333518518517324, 0.3666748148148005, 0.4004999999999706],
            [0.2666681481481387, 0.3002399999999908, 0.3333351851851759, 0.366674814814803, 0.4004999999999708],
            [0.2666681481481412, 0.3002399999999954, 0.3333351851851782, 0.36667481481480424, 0.4004999999999586],
            [0.2666681481481426, 0.3002399999999964, 0.33333518518517896, 0.3666748148148186, 0.400499999999932],
            [0.2666681481481438, 0.3002400000000046, 0.3333351851851805, 0.3666748148148284, 0.40049999999997293],
        ];

        for expected in expected_results.iter() {
            let produced = gli.next_unsummed().unwrap();
            for ch in 0..expected.len().max(produced.len()) {
                let e = expected[ch];
                let p = produced[ch];
                assert_abs_diff_eq!(e, p);
            }
        }

        let sample_iter = WaveGen::new(WaveKind::Sine, 48000, FREQUENCIES);
        let mut gli = GatedLoudnessIter::new(sample_iter, 48000, CHANNEL_WEIGHTS);

        let expected_results = [
            [0.3999999999999994, 0.45, 0.4999999999999989, 0.5500000000000003, 0.6000000000000004],
            [0.39999999999999925, 0.4499999999999999, 0.4999999999999981, 0.5500000000000002, 0.5999999999999999],
            [0.39999999999999963, 0.44999999999999957, 0.5000000000000027, 0.5499999999999998, 0.6000000000000002],
            [0.39999999999999936, 0.44999999999999984, 0.5000000000000017, 0.5500000000000007, 0.6000000000000001],
            [0.3999999999999996, 0.4499999999999999, 0.5000000000000011, 0.5500000000000017, 0.6000000000000002],
            [0.3999999999999996, 0.44999999999999984, 0.5000000000000019, 0.5500000000000005, 0.6000000000000004],
            [0.39999999999999936, 0.4500000000000001, 0.5000000000000012, 0.5500000000000003, 0.6000000000000004],
            [0.39999999999999936, 0.4499999999999995, 0.4999999999999999, 0.5500000000000004, 0.5999999999999995],
        ];

        for expected in expected_results.iter() {
            let produced = gli.next_unsummed().unwrap();
            for ch in 0..expected.len().max(produced.len()) {
                let e = expected[ch];
                let p = produced[ch];
                assert_abs_diff_eq!(e, p);
            }
        }

        // for _ in 0..8 {
        //     println!("{:?}", gli.next_unsummed());
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
