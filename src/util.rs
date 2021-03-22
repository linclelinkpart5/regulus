use sampara::{Frame, Signal};

use crate::stats::Stats;

const DEN_THRESHOLD: f64 = 1.0e-15;

pub struct Util;

impl Util {
    #[inline]
    pub fn lufs(x: f64) -> f64 {
        -0.691 + 10.0 * x.log10()
    }

    /// Given the mean squares (powers) of an input signal and a set of
    /// per-channel weights, calculates the weighted loudness across all input
    /// channels. This is equation #4 in the ITU BS.1770 tech spec.
    pub fn loudness<F, const N: usize>(mean_sq: F, weights: F::Float) -> f64
    where
        F: Frame<N, Sample = f64>,
    {
        let zipped: F = mean_sq.mul_frame(weights);

        Util::lufs(zipped.channels().sum())
    }

    pub fn frame_peak<F, const N: usize>(frame: F) -> f64
    where
        F: Frame<N, Sample = f64>,
    {
        // Take the highest absolute value found in this sample.
        // TODO: Handle NaN.
        frame.into_channels()
            .map(|x| x.abs())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0)
    }

    pub fn lufs_hist(count: u64, sum: f64, reference: f64) -> f64 {
        if count == 0 { reference }
        else { Util::lufs(sum / count as f64) }
    }

    pub fn den(x: f64) -> f64 {
        if x.abs() < DEN_THRESHOLD { 0.0 }
        else { x }
    }

    /// Calculates the mean square of a signal.
    pub fn mean_sq<S, const N: usize>(frames: S) -> S::Frame
    where
        S: Signal<N>,
        S::Frame: Frame<N, Sample = f64>,
    {
        let mut stats = Stats::new();
        stats.extend(frames.map(|s| s.mul_frame(s.into_float_frame())));
        stats.mean
    }

    /// Calculates the root mean square of a signal.
    pub fn root_mean_sq<S, const N: usize>(frames: S) -> S::Frame
    where
        S: Signal<N>,
        S::Frame: Frame<N, Sample = f64>,
    {
        let mean_sqs = Self::mean_sq(frames);
        mean_sqs.apply(f64::sqrt)
    }

    /// Calculates the number of samples in a given number of milliseconds with
    /// respect to a sample rate.
    pub fn ms_to_samples(ms: u64, sample_rate: u32) -> u64 {
        let num = ms * sample_rate as u64;

        // Always round to the nearest sample.
        (num / 1000) + if num % 1000 >= 500 { 1 } else { 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_util::TestUtil;

    use sampara::{Signal, signal};

    use approx::assert_relative_eq;

    #[test]
    fn ms_to_samples() {
        let inputs_and_expected = vec![
            ((100, 44100), 4410),
            ((100, 44123), 4412),
            ((1, 44100), 44),
            ((1, 44600), 45),
            ((1, 44500), 45),
            ((1, 44499), 44),
            ((487, 12345), 6012),
            ((489, 12345), 6037),
        ];

        for (inputs, expected) in inputs_and_expected {
            let (ms, sample_rate) = inputs;
            let produced = Util::ms_to_samples(ms, sample_rate);

            assert_eq!(expected, produced)
        }
    }

    #[test]
    fn root_mean_sq() {
        const SAMPLE_RATE: usize = 100000;
        const AMPLITUDES: [f64; 5] = [0.2, 0.4, 0.6, 0.8, 1.0];
        const EPSILON: f64 = 1e-8;

        // Full flat signal.
        let signal = signal::constant(AMPLITUDES).take(SAMPLE_RATE);

        let produced = Util::root_mean_sq(signal);
        let expected = AMPLITUDES;

        for (p, e) in produced.into_channels().zip(expected.into_channels()) {
            assert_relative_eq!(p, e, epsilon=EPSILON);
        }

        // Sine wave.
        let signal = TestUtil::gen_sine_signal(SAMPLE_RATE as f64, 1000.0)
            .map(|x| AMPLITUDES.mul_amp(x))
            .take(SAMPLE_RATE);

        let produced = Util::root_mean_sq(signal);
        let expected = AMPLITUDES.mul_amp(1.0 / 2.0f64.sqrt());

        for (p, e) in produced.into_channels().zip(expected.into_channels()) {
            assert_relative_eq!(p, e, epsilon=EPSILON);
        }

        // Square wave.
        let signal = TestUtil::gen_square_signal(SAMPLE_RATE as f64, 1000.0)
            .map(|x| AMPLITUDES.mul_amp(x))
            .take(SAMPLE_RATE);

        let produced = Util::root_mean_sq(signal);
        let expected = AMPLITUDES;

        for (p, e) in produced.into_channels().zip(expected.into_channels()) {
            assert_relative_eq!(p, e, epsilon=EPSILON);
        }
    }
}
