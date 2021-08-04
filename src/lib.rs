#![feature(bool_to_option, option_result_contains)]

pub mod filter;
pub mod util;
pub mod gating;
pub mod loudness;
pub mod peak;

pub(crate) mod test_util;

pub use filter::KWeightFilter;
pub use gating::GatedPowers;
pub use loudness::Loudness;

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::Path;

    use crate::test_util::TestUtil;

    use sampara::signal::Signal;
    use sampara::wavegen::{Sine, Phase};

    use approx::assert_abs_diff_eq;

    #[test]
    fn nominal_frequency_reading() {
        // As per the ITU BS.1770 spec:
        // If a 0 dB FS, 997 Hz sine wave is applied to the left, center, or right channel input,
        // the indicated loudness will equal -3.01 LKFS.
        const SAMPLE_RATE: f64 = 48000.0;
        const SINE_HZS: [f64; 5] = [997.0, 0.0, 0.0, 0.0, 0.0];
        const G_WEIGHTS: [f64; 5] = [1.0, 1.0, 1.0, 1.41, 1.41];

        let phase = Phase::fixed_hz(SAMPLE_RATE, SINE_HZS);
        let signal = phase.gen_wave(Sine).take((SAMPLE_RATE as usize) * 2);

        let k_weighter = KWeightFilter::new(SAMPLE_RATE as u32);
        let power_gater = GatedPowers::new(SAMPLE_RATE as u32);

        let filtered_signal = signal.process(k_weighter);
        let gated_signal = filtered_signal.blocking_process(power_gater);

        let loudness = gated_signal.calculate(Loudness::new(G_WEIGHTS)).unwrap();

        assert_abs_diff_eq!(loudness, -3.010251969611668, epsilon = 1e-9);
    }

    #[test]
    fn scan_custom_audio() {
        let custom_audio_dir = Path::new("audio");
        let album_testcases = TestUtil::collect_album_testcases(&custom_audio_dir);

        for (_album_analysis, _album_root_dir) in album_testcases {

        }
    }
}
