// #[cfg(test)] #[macro_use] extern crate approx;

pub mod constants;
pub mod filter;
pub mod stats;
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

    use std::f64::consts::PI;

    use sampara::signal::Signal;

    use approx::assert_abs_diff_eq;

    #[test]
    fn nominal_frequency_reading() {
        // As per the ITU BS.1770 spec:
        // If a 0 dB FS, 997 Hz sine wave is applied to the left, center, or right channel input,
        // the indicated loudness will equal -3.01 LKFS.
        const SAMPLE_RATE: f64 = 48000.0;
        const SINE_HZ: f64 = 997.0;
        const STEP: f64 = SINE_HZ / SAMPLE_RATE;

        // Quick and easy way to generate a sine wave.
        // TODO: Replace with `sampara` wavegen once available.
        let mut phase: f64 = 0.0;
        let signal = sampara::signal::from_fn(move || {
            phase = (phase + STEP) % 1.0;
            let y = (2.0 * PI * phase).sin();
            Some([y, 0.0, 0.0, 0.0, 0.0])
        }).take((SAMPLE_RATE as usize) * 2);

        let filtered_signal = KWeightFilteredSignal::new(signal, SAMPLE_RATE as u32);
        let gated_powers = GatedPowers::new(filtered_signal, SAMPLE_RATE as u32);
        let loudness = Loudness::from_gated_powers(gated_powers, [1.0, 1.0, 1.0, 1.41, 1.41]);

        assert_abs_diff_eq!(loudness, -3.010258819171608, epsilon = 1e-9);
    }
}
