
use std::f64::consts::PI;

#[cfg(test)] use approx::AbsDiffEq;

use crate::util::Util;
use crate::constants::MAX_CHANNELS;

/// Coefficients for a biquad digital filter at a particular sample rate.
/// It is assumed that the `a0` coefficient is always normalized to 1.0, and thus not included here.
#[derive(Copy, Clone, Debug, PartialEq)]
struct Biquad {
    a1: f64,
    a2: f64,
    b0: f64,
    b1: f64,
    b2: f64,
}

#[cfg(test)]
impl AbsDiffEq for Biquad {
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

#[derive(Copy, Clone, Debug)]
enum Kind {
    A, B,
}

impl Kind {
    pub fn get_biquad(&self, sample_rate: u32) -> Biquad {
        let g = 3.999843853973347;

        let (f0, q) =
            match self {
                &Kind::A => (1681.974450955533, 0.7071752369554196),
                &Kind::B => (38.13547087602444, 0.5003270373238773),
            }
        ;

        let k = (PI * f0 / sample_rate as f64).tan();

        let a0 = 1.0 + k / q + k * k;
        let a1 = 2.0 * (k * k - 1.0) / a0;
        let a2 = (1.0 - k / q + k * k) / a0;

        let (b0, b1, b2) =
            match self {
                &Kind::A => {
                    let vh = 10.0f64.powf(g / 20.0);
                    let vb = vh.powf(0.4996667741545416);

                    let b0 = (vh + vb * k / q + k * k) / a0;
                    let b1 = 2.0 * (k * k - vh) / a0;
                    let b2 = (vh - vb * k / q + k * k) / a0;

                    (b0, b1, b2)
                },
                &Kind::B => (1.0, -2.0, 1.0),
            }
        ;

        Biquad { a1, a2, b0, b1, b2, }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Applicator {
    biquad: Biquad,
    state1: [f64; MAX_CHANNELS],
    state2: [f64; MAX_CHANNELS],
}

impl Applicator {
    fn new(kind: Kind, sample_rate: u32) -> Self {
        Applicator {
            biquad: kind.get_biquad(sample_rate),
            state1: [0.0; MAX_CHANNELS],
            state2: [0.0; MAX_CHANNELS],
        }
    }

    pub fn apply(&mut self, input: &[f64; MAX_CHANNELS]) -> [f64; MAX_CHANNELS] {
        let mut output = [0.0f64; MAX_CHANNELS];

        // https://www.earlevel.com/main/2012/11/26/biquad-c-source-code/
        for ch in 0..MAX_CHANNELS {
            // output[ch] = input[ch] * self.ps.a0 + self.state1[ch];
            output[ch] = input[ch] + self.state1[ch];
            self.state1[ch] = input[ch] * self.biquad.a1 + self.state2[ch] - self.biquad.b1 * output[ch];
            self.state2[ch] = input[ch] * self.biquad.a2 - self.biquad.b2 * output[ch];
        }

        output
    }
}

/// The initial two-pass "K"-filter as described by the ITU-R BS.1770-4 spec.
/// The first pass is a shelving filter, which accounts for the acoustic effects of the listener's (spherical) head.
/// The second pass is a simple high pass filter.
#[derive(Copy, Clone, Debug)]
pub struct Filter {
    sample_rate: u32,
    pass_a: Applicator,
    pass_b: Applicator,
}

impl Filter {
    pub fn new(sample_rate: u32) -> Self {
        let pass_a = Applicator::new(Kind::A, sample_rate);
        let pass_b = Applicator::new(Kind::B, sample_rate);

        Filter { sample_rate, pass_a, pass_b, }
    }

    pub fn apply(&mut self, input: &[f64; MAX_CHANNELS]) -> [f64; MAX_CHANNELS] {
        self.pass_b.apply(&self.pass_a.apply(input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_get_biquad() {
        let expected = Biquad {
            a1: -1.6906592931824103,
            a2: 0.7324807742158501,
            b0: 1.5351248595869702,
            b1: -2.6916961894063807,
            b2: 1.19839281085285,
        };
        let produced = Kind::A.get_biquad(48000);

        println!("{:?}, {:?}", expected, produced);
        assert_abs_diff_eq!(expected, produced);

        let expected = Biquad {
            a1: -1.6636551132560204,
            a2: 0.7125954280732254,
            b0: 1.5308412300503478,
            b1: -2.6509799951547297,
            b2: 1.169079079921587,
        };
        let produced = Kind::A.get_biquad(44100);

        println!("{:?}, {:?}", expected, produced);
        assert_abs_diff_eq!(expected, produced);

        let expected = Biquad {
            a1: -0.2933807824149212,
            a2: 0.18687510604540827,
            b0: 1.3216235689299776,
            b1: -0.7262554913156911,
            b2: 0.2981262460162007,
        };
        let produced = Kind::A.get_biquad(8000);

        println!("{:?}, {:?}", expected, produced);
        assert_abs_diff_eq!(expected, produced);

        let expected = Biquad {
            a1: -1.9222022306074886,
            a2: 0.925117735116826,
            b0: 1.5722272150912788,
            b1: -3.0472830515615508,
            b2: 1.4779713409796091,
        };
        let produced = Kind::A.get_biquad(192000);

        println!("{:?}, {:?}", expected, produced);
        assert_abs_diff_eq!(expected, produced);
    }
}
