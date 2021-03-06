#![feature(option_result_contains)]

pub mod constants;
pub mod filter;
pub mod util;
pub mod gating;
pub mod loudness;
pub mod peak;

pub(crate) mod test_util;

pub use constants::MAX_CHANNELS;

pub use filter::KWeightFilteredSignal;
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

        let phase = Phase::fixed_hz(SAMPLE_RATE, SINE_HZS);
        let signal = phase.gen_wave(Sine).take((SAMPLE_RATE as usize) * 2);

        let filtered_signal = KWeightFilteredSignal::new(signal, SAMPLE_RATE as u32);
        let gated_powers = GatedPowers::new(filtered_signal, SAMPLE_RATE as u32);
        let loudness = Loudness::from_gated_powers(gated_powers, [1.0, 1.0, 1.0, 1.41, 1.41]);

        assert_abs_diff_eq!(loudness, -3.010251969611668, epsilon = 1e-9);
    }

    #[test]
    fn scan_custom_audio() {
        let custom_audio_dir = Path::new("audio");
        let album_dirs = TestUtil::collect_album_dirs(&custom_audio_dir);

        for album_dir in album_dirs {

        }
    }
}
