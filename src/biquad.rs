use std::f64::consts::PI;

#[cfg(test)] use approx::AbsDiffEq;

use crate::constants::MAX_CHANNELS;

#[derive(Copy, Clone, Debug)]
enum Kind {
    A, B,
}

/// Coefficients for a biquad digital filter at a particular sample rate.
/// It is assumed that the `a0` coefficient is always normalized to 1.0, and thus not included here.
#[derive(Copy, Clone, Debug, PartialEq)]
struct Coefficients {
    // Numerator coefficients.
    b0: f64,
    b1: f64,
    b2: f64,

    // Denominator coefficients, a0 is implied/assumed to be normalized to 1.0.
    a1: f64,
    a2: f64,
}

impl Coefficients {
    // https://hydrogenaud.io/index.php?topic=76394.0
    pub fn new(kind: Kind, sample_rate: u32) -> Self {
        let (f0, q) =
            match kind {
                Kind::A => (1681.974450955533, 0.7071752369554196),
                Kind::B => (38.13547087602444, 0.5003270373238773),
            }
        ;

        let k = (PI * f0 / sample_rate as f64).tan();

        let a0 = 1.0 + k / q + k * k;
        let a1 = 2.0 * (k * k - 1.0) / a0;
        let a2 = (1.0 - k / q + k * k) / a0;

        let (b0, b1, b2) =
            match kind {
                Kind::A => {
                    let height = 3.999843853973347;

                    let vh = 10.0f64.powf(height / 20.0);
                    let vb = vh.powf(0.4996667741545416);

                    let b0 = (vh + vb * k / q + k * k) / a0;
                    let b1 = 2.0 * (k * k - vh) / a0;
                    let b2 = (vh - vb * k / q + k * k) / a0;

                    (b0, b1, b2)
                },
                Kind::B => (1.0, -2.0, 1.0),
            }
        ;

        Self { a1, a2, b0, b1, b2, }
    }
}

#[cfg(test)]
impl AbsDiffEq for Coefficients {
    type Epsilon = f64;

    fn default_epsilon() -> Self::Epsilon {
        f64::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        f64::abs_diff_eq(&self.a1, &other.a1, epsilon)
            && f64::abs_diff_eq(&self.a2, &other.a2, epsilon)
            && f64::abs_diff_eq(&self.b0, &other.b0, epsilon)
            && f64::abs_diff_eq(&self.b1, &other.b1, epsilon)
            && f64::abs_diff_eq(&self.b2, &other.b2, epsilon)
    }
}

/// Filter parameters, which determine a particular frequency response for a
/// given sampling rate.
struct Parameters {
    // Omega, equal to `tan(PI * fc / fs)`.
    k: f64,

    /// Q factor, used as a quality factor in biquad filters.
    q: f64,

    // Band-pass gain (@ target frequency `fc`).
    vb: f64,

    // Low-pass gain (@ DC).
    vl: f64,

    // High-pass gain (@ Nyquist frequency).
    vh: f64,
}

impl Parameters {
    fn from_coefficients(co: &Coefficients) -> Self {
        let x11 = co.a1 - 2.0;
        let x12 = co.a1;
        let x1 = -co.a1 - 2.0;

        let x21 = co.a2 - 1.0;
        let x22 = co.a2 + 1.0;
        let x2 = -co.a2 + 1.0;

        let dx = (x22 * x11) - (x12 * x21);
        let k_sq = ((x22 * x1) - (x12 * x2))/dx;
        let k_by_q = ((x11 * x2) - (x21 * x1))/dx;
        let a0 = 1.0 + k_by_q + k_sq;

        let k = k_sq.sqrt();
        let q = k / k_by_q;
        let vb = 0.50 * a0 * (co.b0 - co.b2) / k_by_q;
        let vl = 0.25 * a0 * (co.b0 + co.b1 + co.b2) / k_sq;
        let vh = 0.25 * a0 * (co.b0 - co.b1 + co.b2);

        Self { k, q, vb, vl, vh, }
    }
}

#[derive(Copy, Clone, Debug)]
struct Filter {
    coeff: Coefficients,
    s1: [f64; MAX_CHANNELS],
    s2: [f64; MAX_CHANNELS],
}

impl Filter {
    fn new(kind: Kind, sample_rate: u32) -> Self {
        Self {
            coeff: Coefficients::new(kind, sample_rate),
            s1: [0.0; MAX_CHANNELS],
            s2: [0.0; MAX_CHANNELS],
        }
    }

    pub fn apply(&mut self, input: &[f64; MAX_CHANNELS]) -> [f64; MAX_CHANNELS] {
        let mut output = [0.0f64; MAX_CHANNELS];

        // https://www.earlevel.com/main/2012/11/26/biquad-c-source-code/
        // https://github.com/korken89/biquad-rs/blob/master/src/lib.rs
        for ch in 0..MAX_CHANNELS {
            let out = self.s1[ch] + self.coeff.b0 * input[ch];
            self.s1[ch] = self.s2[ch] + self.coeff.b1 * input[ch] - self.coeff.a1 * out;
            self.s2[ch] = self.coeff.b2 * input[ch] - self.coeff.a2 * out;

            output[ch] = out;
        }

        output
    }
}

