use crate::constants::MAX_CHANNELS;

pub struct MeanSquare {
    curr: [f64; MAX_CHANNELS],
    num: usize,
}

impl MeanSquare {
    pub fn new() -> Self {
        Self {
            curr: [0.0f64; MAX_CHANNELS],
            num: 0,
        }
    }

    pub fn mean_sqs(&self) -> [f64; MAX_CHANNELS] {
        self.curr
    }

    pub fn num_samples(&self) -> usize {
        self.num
    }

    pub fn add_samples(&mut self, samples: &[[f64; MAX_CHANNELS]]) {
        let n = self.num;
        let m = samples.len();

        // Bail out early if no work needs to be done.
        if m == 0 { return }

        let mut summed_sqs = [0.0f64; MAX_CHANNELS];
        for sample in samples {
            for ch in 0..MAX_CHANNELS {
                summed_sqs[ch] += sample[ch] * sample[ch];
            }
        }

        if n == 0 {
            // If no samples have been stored, just save the summed samples and their count.
            self.curr = summed_sqs;
            self.num = m;
        } else {
            // These calculations are for a running incremental average.
            let n_p_m = n + m;

            let mut next = [0.0f64; MAX_CHANNELS];

            for ch in 0..MAX_CHANNELS {
                next[ch] = (n as f64 * self.curr[ch] + summed_sqs[ch]) / n_p_m as f64;
            }

            self.curr = next;
            self.num = n_p_m;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_square() {
        let mut mean_sq = MeanSquare::new();
        assert_eq!(mean_sq.num_samples(), 0);

        let expected = [0.0f64; MAX_CHANNELS];
        let produced = mean_sq.mean_sqs();
        for ch in 0..expected.len().max(produced.len()) {
            assert_abs_diff_eq!(expected[ch], produced[ch]);
        }

        mean_sq.add_samples(&[[1.0, 1.0, 1.0, 1.0, 1.0]]);
        assert_eq!(mean_sq.num_samples(), 1);

        let expected = [1.0f64; MAX_CHANNELS];
        let produced = mean_sq.mean_sqs();
        for ch in 0..expected.len().max(produced.len()) {
            assert_abs_diff_eq!(expected[ch], produced[ch]);
        }

        mean_sq.add_samples(&[[-1.0, -0.5, 0.0, 0.5, 1.0]]);
        assert_eq!(mean_sq.num_samples(), 2);

        let expected = [1.0, 0.625, 0.5, 0.625, 1.0];
        let produced = mean_sq.mean_sqs();
        for ch in 0..expected.len().max(produced.len()) {
            assert_abs_diff_eq!(expected[ch], produced[ch]);
        }

        mean_sq.add_samples(&[[0.0, 0.2, 0.4, 0.6, 0.8]]);
        assert_eq!(mean_sq.num_samples(), 3);

        let expected = [0.6666666666666666, 0.43, 0.3866666666666667, 0.5366666666666666, 0.88];
        let produced = mean_sq.mean_sqs();
        for ch in 0..expected.len().max(produced.len()) {
            assert_abs_diff_eq!(expected[ch], produced[ch]);
        }

        mean_sq.add_samples(&[
            [1.0, 1.0, 1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0, 1.0],
        ]);
        assert_eq!(mean_sq.num_samples(), 6);

        let expected = [0.8333333333333334, 0.715, 0.6933333333333334, 0.7683333333333332, 0.9400000000000001];
        let produced = mean_sq.mean_sqs();
        for ch in 0..expected.len().max(produced.len()) {
            assert_abs_diff_eq!(expected[ch], produced[ch]);
        }
    }
}
