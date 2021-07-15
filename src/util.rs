use sampara::Frame;

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
    pub fn loudness<F, const N: usize>(mean_sq: F, weights: F) -> f64
    where
        F: Frame<N, Sample = f64>,
    {
        let zipped: F = mean_sq.mul_frame(weights.into_float_frame());

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
}
