use sampara::{Frame, Signal};
use sampara::stats::CumulativeMean;

use crate::util::Util;

const ABS_LOUDNESS_THRESH: f64 = -70.0;

pub struct Loudness;

impl Loudness {
    pub fn from_gated_powers<S, const N: usize>(
        gated_powers: S,
        channel_weights: S::Frame,
    ) -> f64
    where
        S: Signal<N>,
        S::Frame: Frame<N, Sample = f64>,
    {
        let mut averager = CumulativeMean::default();
        let mut abs_loud_frames = Vec::new();

        let mut num_gates: usize = 0;
        for channel_powers in gated_powers.into_iter() {
            let frame_loudness = Util::loudness(channel_powers, channel_weights);

            // If the frame loudness is greater than the absolute loudness
            // threshold (i.e. it is "loud enough"), save the frame and its
            // loudness.
            if frame_loudness > ABS_LOUDNESS_THRESH {
                averager.advance(channel_powers);
                abs_loud_frames.push((frame_loudness, channel_powers))
            }

            num_gates += 1;
        }

        println!("Num gates processed: {}", num_gates);

        // This performs the calculation done in equation #5 in the ITU BS.1770
        // tech spec. This is the loudness of the average of the per-channel
        // power of frames that were marked as "loud" (i.e. frames with
        // loudness above the absolute loudness threshold) during the initial
        // pass.
        let abs_loudness = Util::loudness(averager.current(), channel_weights);
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
            // threshold. For this calculation however, they also need to be
            // over the relative loudness threshold.
            if frame_loudness > rel_loudness_thresh {
                rel_averager.advance(channel_powers)
            }
        }

        let rel_loudness = Util::loudness(rel_averager.current(), channel_weights);
        println!("Relative loudness: {} LKFS", rel_loudness);

        rel_loudness
    }
}

#[cfg(test)]
mod tests {
}
