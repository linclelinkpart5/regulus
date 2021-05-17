use std::f64::consts::PI;

use sampara::biquad::Params;
use sampara::frame::Frame;
use sampara::signal::{Signal, Biquad};

#[derive(Copy, Clone, Debug)]
enum Kind {
    Shelving, HighPass,
}

impl Kind {
    fn coefficients(&self, sample_rate: u32) -> Params<f64> {
        let (f0, q) =
            match self {
                Self::Shelving => (1681.974450955533, 0.7071752369554196),
                Self::HighPass => (38.13547087602444, 0.5003270373238773),
            }
        ;

        let k = (PI * f0 / sample_rate as f64).tan();
        let k_by_q = k / q;
        let k_sq = k * k;

        let a0 = 1.0 + k_by_q + k_sq;
        let a1 = 2.0 * (k_sq - 1.0) / a0;
        let a2 = (1.0 - k_by_q + k_sq) / a0;

        let (b0, b1, b2) =
            match self {
                Self::Shelving => {
                    let height = 3.999843853973347;

                    let vh = 10.0f64.powf(height / 20.0);
                    let vb = vh.powf(0.4996667741545416);

                    let b0 = (vh + vb * k_by_q + k_sq) / a0;
                    let b1 = 2.0 * (k_sq - vh) / a0;
                    let b2 = (vh - vb * k_by_q + k_sq) / a0;

                    (b0, b1, b2)
                },
                Self::HighPass => (1.0, -2.0, 1.0),
            }
        ;

        Params { a1, a2, b0, b1, b2, }
    }
}

pub struct KWeightFilteredSignal<S, const N: usize>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    signal: Biquad<Biquad<S, N>, N>,
}

impl<S, const N: usize> KWeightFilteredSignal<S, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    pub fn new(signal: S, sample_rate: u32) -> Self {
        let signal = signal
            .biquad(Kind::Shelving.coefficients(sample_rate))
            .biquad(Kind::HighPass.coefficients(sample_rate));

        Self { signal }
    }
}

impl<S, const N: usize> Signal<N> for KWeightFilteredSignal<S, N>
where
    S: Signal<N>,
    S::Frame: Frame<N, Sample = f64>,
{
    type Frame = S::Frame;

    fn next(&mut self) -> Option<Self::Frame> {
        self.signal.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::process::Command;

    use approx::abs_diff_eq;

    use crate::test_util::{TestUtil, WaveKind};

    #[test]
    fn coefficients() {
        // ITU BS.1770 provides coefficients for both filters at a 48KHz
        // sampling rate:
        //     Shelving:
        //         a1: -1.69065929318241
        //         a2:  0.73248077421585
        //         b1: -2.69169618940638
        //         b2:  1.19839281085285
        //         b0:  1.53512485958697
        //     Highpass:
        //         a1: -1.99004745483398
        //         a2:  0.99007225036621
        //         b0:  1.0
        //         b1: -2.0
        //         b2:  1.0
        // However, the biquad coefficients are calculated from scratch from any
        // arbitrary sample rate, including @ 48KHz. The expected calculated
        // coefficients @ 48KHz are very close to the ITU BS.1770 values, but
        // not exact. As a result, in all of these tests the hard-coded
        // coefficients @ 48KHz do not exactly match those in ITU BS.1770, and
        // that is intentional.
        let expected = Params {
            a1: -1.6906592931824103,
            a2:  0.7324807742158501,
            b0:  1.5351248595869702,
            b1: -2.6916961894063807,
            b2:  1.19839281085285,
        };
        let produced = Kind::Shelving.coefficients(48000);

        assert_eq!(expected, produced);

        let expected = Params {
            a1: -1.6636551132560204,
            a2:  0.7125954280732254,
            b0:  1.5308412300503478,
            b1: -2.6509799951547297,
            b2:  1.169079079921587,
        };
        let produced = Kind::Shelving.coefficients(44100);

        assert_eq!(expected, produced);

        let expected = Params {
            a1: -0.2933807824149212,
            a2:  0.18687510604540827,
            b0:  1.3216235689299776,
            b1: -0.7262554913156911,
            b2:  0.2981262460162007,
        };
        let produced = Kind::Shelving.coefficients(8000);

        assert_eq!(expected, produced);

        let expected = Params {
            a1: -1.9222022306074886,
            a2:  0.9251177351168259,
            b0:  1.572227215091279,
            b1: -3.0472830515615508,
            b2:  1.4779713409796094,
        };
        let produced = Kind::Shelving.coefficients(192000);

        assert_eq!(expected, produced);

        let expected = Params {
            a1: -1.9900474548339797,
            a2:  0.9900722503662099,
            b0:  1.0,
            b1: -2.0,
            b2:  1.0,
        };
        let produced = Kind::HighPass.coefficients(48000);

        assert_eq!(expected, produced);
    }

    fn sox_gen_wave_filtered_cmd(sample_rate: u32, kind: &WaveKind, frequency: u32) -> Command {
        let mut cmd = TestUtil::sox_gen_wave_cmd(sample_rate, kind, frequency);

        // Shelving filter.
        let coeff = Kind::Shelving.coefficients(sample_rate);
        cmd.arg("biquad")
            .arg(coeff.b0.to_string())
            .arg(coeff.b1.to_string())
            .arg(coeff.b2.to_string())
            .arg("1.0")
            .arg(coeff.a1.to_string())
            .arg(coeff.a2.to_string())
        ;

        // High pass filter.
        let coeff = Kind::HighPass.coefficients(sample_rate);
        cmd.arg("biquad")
            .arg(coeff.b0.to_string())
            .arg(coeff.b1.to_string())
            .arg(coeff.b2.to_string())
            .arg("1.0")
            .arg(coeff.a1.to_string())
            .arg(coeff.a2.to_string())
        ;

        cmd
    }

    #[test]
    fn sox_filter_suite() {
        const RATE: u32 = 48000;
        const KIND: &WaveKind = &WaveKind::Sine;
        const FREQ: u32 = 997;

        let frames = TestUtil::sox_eval_samples(&mut TestUtil::sox_gen_wave_cmd(RATE, KIND, FREQ));
        let signal = sampara::signal::from_frames(frames);

        let filtered_frames = signal
            .biquad(Kind::Shelving.coefficients(RATE))
            .biquad(Kind::HighPass.coefficients(RATE))
            .into_iter()
            .collect::<Vec<_>>();

        let fx = TestUtil::sox_eval_samples(&mut sox_gen_wave_filtered_cmd(RATE, KIND, FREQ));

        // Check that the number of frames stays the same.
        assert_eq!(
            filtered_frames.len(), fx.len(),
            "frame counts differ: {} != {}", filtered_frames.len(), fx.len(),
        );

        for (i, (px, ex)) in filtered_frames.into_iter().zip(fx).enumerate() {
            assert!(
                abs_diff_eq!(px, ex, epsilon = 1e-9),
                "frames @ {} differ: {} != {}", i, px, ex
            );
        }
    }
}

