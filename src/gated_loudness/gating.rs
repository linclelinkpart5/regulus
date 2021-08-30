use sampara::{Frame, StatefulProcessor, Processor};
use sampara::stats::BufferedMovingMs;
use sampara::sample::FloatSample;

use crate::util::Util;

const MOMENTARY_DELTA_MS: u64 = 100;
const MOMENTARY_GATE_MS: u64 = 400;
const SHORTTERM_DELTA_MS: u64 = 3000;
const SHORTTERM_GATE_MS: u64 = 1000;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Gating {
    pub gate_len_ms: u64,
    pub delta_len_ms: u64,
}

pub const MOMENTARY_GATING: Gating = Gating {
    gate_len_ms: MOMENTARY_GATE_MS,
    delta_len_ms: MOMENTARY_DELTA_MS,
};
pub const SHORTTERM_GATING: Gating = Gating {
    gate_len_ms: SHORTTERM_GATE_MS,
    delta_len_ms: SHORTTERM_DELTA_MS,
};

pub struct GatedPowers<F, const N: usize>
where
    F: Frame<N>,
    F::Sample: FloatSample,
{
    ms_state: BufferedMovingMs<Vec<F>, N>,
    i: usize,
    delta: usize,
}

impl<F, const N: usize> GatedPowers<F, N>
where
    F: Frame<N>,
    F::Sample: FloatSample,
{
    pub fn custom(gating: Gating, sample_rate: u32) -> Self {
        // Calculate the gate length, in frames.
        // This will in turn determine the length of the mean squares buffer.
        let gate_buffer_len = Util::ms_to_samples(gating.gate_len_ms, sample_rate) as usize;

        // Number of frames to add at a time for each iteration after the first.
        // This is the number of steps to advance the mean squares iterator for
        // each iteration (i.e. the "step-by" amount).
        let frames_per_delta = Util::ms_to_samples(gating.delta_len_ms, sample_rate) as usize;

        assert!(frames_per_delta > 0);

        let buffer = vec![Frame::EQUILIBRIUM; gate_buffer_len];

        let ms_state = BufferedMovingMs::from(buffer);

        Self {
            ms_state,
            i: usize::MAX,
            delta: frames_per_delta,
        }
    }

    pub fn momentary(sample_rate: u32) -> Self {
        Self::custom(MOMENTARY_GATING, sample_rate)
    }

    pub fn shortterm(sample_rate: u32) -> Self {
        Self::custom(SHORTTERM_GATING, sample_rate)
    }

    pub fn process(&mut self, input: F) -> Option<F> {
        Processor::process(self, input)
    }
}

impl<F, const N: usize> StatefulProcessor for GatedPowers<F, N>
where
    F: Frame<N>,
    F::Sample: FloatSample,
{
    type Input = F;
    type Output = Option<F>;

    fn advance(&mut self, input: Self::Input) {
        let was_active = self.ms_state.is_active();
        self.ms_state.advance(input);
        let now_active = self.ms_state.is_active();

        if now_active {
            if was_active {
                self.i = (self.i + 1) % self.delta;
            }
            else {
                self.i = 0;
            }
        }
    }

    fn current(&self) -> Self::Output {
        // We only want to output the frame when a new delta is starting.
        if self.i == 0 {
            self.ms_state.current()
        }
        else {
            None
        }
    }
}


#[cfg(test)]
mod tests {
    // use super::*;

    // #[test]
    // fn gated_power_iter() {
    //     const FREQUENCIES: [f64; MAX_CHANNELS] = [440.0, 480.0, 520.0, 560.0, 600.0];
    //     const AMPLITUDES: [f64; MAX_CHANNELS] = [1.0, 1.0, 1.0, 1.0, 1.0];

    //     let sample_iter = WaveGen::new(WaveKind::Sawtooth, 48000, FREQUENCIES, AMPLITUDES);
    //     let mut gpi = GatedPowers::new(sample_iter, 48000);

