use sampara::{Frame, Calculator};
use sampara::stats::CumulativeMean;

use crate::util::Util;

const ABS_LOUDNESS_THRESH: f64 = -70.0;

pub struct Loudness<F, const N: usize>
where
    F: Frame<N, Sample = f64>,
{
    abs_averager: CumulativeMean<F, N>,
    abs_loud_frames: Vec<(f64, F)>,
    g_weights: F,
}

impl<F, const N: usize> Loudness<F, N>
where
    F: Frame<N, Sample = f64>,
{
    pub fn new(g_weights: F) -> Self {
        Self {
            abs_averager: CumulativeMean::default(),
            abs_loud_frames: Vec::new(),
            g_weights,
        }
    }

    pub fn push(&mut self, gated_powers: F) {
        let frame_loudness = Util::loudness(gated_powers, self.g_weights);

        // If the frame loudness is greater than the absolute loudness
        // threshold (i.e. it is "not silence"), save the frame and its
        // loudness.
        if frame_loudness > ABS_LOUDNESS_THRESH {
            self.abs_averager.advance(gated_powers);
            self.abs_loud_frames.push((frame_loudness, gated_powers))
        }
    }

    pub fn is_empty(&self) -> bool {
        self.abs_averager.is_empty()
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.g_weights)
    }

    pub fn calculate(self) -> Option<f64> {
        let Self { abs_averager, abs_loud_frames, g_weights, .. } = self;

        println!("Num gates processed: {}", abs_averager.count());

        // This performs the calculation done in equation #5 in the ITU BS.1770
        // tech spec. This is the loudness of the average of the per-channel
        // power of frames that were marked as "loud" (i.e. frames with
        // loudness above the absolute loudness threshold) during the initial
        // pass.
        let abs_avg_gated_power = abs_averager.try_current()?;
        let abs_loudness = Util::loudness(abs_avg_gated_power, g_weights);
        println!("Absolute loudness: {} LKFS", abs_loudness);

        // This performs the calculation done in equation #6 in the ITU BS.1770
        // tech spec. The relative loudness threshold is the absolute loudness
        // minus 10.0.
        let rel_loudness_thresh = abs_loudness - 10.0;
        println!("Relative threshold: {} LKFS", rel_loudness_thresh);

        // This performs the calculation done in equation #7 in the ITU BS.1770
        // tech spec. From the collection of saved frames that were marked as
        // "absolutely loud", only those that exceed the relative loudness
        // threshold need to be selected and averaged.
        let mut rel_averager = CumulativeMean::default();

        for (frame_loudness, channel_powers) in abs_loud_frames {
            // These frames are already known to be above the absolute loudness
            // threshold. However, for this calculation they also need to be
            // above the relative loudness threshold.
            if frame_loudness > rel_loudness_thresh {
                rel_averager.advance(channel_powers)
            }
        }

        let rel_avg_gated_power = rel_averager.try_current()?;
        let rel_loudness = Util::loudness(rel_avg_gated_power, g_weights);
        println!("Relative loudness: {} LKFS", rel_loudness);

        Some(rel_loudness)
    }
}

impl<F, const N: usize> Calculator for Loudness<F, N>
where
    F: Frame<N, Sample = f64>,
{
    type Input = F;
    type Output = Option<f64>;

    fn push(&mut self, gated_powers: Self::Input) {
        self.push(gated_powers)
    }

    fn calculate(self) -> Self::Output {
        self.calculate()
    }
}

#[cfg(test)]
mod tests {
}
