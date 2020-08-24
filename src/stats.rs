
use crate::constants::MAX_CHANNELS;

#[derive(Copy, Clone)]
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

    pub fn add(&mut self, sample: &[f64; MAX_CHANNELS]) {
        if self.count == 0 {
            // If no existing samples have been analyzed, just store the new sample.
            self.mean = *sample;
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

    pub fn extend<I>(&mut self, samples: I)
    where
        I: IntoIterator<Item = [f64; MAX_CHANNELS]>
    {
        for sample in samples {
            self.add(&sample);
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

    use approx::assert_abs_diff_eq;

    fn validate(expected_mean: [f64; MAX_CHANNELS], expected_count: usize, produced: Stats) {
        let produced_mean = produced.mean;
        let produced_count = produced.count;

        for ch in 0..expected_mean.len().max(produced_mean.len()) {
            assert_abs_diff_eq!(expected_mean[ch], produced_mean[ch]);
        }
        assert_eq!(expected_count, produced_count);
    }

    #[test]
    fn stats_add() {
        const INITIAL: [f64; MAX_CHANNELS] = [0.1, 0.2, 0.3, 0.4, 0.5];

        let mut stats = Stats::new();
        validate([0.0; MAX_CHANNELS], 0, stats);

        stats.add(&INITIAL);
        validate(INITIAL, 1, stats);

        stats.add(&INITIAL);
        stats.add(&INITIAL);
        validate(INITIAL, 3, stats);

        stats.add(&[1.0, 1.0, 1.0, 1.0, 1.0]);
        validate([1.3 / 4.0, 1.6 / 4.0, 1.9 / 4.0, 2.2 / 4.0, 2.5 / 4.0], 4, stats);

        stats.add(&[-1.0, -0.5, 0.0, 0.5, 1.0]);
        validate([0.3 / 5.0, 1.1 / 5.0, 1.9 / 5.0, 2.7 / 5.0, 3.5 / 5.0], 5, stats);

        stats.add(&[0.0, 0.2, 0.4, 0.6, 0.8]);
        validate([0.3 / 6.0, 1.3 / 6.0, 2.3 / 6.0, 3.3 / 6.0, 4.3 / 6.0], 6, stats);

        stats.add(&[1.0, 1.0, 1.0, 1.0, 1.0]);
        stats.add(&[1.0, 1.0, 1.0, 1.0, 1.0]);
        stats.add(&[1.0, 1.0, 1.0, 1.0, 1.0]);
        validate([3.3 / 9.0, 4.3 / 9.0, 5.3 / 9.0, 6.3 / 9.0, 7.3 / 9.0], 9, stats);
    }

    #[test]
    fn stats_merge() {
        let mut stats_a = Stats::new();
        stats_a.add(&[0.1, 0.2, 0.3, 0.4, 0.5]);
        stats_a.add(&[0.6, 0.7, 0.8, 0.9, 1.0]);

        let mut stats_b = Stats::new();
        stats_b.add(&[0.01, 0.02, 0.03, 0.04, 0.05]);
        stats_b.add(&[0.06, 0.07, 0.08, 0.09, 0.10]);

        let merged = stats_a.merge(stats_b);
        validate([0.77 / 4.0, 0.99 / 4.0, 1.21 / 4.0, 1.43 / 4.0, 1.65 / 4.0], 4, merged);

        let merged = stats_b.merge(stats_a);
        validate([0.77 / 4.0, 0.99 / 4.0, 1.21 / 4.0, 1.43 / 4.0, 1.65 / 4.0], 4, merged);

        let stats_b = Stats::new();

        let merged = stats_a.merge(stats_b);
        validate(stats_a.mean, stats_a.count, merged);

        let merged = stats_b.merge(stats_a);
        validate(stats_a.mean, stats_a.count, merged);

        let stats_a = Stats::new();

        let merged = stats_a.merge(stats_b);
        validate([0.0; MAX_CHANNELS], 0, merged);
    }
}
