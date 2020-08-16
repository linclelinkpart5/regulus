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
struct Params {
    a1: f64,
    a2: f64,
    b0: f64,
    b1: f64,
    b2: f64,
}

impl Params {
    pub fn new(kind: Kind, sample_rate: u32) -> Self {
        let g = 3.999843853973347;

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
                    let vh = 10.0f64.powf(g / 20.0);
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
impl AbsDiffEq for Params {
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
struct Filter {
    ps: Params,
    s1: [f64; MAX_CHANNELS],
    s2: [f64; MAX_CHANNELS],
}

impl Filter {
    fn new(kind: Kind, sample_rate: u32) -> Self {
        Self {
            ps: Params::new(kind, sample_rate),
            s1: [0.0; MAX_CHANNELS],
            s2: [0.0; MAX_CHANNELS],
        }
    }

    pub fn apply(&mut self, input: &[f64; MAX_CHANNELS]) -> [f64; MAX_CHANNELS] {
        let mut output = [0.0f64; MAX_CHANNELS];

        // https://www.earlevel.com/main/2012/11/26/biquad-c-source-code/
        for ch in 0..MAX_CHANNELS {
            let out = self.s1[ch] + self.ps.b0 * input[ch];
            self.s1[ch] = self.s2[ch] + self.ps.b1 * input[ch] - self.ps.a1 * out;
            self.s2[ch] = self.ps.b2 * input[ch] - self.ps.a2 * out;

            output[ch] = out;
        }

        output
    }
}

/// The initial two-pass "K"-filter as described by the ITU-R BS.1770-4 spec.
/// The first pass is a shelving filter, which accounts for the acoustic effects of the listener's (spherical) head.
/// The second pass is a simple high pass filter.
#[derive(Copy, Clone, Debug)]
struct FilterChain {
    sample_rate: u32,
    pass_a: Filter,
    pass_b: Filter,
}

impl FilterChain {
    pub fn new(sample_rate: u32) -> Self {
        let pass_a = Filter::new(Kind::A, sample_rate);
        let pass_b = Filter::new(Kind::B, sample_rate);

        Self { sample_rate, pass_a, pass_b, }
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
