
use crate::util::Util;

/// Coefficients for a biquad digital filter at a particular sample rate.
/// It is assumed that the `a0` coefficient is always normalized to 1.0, and thus not included here.
#[derive(Copy, Clone, Debug)]
pub struct Biquad {
    a1: f64,
    a2: f64,
    b0: f64,
    b1: f64,
    b2: f64,
}

// https://github.com/mzuther/K-Meter/blob/master/doc/specifications/ITU-R%20BS.1770-1%20(Filters).pdf
#[derive(Copy, Clone, Debug)]
struct AnalogParams {
    k: f64, // a.k.a. Ω, equal to `tan(pi * Fc / Fs)`, where Fc = target sample rate, Fs = source sample rate
    q: f64, // Q factor
    vb: f64, // band-pass gain factor
    vl: f64, // low-pass gain factor
    vh: f64, // high-pass gain factor
}

impl Biquad {
    /// Calculates the analog characteristics/parameters of this biquad filter.
    fn get_analog_params(&self) -> AnalogParams {
        let x11 =  self.a1 - 2.0;
        let x12 =  self.a1;
        let x1  = -self.a1 - 2.0;

        let x21 =  self.a2 - 1.0;
        let x22 =  self.a2 + 1.0;
        let x2  = -self.a2 + 1.0;

        let dx      = (x22 * x11) - (x12 * x21);
        let k_sq    = ((x22 * x1) - (x12 * x2)) / dx;
        let k_by_q  = ((x11 * x2) - (x21 * x1)) / dx;
        let a0      = 1.0 + k_by_q + k_sq;

        let k   = k_sq.sqrt();
        let q   = k / k_by_q;
        let vb  = 0.5 * a0 * (self.b0 - self.b2) / k_by_q;
        let vl  = 0.25 * a0 * (self.b0 + self.b1 + self.b2) / k_sq;
        let vh  = 0.25 * a0 * (self.b0 - self.b1 + self.b2);

        AnalogParams {
            k, q, vb, vl, vh
        }
    }

    /// Creates a new biquad filter with a new target sample rate that keeps the same analog characteristics.
    pub fn requantize(&self, source_sample_rate: u32, target_sample_rate: u32) -> Self {
        if target_sample_rate == source_sample_rate {
            // No work needed, return a copy of the original biquad.
            return *self
        }

        let ps = self.get_analog_params();

        let k       = ((source_sample_rate as f64 / target_sample_rate as f64) * ps.k.atan()).tan();
        let k_sq    = k * k;
        let k_by_q  = k / ps.q;
        let a0      = 1.0 + k_by_q + k_sq;

        let a1 = Util::den((2.0 * (k_sq - 1.0)) / a0);
        let a2 = Util::den((1.0 - k_by_q + k_sq) / a0);
        let b0 = Util::den((ps.vh + ps.vb * k_by_q + ps.vl * k_sq) / a0);
        let b1 = Util::den((2.0 * (ps.vl * k_sq - ps.vh)) / a0);
        let b2 = Util::den((ps.vh - ps.vb * k_by_q + ps.vl * k_sq) / a0);

        Biquad {
            a1, a2, b0, b1, b2,
        }
    }
}

// The ITU-R BS.1770-4 spec provides filter coefficient constants for both passes at a sample rate of 48000 Hz.
// Requantization is used to calculate the coefficient constants for different sample rates.
const REFERENCE_SAMPLE_RATE: u32 = 48000;
const REFERENCE_PASS_A: Biquad =
    Biquad {
        a1: -1.69065929318241,
        a2:  0.73248077421585,
        b0:  1.53512485958697,
        b1: -2.69169618940638,
        b2:  1.19839281085285,
    }
;
const REFERENCE_PASS_B: Biquad =
    Biquad {
        a1: -1.99004745483398,
        a2:  0.99007225036621,
        b0:  1.00000000000000,
        b1: -2.00000000000000,
        b2:  1.00000000000000,
    }
;

/// The initial two-pass "K"-filter as described by the ITU-R BS.1770-4 spec.
/// The first pass is a shelving filter, which accounts for the acoustic effects of the listener's (spherical) head.
/// The second pass is a simple high pass filter.
#[derive(Copy, Clone, Debug)]
pub struct Filter {
    sample_rate: u32,
    pass_a: Biquad,
    pass_b: Biquad,
}

impl Filter {
    pub fn new(sample_rate: u32) -> Self {
        let pass_a = REFERENCE_PASS_A.requantize(REFERENCE_SAMPLE_RATE, sample_rate);
        let pass_b = REFERENCE_PASS_B.requantize(REFERENCE_SAMPLE_RATE, sample_rate);

        Filter { sample_rate, pass_a, pass_b, }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn biquad_requantize() {
        let expected = Biquad {
            a1: -1.69065929318241,
            a2: 0.73248077421585,
            b0: 1.53512485958697,
            b1: -2.69169618940638,
            b2: 1.19839281085285,
        };
        let produced = REFERENCE_PASS_A.requantize(REFERENCE_SAMPLE_RATE, 48000);

        println!("{:?}, {:?}", expected, produced);
        assert_abs_diff_eq!(expected.a1, produced.a1);
        assert_abs_diff_eq!(expected.a2, produced.a2);
        assert_abs_diff_eq!(expected.b0, produced.b0);
        assert_abs_diff_eq!(expected.b1, produced.b1);
        assert_abs_diff_eq!(expected.b2, produced.b2);

        let expected = Biquad {
            a1: -1.6636551132560204,
            a2: 0.7125954280732254,
            b0: 1.5308412300503476,
            b1: -2.6509799951547293,
            b2: 1.1690790799215869,
        };
        let produced = REFERENCE_PASS_A.requantize(REFERENCE_SAMPLE_RATE, 44100);

        println!("{:?}, {:?}", expected, produced);
        assert_abs_diff_eq!(expected.a1, produced.a1);
        assert_abs_diff_eq!(expected.a2, produced.a2);
        assert_abs_diff_eq!(expected.b0, produced.b0);
        assert_abs_diff_eq!(expected.b1, produced.b1);
        assert_abs_diff_eq!(expected.b2, produced.b2);

        let expected = Biquad {
            a1: -0.2933807824149224,
            a2: 0.18687510604540813,
            b0: 1.3216235689299791,
            b1: -0.7262554913156887,
            b2: 0.2981262460162027,
        };
        let produced = REFERENCE_PASS_A.requantize(REFERENCE_SAMPLE_RATE, 8000);

        println!("{:?}, {:?}", expected, produced);
        assert_abs_diff_eq!(expected.a1, produced.a1);
        assert_abs_diff_eq!(expected.a2, produced.a2);
        assert_abs_diff_eq!(expected.b0, produced.b0);
        assert_abs_diff_eq!(expected.b1, produced.b1);
        assert_abs_diff_eq!(expected.b2, produced.b2);

        let expected = Biquad {
            a1: -1.9222022306074886,
            a2: 0.925117735116826,
            b0: 1.5722272150912788,
            b1: -3.0472830515615508,
            b2: 1.4779713409796091,
        };
        let produced = REFERENCE_PASS_A.requantize(REFERENCE_SAMPLE_RATE, 192000);

        println!("{:?}, {:?}", expected, produced);
        assert_abs_diff_eq!(expected.a1, produced.a1);
        assert_abs_diff_eq!(expected.a2, produced.a2);
        assert_abs_diff_eq!(expected.b0, produced.b0);
        assert_abs_diff_eq!(expected.b1, produced.b1);
        assert_abs_diff_eq!(expected.b2, produced.b2);
    }
}