/// The initial two-pass "K"-filter as described by the ITU-R BS.1770-4 spec.
/// The first pass is a high shelf boost filter, which accounts for the acoustic
/// effects of the listener's head, assumed to be roughly spherical. The second
/// pass is a simple high pass filter.
#[derive(Copy, Clone, Debug)]
struct FilterChain {
    pass_a: Filter,
    pass_b: Filter,
}

impl FilterChain {
    pub fn new(sample_rate: u32) -> Self {
        let pass_a = Filter::new(Kind::A, sample_rate);
        let pass_b = Filter::new(Kind::B, sample_rate);

        Self { pass_a, pass_b, }
    }

    pub fn apply(&mut self, input: &[f64; MAX_CHANNELS]) -> [f64; MAX_CHANNELS] {
        self.pass_b.apply(&self.pass_a.apply(input))
    }
}

pub struct FilteredSamples<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    sample_iter: I,
    filter_chain: FilterChain,
}

impl<I> FilteredSamples<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    pub fn new(sample_iter: I, sample_rate: u32) -> Self {
        let filter_chain = FilterChain::new(sample_rate);
        Self {
            sample_iter,
            filter_chain,
        }
    }
}

impl<I> Iterator for FilteredSamples<I>
where
    I: Iterator<Item = [f64; MAX_CHANNELS]>
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let raw_sample = self.sample_iter.next()?;
        let filtered_sample = self.filter_chain.apply(&raw_sample);

        Some(filtered_sample)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_new() {
        let expected = Coefficients {
            a1: -1.6906592931824103,
            a2: 0.7324807742158501,
            b0: 1.5351248595869702,
            b1: -2.6916961894063807,
            b2: 1.19839281085285,
        };
        let produced = Coefficients::new(Kind::A, 48000);

        println!("{:?}, {:?}", expected, produced);
        assert_abs_diff_eq!(expected, produced);

        let expected = Coefficients {
            a1: -1.6636551132560204,
            a2: 0.7125954280732254,
            b0: 1.5308412300503478,
            b1: -2.6509799951547297,
            b2: 1.169079079921587,
        };
        let produced = Coefficients::new(Kind::A, 44100);

        println!("{:?}, {:?}", expected, produced);
        assert_abs_diff_eq!(expected, produced);

        let expected = Coefficients {
            a1: -0.2933807824149212,
            a2: 0.18687510604540827,
            b0: 1.3216235689299776,
            b1: -0.7262554913156911,
            b2: 0.2981262460162007,
        };
        let produced = Coefficients::new(Kind::A, 8000);

        println!("{:?}, {:?}", expected, produced);
        assert_abs_diff_eq!(expected, produced);

        let expected = Coefficients {
            a1: -1.9222022306074886,
            a2: 0.925117735116826,
            b0: 1.5722272150912788,
            b1: -3.0472830515615508,
            b2: 1.4779713409796091,
        };
        let produced = Coefficients::new(Kind::A, 192000);

        println!("{:?}, {:?}", expected, produced);
        assert_abs_diff_eq!(expected, produced);

        let expected = Coefficients {
            a1: -1.99004745483398,
            a2:  0.99007225036621,
            b0:  1.00000000000000,
            b1: -2.00000000000000,
            b2:  1.00000000000000,
        };
        let produced = Coefficients::new(Kind::B, 48000);

        println!("{:?}, {:?}", expected, produced);
        assert_abs_diff_eq!(expected, produced);
    }

    #[test]
    fn filter_new() {
        let filter = Filter::new(Kind::A, 48000);

        assert_abs_diff_eq!(
            filter.coeff,
            Coefficients {
                a1: -1.6906592931824103,
                a2: 0.7324807742158501,
                b0: 1.5351248595869702,
                b1: -2.6916961894063807,
                b2: 1.19839281085285,
            },
        );
        assert_eq!(filter.s1, [0.0f64; MAX_CHANNELS]);
        assert_eq!(filter.s2, [0.0f64; MAX_CHANNELS]);
    }

    #[test]
    fn filter_apply() {
        let mut filter = Filter::new(Kind::A, 48000);

        let expected_rows = vec![
            [-1.5351248595869702, -0.7675624297934851, 0.0, 0.7675624297934851, 1.5351248595869702],
            [-1.4388017802366435, -0.7194008901183218, 0.0, 0.7194008901183218, 1.4388017802366435],
            [-1.3498956361696552, -0.6749478180848276, 0.0, 0.6749478180848276, 1.3498956361696552],
            [-1.2701404412191692, -0.6350702206095846, 0.0, 0.6350702206095846, 1.2701404412191692],
            [-1.2004236209352888, -0.6002118104676444, 0.0, 0.6002118104676444, 1.2004236209352888],
            [-1.1409753777762859, -0.5704876888881429, 0.0, 0.5704876888881429, 1.1409753777762859],
            [-1.0915348835135539, -0.5457674417567769, 0.0, 0.5457674417567769, 1.0915348835135539],
            [-1.0514925476036132, -0.5257462738018066, 0.0, 0.5257462738018066, 1.0514925476036132],
        ];

        let input = [-1.0, -0.5, 0.0, 0.5, 1.0];

        for expected in expected_rows {
            let produced = filter.apply(&input);

            println!("{:?}", expected);
            println!("{:?}", produced);
            for (e, p) in expected.iter().zip(&produced) {
                assert_abs_diff_eq!(e, p);
            }
        }
    }
}
