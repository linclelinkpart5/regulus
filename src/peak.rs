//! Utilities for sample and true peak analysis, according to the BS.1770 spec.

use sampara::{Frame, Signal};

pub struct RunningPeak<S, const N: usize>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    frames: S,

    // This stores the highest absolute value peak for each channel that has
    // been seen so far.
    peaks: S::Frame,
}

impl<S, const N: usize> RunningPeak<S, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    pub fn new(frames: S) -> Self {
        Self {
            frames,
            peaks: Frame::EQUILIBRIUM,
        }
    }
}

impl<S, const N: usize> Signal<N> for RunningPeak<S, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    type Frame = S::Frame;

    fn next(&mut self) -> Option<Self::Frame> {
        let frame = self.frames.next()?;

        self.peaks.zip_transform(frame, |p, x| p.max(x.abs()));

        // Pass through the original frame.
        Some(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use sampara::signal;

    use approx::assert_abs_diff_eq;

    #[test]
    fn running_peak() {
        let frames = [
            [0.0, 0.0, 0.0, 0.0, 0.0],
            [-0.1, 0.2, -0.3, 0.4, -0.5],
        ];

        let mut original_iter = frames.iter().copied();
        let mut running_peak = RunningPeak::new(
            signal::from_frames(frames.iter().copied())
        );

        while let Some(produced) = running_peak.next() {
            let expected = original_iter.next().unwrap();
            for (e, p) in expected.into_channels().zip(produced.into_channels()) {
                assert_abs_diff_eq!(e, p);
            }
        }

        let expected = [0.1, 0.2, 0.3, 0.4, 0.5];
        for (e, p) in expected.into_channels().zip(running_peak.peaks.into_channels()) {
            assert_abs_diff_eq!(e, p);
        }

        let frames = [
            [0.1, 0.2, 0.3, 0.4, 0.5],
            [-1.0, 1.0, -1.0, 1.0, -1.0],
        ];

        let mut original_iter = frames.iter().copied();
        let mut running_peak = RunningPeak::new(
            signal::from_frames(frames.iter().copied())
        );

        while let Some(produced) = running_peak.next() {
            let expected = original_iter.next().unwrap();
            for (e, p) in expected.into_channels().zip(produced.into_channels()) {
                assert_abs_diff_eq!(e, p);
            }
        }

        let expected = [1.0, 1.0, 1.0, 1.0, 1.0];
        for (e, p) in expected.into_channels().zip(running_peak.peaks.into_channels()) {
            assert_abs_diff_eq!(e, p);
        }

        let mut running_peak = RunningPeak::new(signal::empty::<f64, 1>());

        assert_eq!(running_peak.next(), None);
        assert_eq!(running_peak.peaks, 0.0);
    }
}
