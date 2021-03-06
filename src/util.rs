use sampara::Frame;

use crate::stats::Stats;
use crate::constants::{MAX_CHANNELS, CHANNEL_G_WEIGHTS};

const DEN_THRESHOLD: f64 = 1.0e-15;

pub struct Util;

impl Util {
    pub fn lufs(x: f64) -> f64 {
        -0.691 + 10.0 * x.log10()
    }

    /// Given the mean squares (powers) of an input signal and a set of
    /// per-channel weights, calculates the weighted loudness.
    ///
    /// Note that this will produce a single scalar: the loudness across ALL
    /// input channels.
    pub fn loudness<F, W, const N: usize>(mean_sq: F, weights: W) -> f64
    where
        F: Frame<N, Sample = f64>,
        W: Frame<N, Sample = f64>,
    {
        let zipped: F = mean_sq.zip_apply(weights, |x, w| x * w);

        Util::lufs(zipped.channels().sum())
    }

    pub fn lufs_hist(count: u64, sum: f64, reference: f64) -> f64 {
        if count == 0 { reference }
        else { Util::lufs(sum / count as f64) }
    }

    pub fn den(x: f64) -> f64 {
        if x.abs() < DEN_THRESHOLD { 0.0 }
        else { x }
    }

    pub fn scale(sample: &[f64; MAX_CHANNELS], scale: f64) -> [f64; MAX_CHANNELS] {
        let mut scaled = [0.0f64; MAX_CHANNELS];
        for ch in 0..MAX_CHANNELS {
            scaled[ch] = sample[ch] * scale;
        }

        scaled
    }

    pub fn sample_sq(sample: &[f64; MAX_CHANNELS]) -> [f64; MAX_CHANNELS] {
        let mut sample_sq = [0.0f64; MAX_CHANNELS];
        for ch in 0..MAX_CHANNELS {
            sample_sq[ch] = sample[ch] * sample[ch];
        }
        sample_sq
    }

    /// Calculates the mean square of an iterable of samples.
    pub fn mean_sq<I>(samples: I) -> [f64; MAX_CHANNELS]
    where
        I: IntoIterator<Item = [f64; MAX_CHANNELS]>
    {
        let mut stats = Stats::new();
        stats.extend(samples.into_iter().map(|s| Util::sample_sq(&s)));
        stats.mean
    }

    /// Calculates the root mean square of an iterable of samples.
    pub fn root_mean_sq<I>(samples: I) -> [f64; MAX_CHANNELS]
    where
        I: IntoIterator<Item = [f64; MAX_CHANNELS]>
    {
        let mean_sqs = Self::mean_sq(samples);
        let mut root_mean_sqs = [0.0f64; MAX_CHANNELS];
        for ch in 0..MAX_CHANNELS {
            root_mean_sqs[ch] = mean_sqs[ch].sqrt();
        }

        root_mean_sqs
    }

    /// Calculates the number of samples in a given number of milliseconds with respect to a sample rate.
    pub fn ms_to_samples(ms: u64, sample_rate: u32) -> u64 {
        let num = ms * sample_rate as u64;

        // Always round to the nearest sample.
        (num / 1000) + if num % 1000 >= 500 { 1 } else { 0 }
    }

    // pub fn block_loudness(channel_powers: &[f64; MAX_CHANNELS], channel_weights: &[f64; MAX_CHANNELS]) -> f64 {
    pub fn block_loudness(channel_powers: &[f64; MAX_CHANNELS]) -> f64 {
        // This performs the calculation done in equation #4 in the ITU BS.1770 tech spec.
        // Weight the power for each channel according to the channel weights.
        let mut weighted_channel_powers = [0.0; MAX_CHANNELS];
        for ch in 0..MAX_CHANNELS {
            weighted_channel_powers[ch] = channel_powers[ch] * CHANNEL_G_WEIGHTS[ch];
        }

        // Calculate the loudness of this block from the total weighted channel power.
        let block_power = weighted_channel_powers.iter().sum::<f64>();
        let block_loudness = -0.691 + 10.0 * block_power.log10();

        block_loudness
    }

    pub fn block_peak(block_sample: &[f64; MAX_CHANNELS]) -> f64 {
        // Take the highest absolute value found in this sample.
        let mut peak = 0.0f64;
        for ch in 0..MAX_CHANNELS {
            let mag = block_sample[ch].abs();
            peak = peak.max(mag);
        }

        peak
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    // use sampara::Signal;

    // use approx::assert_relative_eq;

    // #[test]
    // fn util_ms_to_samples() {
    //     let inputs_and_expected = vec![
    //         ((100, 44100), 4410),
    //         ((100, 44123), 4412),
    //         ((1, 44100), 44),
    //         ((1, 44600), 45),
    //         ((1, 44500), 45),
    //         ((1, 44499), 44),
    //         ((487, 12345), 6012),
    //         ((489, 12345), 6037),
    //     ];

    //     for (inputs, expected) in inputs_and_expected {
    //         let (ms, sample_rate) = inputs;
    //         let produced = Util::ms_to_samples(ms, sample_rate);

    //         assert_eq!(expected, produced)
    //     }
    // }

    // #[test]
    // fn root_mean_sq() {
    //     const SAMPLE_RATE: usize = 100000;
    //     const AMPLITUDES: [f64; MAX_CHANNELS] = [0.2, 0.4, 0.6, 0.8, 1.0];
    //     const EPSILON: f64 = 1e-8;

    //     // Full flat signal.
    //     let signal =
    //         std::iter::repeat(AMPLITUDES)
    //         .take(SAMPLE_RATE)
    //     ;

    //     let produced = Util::root_mean_sq(signal);
    //     let expected = AMPLITUDES;

    //     for ch in 0..MAX_CHANNELS {
    //         let e = expected[ch];
    //         let p = produced[ch];
    //         assert_relative_eq!(e, p, epsilon=EPSILON);
    //     }

    //     // Sine wave.
    //     let mut mono_signal = sampara::signal::rate(SAMPLE_RATE as f64).const_hz(1000.0).sine();
    //     let signal =
    //         std::iter::from_fn(move || {
    //             let mut s = AMPLITUDES;
    //             let x = mono_signal.next();
    //             s.iter_mut().for_each(|e| *e *= x);
    //             Some(s)
    //         })
    //         .take(SAMPLE_RATE)
    //     ;

    //     let produced = Util::root_mean_sq(signal);
    //     let expected = Util::scale(&AMPLITUDES, 1.0 / 2.0f64.sqrt());

    //     for ch in 0..MAX_CHANNELS {
    //         let e = expected[ch];
    //         let p = produced[ch];
    //         assert_relative_eq!(e, p, epsilon=EPSILON);
    //     }

    //     // Square wave.
    //     let mut mono_signal = sampara::signal::rate(SAMPLE_RATE as f64).const_hz(1000.0).square();
    //     let signal =
    //         std::iter::from_fn(move || {
    //             let mut s = AMPLITUDES;
    //             let x = mono_signal.next();
    //             s.iter_mut().for_each(|e| *e *= x);
    //             Some(s)
    //         })
    //         .take(SAMPLE_RATE)
    //     ;

    //     let produced = Util::root_mean_sq(signal);
    //     let expected = AMPLITUDES;

    //     for ch in 0..MAX_CHANNELS {
    //         let e = expected[ch];
    //         let p = produced[ch];
    //         assert_relative_eq!(e, p, epsilon=EPSILON);
    //     }
    // }
}
