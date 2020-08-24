//! Utilities for sample and true peak analysis, according to the BS.1770 spec.

use crate::constants::MAX_CHANNELS;

pub struct SamplePeakIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    sample_iter: I,
    peak_per_channel: [f64; MAX_CHANNELS],
}

impl<I> SamplePeakIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    pub fn new(sample_iter: I) -> Self {
        Self {
            sample_iter,
            peak_per_channel: [0.0f64; MAX_CHANNELS],
        }
    }
}

impl<I> Iterator for SamplePeakIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    type Item = [f64; MAX_CHANNELS];

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.sample_iter.next()?;

        for ch in 0..MAX_CHANNELS {
            self.peak_per_channel[ch] = self.peak_per_channel[ch].max(sample[ch].abs());
        }

        // Pass through the original sample.
        Some(sample)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use approx::assert_abs_diff_eq;

    #[test]
    fn sample_peak_iter() {
        let samples = [
            [0.0, 0.0, 0.0, 0.0, 0.0],
            [-0.1, 0.2, -0.3, 0.4, -0.5],
        ];

        let mut original_iter = samples.iter().copied();
        let mut sample_peak_iter = SamplePeakIter::new(samples.iter().copied());

        while let Some(produced) = sample_peak_iter.next() {
            let expected = original_iter.next().unwrap();
            for ch in 0..MAX_CHANNELS { assert_abs_diff_eq!(expected[ch], produced[ch]); }
        }

        let expected = [0.1, 0.2, 0.3, 0.4, 0.5];
        for ch in 0..MAX_CHANNELS {
            let e = expected[ch];
            let p = sample_peak_iter.peak_per_channel[ch];
            assert_abs_diff_eq!(e, p);
        }

        let samples = [
            [0.1, 0.2, 0.3, 0.4, 0.5],
            [-1.0, 1.0, -1.0, 1.0, -1.0],
        ];

        let mut original_iter = samples.iter().copied();
        let mut sample_peak_iter = SamplePeakIter::new(samples.iter().copied());

        while let Some(produced) = sample_peak_iter.next() {
            let expected = original_iter.next().unwrap();
            for ch in 0..MAX_CHANNELS { assert_abs_diff_eq!(expected[ch], produced[ch]); }
        }

        let expected = [1.0, 1.0, 1.0, 1.0, 1.0];
        for ch in 0..MAX_CHANNELS {
            let e = expected[ch];
            let p = sample_peak_iter.peak_per_channel[ch];
            assert_abs_diff_eq!(e, p);
        }

        let samples = [
        ];

        let mut original_iter = samples.iter().copied();
        let mut sample_peak_iter = SamplePeakIter::new(samples.iter().copied());

        while let Some(produced) = sample_peak_iter.next() {
            let expected = original_iter.next().unwrap();
            for ch in 0..MAX_CHANNELS { assert_abs_diff_eq!(expected[ch], produced[ch]); }
        }

        let expected = [0.0, 0.0, 0.0, 0.0, 0.0];
        for ch in 0..MAX_CHANNELS {
            let e = expected[ch];
            let p = sample_peak_iter.peak_per_channel[ch];
            assert_abs_diff_eq!(e, p);
        }
    }
}
