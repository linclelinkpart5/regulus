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
