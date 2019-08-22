
use crate::constants::MAX_CHANNELS;
use crate::gating::GatedPowerIter;
use crate::stats::Stats;
use crate::util::Util;

const ABSOLUTE_LOUDNESS_THRESHOLD: f64 = -70.0;

pub struct GatedLoudnessIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    gpi: GatedPowerIter<I>,

    // Denoted as the `G` weights in the tech doc.
    channel_weights: [f64; MAX_CHANNELS],

    averager: Stats,
    absolutely_loud_blocks: Vec<(f64, [f64; MAX_CHANNELS])>,
}

impl<I> GatedLoudnessIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    pub fn new(sample_iter: I, sample_rate: u32, channel_weights: [f64; MAX_CHANNELS]) -> Self {
        let gpi = GatedPowerIter::new(sample_iter, sample_rate);

        Self {
            gpi,
            channel_weights,
            averager: Stats::new(),
            absolutely_loud_blocks: Vec::new(),
        }
    }
}

impl<I> GatedLoudnessIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    pub fn absolute_loudness(&self) -> f64 {
        // This performs the calculation done in equation #5 in the ITU BS.1770 tech spec.
        // This is the loudness of the average of the per-channel power of blocks that were marked as "loud"
        // (i.e. blocks with loudness above the absolute loudness threshold) during the initial pass.
        Util::block_loudness(&self.averager.mean, &self.channel_weights)
    }

    #[inline]
    pub fn relative_loudness_threshold(&self) -> f64 {
        // This performs the calculation done in equation #6 in the ITU BS.1770 tech spec.
        // The relative loudness threshold is the absolute loudness minus 10.0.
        self.absolute_loudness() - 10.0
    }

    pub fn relative_loudness(&self) -> f64 {
        // This performs the calculation done in equation #7 in the ITU BS.1770 tech spec.
        // From the collected of saved blocks that were marked as "absolutely loud",
        // only those that exceed the relative loudness threshold need to be selected and averaged.
        let mut relative_averager = Stats::new();

        let relative_loudness_threshold = self.relative_loudness_threshold();

        for (block_loudness, channel_powers) in &self.absolutely_loud_blocks {
            // These blocks are already known to be above the absolute loudness threshold.
            // However they also need to be over the relative loudness threshold for this calculation.
            if block_loudness > &relative_loudness_threshold {
                relative_averager.add(channel_powers)
            }
        }

        let relative_loudness = Util::block_loudness(&relative_averager.mean, &self.channel_weights);

        relative_loudness
    }
}

impl<I> Iterator for GatedLoudnessIter<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        let channel_powers = self.gpi.next()?;
        let block_loudness = Util::block_loudness(&channel_powers, &self.channel_weights);

        // If the block loudness is greater than the absolute loudness threshold, save the channel powers.
        if block_loudness > ABSOLUTE_LOUDNESS_THRESHOLD {
            self.averager.add(&channel_powers);
            self.absolutely_loud_blocks.push((block_loudness, channel_powers))
        }

        Some(block_loudness)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::wave::WaveKind;
    use crate::wave::WaveGen;

    #[test]
    fn gated_loudness_iter() {
        const FREQUENCIES: [u32; MAX_CHANNELS] = [440, 480, 520, 560, 600];
        const CHANNEL_WEIGHTS: [f64; MAX_CHANNELS] = [0.8, 0.9, 1.0, 1.1, 1.2];

        let sample_iter = WaveGen::new(WaveKind::Sawtooth, 48000, FREQUENCIES);
        let mut gli = GatedLoudnessIter::new(sample_iter, 48000, CHANNEL_WEIGHTS);

        let expected_results = [
            1.527977014965285,
            1.5279770149652738,
            1.5279770149652414,
            1.5279770149652028,
            1.5279770149651917,
            1.5279770149651748,
            1.5279770149651393,
            1.5279770149651726,
        ];

        for expected in expected_results.iter() {
            let produced = gli.next().unwrap();
            assert_abs_diff_eq!(*expected, produced);
        }

        let sample_iter = WaveGen::new(WaveKind::Square, 48000, FREQUENCIES);
        let mut gli = GatedLoudnessIter::new(sample_iter, 48000, CHANNEL_WEIGHTS);

        let expected_results = [
            6.2987000433601885,
            6.2987000433601885,
            6.2987000433601885,
            6.2987000433601885,
            6.2987000433601885,
            6.2987000433601885,
            6.2987000433601885,
            6.2987000433601885,
        ];

        for expected in expected_results.iter() {
            let produced = gli.next().unwrap();
            assert_abs_diff_eq!(*expected, produced);
        }

        let sample_iter = WaveGen::new(WaveKind::Triangle, 48000, FREQUENCIES);
        let mut gli = GatedLoudnessIter::new(sample_iter, 48000, CHANNEL_WEIGHTS);

        let expected_results = [
            1.5294452403980916,
            1.5294452403980507,
            1.5294452403979846,
            1.5294452403979464,
            1.5294452403979424,
            1.529445240397938,
            1.5294452403979144,
            1.5294452403980756,
        ];

        for expected in expected_results.iter() {
            let produced = gli.next().unwrap();
            assert_abs_diff_eq!(*expected, produced);
        }

        let sample_iter = WaveGen::new(WaveKind::Sine, 48000, FREQUENCIES);
        let mut gli = GatedLoudnessIter::new(sample_iter, 48000, CHANNEL_WEIGHTS);

        let expected_results = [
            3.2884000867203746,
            3.2884000867203715,
            3.2884000867203795,
            3.2884000867203795,
            3.28840008672038,
            3.28840008672038,
            3.2884000867203786,
            3.288400086720374,
        ];

        for expected in expected_results.iter() {
            let produced = gli.next().unwrap();
            assert_abs_diff_eq!(*expected, produced);
        }

        // for _ in 0..8 {
        //     println!("{:?}", gli.next());
        // }

        // return;
    }
}