    //     let expected_results = [
    //         // [0.33333379629629456, 0.3334000000000141, 0.3333337962962951, 0.3333351851851806, 0.33343750000000755],
    //         [0.3333337962962942, 0.333400000000007, 0.333333796296294, 0.33333518518517746, 0.33343750000000816],
    //         // [0.33333379629629417, 0.3334000000000152, 0.33333379629629406, 0.33333518518517913, 0.33343750000000566],
    //         [0.3333337962962929, 0.333400000000007, 0.3333337962962926, 0.3333351851851782, 0.333437500000005],
    //         // [0.33333379629629306, 0.33340000000000763, 0.33333379629629295, 0.33333518518517596, 0.33343750000000544],
    //         [0.3333337962962926, 0.33340000000001024, 0.3333337962962934, 0.3333351851851788, 0.3334375000000021],
    //         // [0.33333379629629234, 0.33340000000000103, 0.3333337962962912, 0.33333518518517324, 0.3334375000000025],
    //         [0.3333337962962934, 0.33340000000001485, 0.33333379629629206, 0.33333518518517635, 0.33343749999999983],
    //         // [0.33333379629629073, 0.3334000000000033, 0.3333337962962899, 0.33333518518517247, 0.33343749999999983],
    //         [0.33333379629629134, 0.33340000000001596, 0.33333379629629084, 0.3333351851851741, 0.33343749999999983],
    //         // [0.3333337962962897, 0.33340000000000347, 0.3333337962962904, 0.3333351851851727, 0.33343749999999456],
    //         [0.33333379629629045, 0.33340000000000686, 0.3333337962962904, 0.3333351851851727, 0.33343749999999983],
    //         // [0.3333337962962901, 0.3334000000000038, 0.3333337962962902, 0.333335185185172, 0.3334374999999834],
    //         [0.3333337962962901, 0.3334000000000038, 0.3333337962962903, 0.33333518518517463, 0.3334374999999951],
    //         // [0.33333379629629045, 0.33340000000000836, 0.3333337962962904, 0.3333351851851737, 0.33343749999998906],
    //         [0.33333379629628995, 0.33340000000000425, 0.33333379629629056, 0.3333351851851753, 0.3334374999999927],
    //     ];

    //     for expected in expected_results.iter() {
    //         let produced = gpi.next().unwrap();

    //         for ch in 0..expected.len().max(produced.len()) {
    //             let e = expected[ch];
    //             let p = produced[ch];
    //             assert_abs_diff_eq!(e, p);
    //         }
    //     }

    //     let sample_iter = WaveGen::new(WaveKind::Square, 48000, FREQUENCIES, AMPLITUDES);
    //     let mut gpi = GatedPowers::new(sample_iter, 48000);

    //     let expected_results = [
    //         [1.0, 1.0, 1.0, 1.0, 1.0],
    //         [1.0, 1.0, 1.0, 1.0, 1.0],
    //         [1.0, 1.0, 1.0, 1.0, 1.0],
    //         [1.0, 1.0, 1.0, 1.0, 1.0],
    //         [1.0, 1.0, 1.0, 1.0, 1.0],
    //         [1.0, 1.0, 1.0, 1.0, 1.0],
    //         [1.0, 1.0, 1.0, 1.0, 1.0],
    //         [1.0, 1.0, 1.0, 1.0, 1.0],
    //     ];

    //     for expected in expected_results.iter() {
    //         let produced = gpi.next().unwrap();

    //         for ch in 0..expected.len().max(produced.len()) {
    //             let e = expected[ch];
    //             let p = produced[ch];
    //             assert_abs_diff_eq!(e, p);
    //         }
    //     }

    //     let sample_iter = WaveGen::new(WaveKind::Triangle, 48000, FREQUENCIES, AMPLITUDES);
    //     let mut gpi = GatedPowers::new(sample_iter, 48000);

