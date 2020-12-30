
use crate::constants::MAX_CHANNELS;
use crate::stats::Stats;
use crate::util::Util;

const ABSOLUTE_LOUDNESS_THRESHOLD: f64 = -70.0;

pub struct Loudness;

impl Loudness {
    pub fn from_gated_channel_powers<I>(gated_channel_powers_iter: I) -> f64
    where
        I: IntoIterator<Item = [f64; MAX_CHANNELS]>
    {
        let mut averager = Stats::new();
        let mut absolutely_loud_blocks = Vec::new();

        let mut num_gates: usize = 0;
        for (j, channel_powers) in gated_channel_powers_iter.into_iter().enumerate() {
            let block_loudness = Util::block_loudness(&channel_powers);

            // If the block loudness is greater than the absolute loudness threshold, save the channel powers.
            if block_loudness > ABSOLUTE_LOUDNESS_THRESHOLD {
                averager.add(&channel_powers);
                absolutely_loud_blocks.push((j, block_loudness, channel_powers))
            }

            num_gates += 1;
        }

        println!("Num gates processed: {}", num_gates);

        // This performs the calculation done in equation #5 in the ITU BS.1770 tech spec.
        // This is the loudness of the average of the per-channel power of blocks that were marked as "loud"
        // (i.e. blocks with loudness above the absolute loudness threshold) during the initial pass.
        let absolute_loudness = Util::block_loudness(&averager.mean);
        println!("Absolute loudness: {} LKFS", absolute_loudness);

        // This performs the calculation done in equation #6 in the ITU BS.1770 tech spec.
        // The relative loudness threshold is the absolute loudness minus 10.0.
        let relative_loudness_threshold = absolute_loudness - 10.0;
        println!("Relative threshold: {} LKFS", relative_loudness_threshold);

        // This performs the calculation done in equation #7 in the ITU BS.1770 tech spec.
        // From the collection of saved blocks that were marked as "absolutely loud",
        // only those that exceed the relative loudness threshold need to be selected and averaged.
        let mut relative_averager = Stats::new();

        for (_, block_loudness, channel_powers) in absolutely_loud_blocks {
            // These blocks are already known to be above the absolute loudness threshold.
            // However, for this calculation, they also need to be over the relative loudness threshold.
            if block_loudness > relative_loudness_threshold {
                relative_averager.add(&channel_powers)
            }
        }

        let relative_loudness = Util::block_loudness(&relative_averager.mean);
        println!("Relative loudness: {} LKFS", relative_loudness);

        relative_loudness
    }
}

#[cfg(test)]
mod tests {
}
