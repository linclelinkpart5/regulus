use crate::util::Util;

// https://github.com/mzuther/K-Meter/blob/master/doc/specifications/ITU-R%20BS.1770-1%20(Filters).pdf
#[derive(Clone, Copy, Debug)]
struct BiquadPs {
    k: f64, // a.k.a. Ω, tan(pi * Fc / Fs), Fc = target sample rate, Fs = source sample rate
    q: f64, // Q factor
    vb: f64, // band-pass gain factor
    vl: f64, // low-pass gain factor
    vh: f64, // high-pass gain factor
}

// Coefficients for a biquad digital filter.
// It is assumed that `a0` is always equal to 1.0, and thus not included.
#[derive(Clone, Copy, Debug)]
struct Biquad {
    sample_rate: f64,
    a1: f64,
    a2: f64,
    b0: f64,
    b1: f64,
    b2: f64,
}

impl Biquad {
    fn get_ps(&self) -> BiquadPs {
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

        BiquadPs {
            k,
            q,
            vb,
            vl,
            vh,
        }
    }

    fn requantize(&self, new_sample_rate: f64) -> Biquad {
        if new_sample_rate == self.sample_rate {
            // No work needed, return a copy of the original biquad.
            return *self
        }

        let ps = self.get_ps();

        let k       = ((self.sample_rate / new_sample_rate) * ps.k.atan()).tan();
        let k_sq    = k * k;
        let k_by_q  = k / ps.q;
        let a0      = 1.0 + k_by_q + k_sq;

        let sample_rate = new_sample_rate;
        let a1 = Util::den((2.0 * (k_sq - 1.0)) / a0);
        let a2 = Util::den((1.0 - k_by_q + k_sq) / a0);
        let b0 = Util::den((ps.vh + ps.vb * k_by_q + ps.vl * k_sq) / a0);
        let b1 = Util::den((2.0 * (ps.vl * k_sq - ps.vh)) / a0);
        let b2 = Util::den((ps.vh - ps.vb * k_by_q + ps.vl * k_sq) / a0);

        Biquad {
            sample_rate,
            a1,
            a2,
            b0,
            b1,
            b2,
        }
    }
}
