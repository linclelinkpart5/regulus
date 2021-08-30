use std::f64::consts::PI;

use sampara::{Frame, Processor};
use sampara::biquad::{Params, Biquad as BQ};

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

pub struct KWeightFilter<F, const N: usize>
where
    F: Frame<N, Sample = f64>,
{
    bq_shelving: BQ<F, N>,
    bq_highpass: BQ<F, N>,
}

impl<F, const N: usize> KWeightFilter<F, N>
where
    F: Frame<N, Sample = f64>,
{
    pub fn new(sample_rate: u32) -> Self {
        let bq_shelving = BQ::from(Kind::Shelving.coefficients(sample_rate));
        let bq_highpass = BQ::from(Kind::HighPass.coefficients(sample_rate));

        Self { bq_shelving, bq_highpass }
    }

    pub fn process(&mut self, input: F) -> F {
        Processor::process(self, input)
    }
}

impl<F, const N: usize> Processor for KWeightFilter<F, N>
where
    F: Frame<N, Sample = f64>,
{
    type Input = F;
    type Output = F;

    fn process(&mut self, input: Self::Input) -> Self::Output {
        self.bq_highpass.process(self.bq_shelving.process(input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

