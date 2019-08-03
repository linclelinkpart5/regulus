//! Performs the initial two-pass "K"-filtering as described by the ITU-R BS.1770-4 spec.
//! The first pass is a shelving filter, which accounts for the acoustic effects of the listener's (spherical) head.
//! The second pass is a simple high pass filter.

/// Coefficients for a digital biquad filter, along with the sampling rate.
#[derive(Copy, Clone, Debug)]
pub struct Coefficients {
    sample_rate: u32,
    a1: f64,
    a2: f64,
    b0: f64,
    b1: f64,
    b2: f64,
}

// The ITU-R BS.1770-4 spec provides filter coefficient constants for both passes.
// However, these are specific for a sampling rate of 48000 Hz, and need to be recalculated if the rate differs!

pub const REFERENCE_SAMPLE_RATE: u32 = 48000;

pub const PASS_1_COEFFICIENTS: Coefficients =
    Coefficients {
        sample_rate: REFERENCE_SAMPLE_RATE,
        a1: -1.69065929318241,
        a2:  0.73248077421585,
        b0:  1.53512485958697,
        b1: -2.69169618940638,
        b2:  1.19839281085285,
    }
;

pub const PASS_2_COEFFICIENTS: Coefficients =
    Coefficients {
        sample_rate: REFERENCE_SAMPLE_RATE,
        a1: -1.99004745483398,
        a2:  0.99007225036621,
        b0:  1.00000000000000,
        b1: -2.00000000000000,
        b2:  1.00000000000000,
    }
;