    //     let expected_results = [
    //         // [0.3333351851851775, 0.33360000000000145, 0.33333518518517663, 0.33334074074073505, 0.33375000000000676],
    //         [0.3333351851851775, 0.33360000000000845, 0.33333518518517713, 0.33334074074074194, 0.33375000000000643],
    //         // [0.33333518518517413, 0.33360000000000267, 0.33333518518517297, 0.33334074074073333, 0.33374999999999966],
    //         [0.33333518518517685, 0.333600000000013, 0.33333518518517596, 0.33334074074074466, 0.33375000000000715],
    //         // [0.33333518518517236, 0.3336000000000016, 0.3333351851851718, 0.33334074074072767, 0.3337499999999865],
    //         [0.3333351851851748, 0.333600000000011, 0.33333518518517663, 0.33334074074074155, 0.33375000000000193],
    //         // [0.33333518518517175, 0.33359999999999884, 0.33333518518517324, 0.3333407407407277, 0.3337499999999755],
    //         [0.3333351851851759, 0.33360000000000417, 0.33333518518517824, 0.3333407407407399, 0.33374999999999055],
    //         // [0.33333518518517335, 0.33359999999998974, 0.3333351851851759, 0.33334074074073, 0.3337499999999757],
    //         [0.3333351851851784, 0.33359999999999596, 0.3333351851851788, 0.33334074074073455, 0.33374999999997923],
    //         // [0.3333351851851765, 0.3335999999999949, 0.3333351851851782, 0.3333407407407311, 0.3337499999999655],
    //         [0.3333351851851788, 0.33359999999999596, 0.3333351851851782, 0.3333407407407311, 0.3337499999999756],
    //         // [0.33333518518517824, 0.333599999999996, 0.33333518518517896, 0.33334074074074416, 0.33374999999994337],
    //         [0.33333518518517824, 0.333599999999996, 0.33333518518517763, 0.3333407407407332, 0.3337499999999766],
    //         // [0.33333518518517974, 0.3336000000000051, 0.3333351851851805, 0.33334074074075304, 0.33374999999997745],
    //         [0.3333351851851758, 0.333599999999992, 0.33333518518517496, 0.3333407407407357, 0.333749999999976],
    //     ];

    //     for expected in expected_results.iter() {
    //         let produced = gpi.next().unwrap();

    //         for ch in 0..expected.len().max(produced.len()) {
    //             let e = expected[ch];
    //             let p = produced[ch];
    //             assert_abs_diff_eq!(e, p);
    //         }
    //     }

    //     let sample_iter = WaveGen::new(WaveKind::Sine, 48000, FREQUENCIES, AMPLITUDES);
    //     let mut gpi = GatedPowers::new(sample_iter, 48000);

    //     let expected_results = [
    //         // [0.4999999999999992, 0.5, 0.4999999999999989, 0.5000000000000002, 0.5000000000000003],
    //         [0.4999999999999992, 0.5000000000000004, 0.5000000000000007, 0.5000000000000003, 0.4999999999999999],
    //         // [0.49999999999999906, 0.4999999999999999, 0.4999999999999981, 0.5000000000000001, 0.4999999999999999],
    //         [0.4999999999999992, 0.4999999999999996, 0.4999999999999995, 0.5000000000000008, 0.5000000000000001],
    //         // [0.4999999999999995, 0.4999999999999995, 0.5000000000000027, 0.49999999999999983, 0.5000000000000002],
    //         [0.4999999999999992, 0.49999999999999933, 0.5000000000000006, 0.4999999999999995, 0.4999999999999999],
    //         // [0.49999999999999917, 0.49999999999999983, 0.5000000000000017, 0.5000000000000006, 0.5000000000000001],
    //         [0.49999999999999906, 0.4999999999999999, 0.5000000000000002, 0.5000000000000003, 0.49999999999999983],
    //         // [0.49999999999999944, 0.4999999999999999, 0.5000000000000011, 0.5000000000000016, 0.5000000000000002],
    //         [0.49999999999999983, 0.5, 0.5000000000000017, 0.5000000000000006, 0.5000000000000001],
    //         // [0.49999999999999944, 0.49999999999999983, 0.5000000000000019, 0.5000000000000004, 0.5000000000000003],
    //         [0.4999999999999992, 0.5000000000000003, 0.5000000000000016, 0.5000000000000013, 0.5000000000000001],
    //         // [0.49999999999999917, 0.5000000000000001, 0.5000000000000012, 0.5000000000000002, 0.5000000000000003],
    //         [0.49999999999999933, 0.4999999999999999, 0.5000000000000009, 0.5000000000000013, 0.4999999999999999],
    //         // [0.49999999999999917, 0.49999999999999944, 0.4999999999999999, 0.5000000000000003, 0.4999999999999996],
    //         [0.4999999999999999, 0.5000000000000001, 0.5000000000000003, 0.5000000000000009, 0.49999999999999983],
    //     ];

    //     for expected in expected_results.iter() {
    //         let produced = gpi.next().unwrap();

    //         for ch in 0..expected.len().max(produced.len()) {
    //             let e = expected[ch];
    //             let p = produced[ch];
    //             assert_abs_diff_eq!(e, p);
    //         }
    //     }

    //     // for _ in 0..8 {
    //     //     println!("{:?}", gpi.next());
    //     // }

    //     // return;
    // }
}
