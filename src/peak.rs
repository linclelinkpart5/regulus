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
