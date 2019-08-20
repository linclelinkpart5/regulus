
const DEN_THRESHOLD: f64 = 1.0e-15;

pub struct Util;

impl Util {
    pub fn lufs(x: f64) -> f64 {
        -0.691 + 10.0 * x.log10()
    }

    pub fn lufs_hist(count: u64, sum: f64, reference: f64) -> f64 {
        match count == 0 {
            false => Util::lufs(sum / count as f64),
            true => reference,
        }
    }

    pub fn den(x: f64) -> f64 {
        if x.abs() < DEN_THRESHOLD { 0.0 }
        else { x }
    }

    /// Using a sample rate, calculates the number of samples in a given number of milliseconds.
    pub fn ms_to_samples(ms: u64, sample_rate: u32) -> u64 {
        let num = ms * sample_rate as u64;

        // Always round to the nearest sample.
        (num / 1000) + if num % 1000 >= 500 { 1 } else { 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn util_ms_to_samples() {
        let inputs_and_expected = vec![
            ((100, 44100), 4410),
            ((100, 44123), 4412),
            ((1, 44100), 44),
            ((1, 44600), 45),
            ((1, 44500), 45),
            ((1, 44499), 44),
            ((487, 12345), 6012),
            ((489, 12345), 6037),
        ];

        for (inputs, expected) in inputs_and_expected {
            let (ms, sample_rate) = inputs;
            let produced = Util::ms_to_samples(ms, sample_rate);

            assert_eq!(expected, produced)
        }
    }
}
