
use crate::constants::MAX_CHANNELS;

pub struct Stats {
    pub count: usize,
    pub mean: [f64; MAX_CHANNELS],
}

impl Stats {
    pub fn new() -> Self {
        Self {
            mean: [0.0f64; MAX_CHANNELS],
            count: 0,
        }
    }

    pub fn add(&mut self, sample: [f64; MAX_CHANNELS]) {
        if self.count == 0 {
            // If no existing samples have been analyzed, just store the new sample.
            self.mean = sample;
            self.count = 1;
        }
        else {
            // Calculate the incremental average.
            for ch in 0..MAX_CHANNELS {
                self.mean[ch] = (self.count as f64 * self.mean[ch] + sample[ch]) / (self.count + 1) as f64;
            }

            self.count += 1;
        }
    }

    pub fn merge(self, other: Self) -> Self {
        match (self.count, other.count) {
            (_, 0) => self,
            (0, _) => other,
            (n, m) => {
                let mut mean_new = [0.0f64; MAX_CHANNELS];

                for ch in 0..MAX_CHANNELS {
                    mean_new[ch] = (n as f64 * self.mean[ch] + m as f64 * other.mean[ch]) / (n + m) as f64;
                }

                Self {
                    mean: mean_new,
                    count: n + m,
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats() {
        let mut stats = Stats::new();
        assert_eq!(stats.count, 0);

        let expected = [0.0f64; MAX_CHANNELS];
        let produced = stats.mean;
        for ch in 0..expected.len().max(produced.len()) {
            assert_abs_diff_eq!(expected[ch], produced[ch]);
        }

        stats.add([0.1f64, 0.2, 0.3, 0.4, 0.5]);
        stats.add([0.1f64, 0.2, 0.3, 0.4, 0.5]);
        stats.add([0.1f64, 0.2, 0.3, 0.4, 0.5]);
        assert_eq!(stats.count, 3);

        let expected = [0.1, 0.2, 0.3, 0.4, 0.5];
        let produced = stats.mean;
        for ch in 0..expected.len().max(produced.len()) {
            assert_abs_diff_eq!(expected[ch], produced[ch]);
        }

        stats.add([1.0, 1.0, 1.0, 1.0, 1.0]);
        assert_eq!(stats.count, 4);

        let expected = [0.325, 0.4, 0.475, 0.55, 0.625];
        let produced = stats.mean;
        for ch in 0..expected.len().max(produced.len()) {
            assert_abs_diff_eq!(expected[ch], produced[ch]);
        }

        stats.add([-1.0, -0.5, 0.0, 0.5, 1.0]);
        assert_eq!(stats.count, 5);

        let expected = [0.06, 0.22, 0.38, 0.54, 0.7];
        let produced = stats.mean;
        for ch in 0..expected.len().max(produced.len()) {
            assert_abs_diff_eq!(expected[ch], produced[ch]);
        }

        stats.add([0.0, 0.2, 0.4, 0.6, 0.8]);
        assert_eq!(stats.count, 6);

        let expected = [0.05, 0.21666666666666667, 0.3833333333333333, 0.55, 0.7166666666666667];
        let produced = stats.mean;
        for ch in 0..expected.len().max(produced.len()) {
            assert_abs_diff_eq!(expected[ch], produced[ch]);
        }

        stats.add([1.0, 1.0, 1.0, 1.0, 1.0]);
        stats.add([1.0, 1.0, 1.0, 1.0, 1.0]);
        stats.add([1.0, 1.0, 1.0, 1.0, 1.0]);
        assert_eq!(stats.count, 9);

        let expected = [0.36666666666666664, 0.47777777777777775, 0.5888888888888889, 0.7, 0.8111111111111111];
        let produced = stats.mean;
        for ch in 0..expected.len().max(produced.len()) {
            assert_abs_diff_eq!(expected[ch], produced[ch]);
        }
    }
}
