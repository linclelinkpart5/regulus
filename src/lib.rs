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

pub use filter::FilteredSamples;
pub use gating::GatedPowerIter;
pub use loudness::Loudness;

// #[derive(Clone, Copy, Debug)]
// enum NormKind {
//     ReplayGain,
//     ATSC,
//     EBU,
//     Custom(f64),
// }

// impl Default for NormKind {
//     fn default() -> Self {
//         NormKind::ReplayGain
//     }
// }

// impl NormKind {
//     fn level(&self) -> f64 {
//         match *self {
//             NormKind::ReplayGain => -18.0,
//             NormKind::ATSC => -24.0,
//             NormKind::EBU => -23.0,
//             NormKind::Custom(n) => n,
//         }
//     }

//     fn units(&self) -> &'static str {
//         match *self {
//             NormKind::ReplayGain => "dB",
//             _ => "LU",
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    use dasp::signal::Signal;

    use approx::assert_abs_diff_eq;

    #[test]
    fn nominal_frequency_reading() {
        // As per the ITU BS.1770 spec:
        // If a 0 dB FS, 997 Hz sine wave is applied to the left, center, or right channel input,
        // the indicated loudness will equal -3.01 LKFS.
        let sample_rate: u32 = 48000;

        let mut mono_raw_signal = dasp::signal::rate(48000.0).const_hz(997.0).sine();
        let raw_signal =
            std::iter::from_fn(move || {
                let x = mono_raw_signal.next();
                Some([x, 0.0, 0.0, 0.0, 0.0])
            })
            .take(sample_rate as usize * 10)
        ;

        let filtered_signal = FilteredSamples::new(raw_signal, sample_rate);
        let gated_channel_powers_iter = GatedPowerIter::new(filtered_signal, sample_rate);
        let loudness = Loudness::from_gated_channel_powers(gated_channel_powers_iter);

        // assert_abs_diff_eq!(-3.01, loudness);
        assert_abs_diff_eq!(loudness, -3.0102799213963327);
    }
}
