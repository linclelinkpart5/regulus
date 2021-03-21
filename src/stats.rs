use sampara::{Frame, Signal};

#[derive(Copy, Clone)]
pub struct Stats<F, const N: usize>
where
    F: Frame<N, Sample = f64>,
{
    pub count: usize,
    pub mean: F,
}

impl<F, const N: usize> Stats<F, N>
where
    F: Frame<N, Sample = f64>,
{
    pub fn new() -> Self {
        Self {
            mean: Frame::EQUILIBRIUM,
            count: 0,
        }
    }

    pub fn add(&mut self, frame: F) {
        if self.count == 0 {
            // If no frames have been analyzed yet, just store the new frame.
            self.mean = frame;
            self.count = 1;
        }
        else {
            // Calculate the incremental average.
            let old_count = self.count as f64;
            let new_count = (self.count + 1) as f64;
            self.mean.zip_transform(frame, |m, f| (old_count * m + f) / new_count);

            self.count += 1;
        }
    }

    pub fn extend<S>(&mut self, signal: S)
    where
        S: Signal<N, Frame = F>
    {
        for frame in signal.into_iter() {
            self.add(frame);
        }
    }

    pub fn merge(self, other: Self) -> Self {
        match (self.count, other.count) {
            (_, 0) => self,
            (0, _) => other,
            (n, m) => {
                let mut merged_mean = self.mean;

                let nf = n as f64;
                let mf = m as f64;
                let nmf = (n + m) as f64;

                merged_mean.zip_transform(other.mean, |s, o| {
                    (nf * s + mf * o) / nmf
                });

                Self {
                    mean: merged_mean,
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

    fn validate<F, const N: usize>(expected_mean: F, expected_count: usize, produced: Stats<F, N>)
    where
        F: Frame<N, Sample = f64>,
    {
        let produced_mean = produced.mean;
        let produced_count = produced.count;

        for (e, p) in expected_mean.into_channels().zip(produced_mean.into_channels()) {
            assert_abs_diff_eq!(e, p);
        }
        assert_eq!(expected_count, produced_count);
    }

    #[test]
    fn stats_add() {
        const INITIAL: [f64; 5] = [0.1, 0.2, 0.3, 0.4, 0.5];

        let mut stats = Stats::new();
        validate([0.0; 5], 0, stats);

        stats.add(INITIAL);
        validate(INITIAL, 1, stats);

        stats.add(INITIAL);
        stats.add(INITIAL);
        validate(INITIAL, 3, stats);

        stats.add([1.0, 1.0, 1.0, 1.0, 1.0]);
        validate([1.3 / 4.0, 1.6 / 4.0, 1.9 / 4.0, 2.2 / 4.0, 2.5 / 4.0], 4, stats);

        stats.add([-1.0, -0.5, 0.0, 0.5, 1.0]);
        validate([0.3 / 5.0, 1.1 / 5.0, 1.9 / 5.0, 2.7 / 5.0, 3.5 / 5.0], 5, stats);

        stats.add([0.0, 0.2, 0.4, 0.6, 0.8]);
        validate([0.3 / 6.0, 1.3 / 6.0, 2.3 / 6.0, 3.3 / 6.0, 4.3 / 6.0], 6, stats);

        stats.add([1.0, 1.0, 1.0, 1.0, 1.0]);
        stats.add([1.0, 1.0, 1.0, 1.0, 1.0]);
        stats.add([1.0, 1.0, 1.0, 1.0, 1.0]);
        validate([3.3 / 9.0, 4.3 / 9.0, 5.3 / 9.0, 6.3 / 9.0, 7.3 / 9.0], 9, stats);
    }

    #[test]
    fn stats_merge() {
        let mut stats_a = Stats::new();
        stats_a.add([0.1, 0.2, 0.3, 0.4, 0.5]);
        stats_a.add([0.6, 0.7, 0.8, 0.9, 1.0]);

        let mut stats_b = Stats::new();
        stats_b.add([0.01, 0.02, 0.03, 0.04, 0.05]);
        stats_b.add([0.06, 0.07, 0.08, 0.09, 0.10]);

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
        validate([0.0; 5], 0, merged);
    }
}
