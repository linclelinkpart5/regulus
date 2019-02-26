use std::cmp::Ordering;

use crate::constants::SILENCE;
use crate::constants::SILENCE_GATE;
use crate::bin::Bin;
use crate::util::Util;

const HIST_MIN: i32 = -70;
const HIST_MAX: i32 = 5;
const HIST_GRAIN: i32 = 100;
const HIST_NBINS: usize = HIST_GRAIN as usize * (HIST_MAX - HIST_MIN) as usize + 1;

#[derive(Clone, Copy)]
struct Stats {
    max_wmsq: f64,
    wmsq: f64,
    count: u64,
    bins: [Bin; HIST_NBINS],
}

impl Default for Stats {
    fn default() -> Self {
        let max_wmsq = SILENCE_GATE;

        let step: f64 = 1.0 / HIST_GRAIN as f64;

        let to_copy = Bin::default();
        let mut bins: [Bin; HIST_NBINS] = [to_copy; HIST_NBINS];

        for i in 0..HIST_NBINS {
            let db = (step * i as f64) + HIST_MIN as f64;
            let wsmq = 10.0f64.powf(0.1 * (0.691 + db));

            bins[i].db = db;
            bins[i].x = wsmq;
            bins[i].y = 0.0;
            bins[i].count = 0;

            if i > 0 {
                bins[i - 1].y = wsmq;
            }
        }

        let wmsq = 0.0;
        let count = 0;

        Stats {
            max_wmsq,
            wmsq,
            count,
            bins,
        }
    }
}

impl Stats {
    fn merge(&self, other: &Self) -> Self {
        let new_max_wmsq = self.max_wmsq.max(other.max_wmsq);
        let new_count = self.count + other.count;

        let (new_wmsq, new_bins) = if new_count > 0 {
            let q1 = self.count as f64 / new_count as f64;
            let q2 = other.count as f64 / new_count as f64;

            let new_wmsq = q1 * self.wmsq + q2 * other.wmsq;

            let mut new_bins = self.bins.clone();

            for i in 0..HIST_NBINS {
                new_bins[i].count += other.bins[i].count;
            }

            (new_wmsq, new_bins)
        }
        else {
            (self.wmsq, self.bins)
        };

        Stats {
            max_wmsq: new_max_wmsq,
            wmsq: new_wmsq,
            count: new_count,
            bins: new_bins,
        }
    }

    fn add_sqs(&self, wmsq: f64) -> Self {
        let new_max_wmsq = self.max_wmsq.max(wmsq);

        for (i, bin) in self.bins.iter().enumerate() {
            if bin.wmsq_cmp(wmsq) == Ordering::Equal {
                let mut new_bins = self.bins.clone();

                let new_wmsq: f64 = self.wmsq + ((wmsq - self.wmsq) / self.count as f64);

                let new_count = self.count + 1;

                new_bins[i].count += 1;

                return Stats {
                    max_wmsq: new_max_wmsq,
                    wmsq: new_wmsq,
                    count: new_count,
                    bins: new_bins,
                }
            }
        }

        let mut new_stats = self.clone();
        new_stats.max_wmsq = new_max_wmsq;
        new_stats
    }

    fn get_max(&self) -> f64 {
        Util::lufs(self.max_wmsq)
    }

    fn get_mean(&self, gate: f64) -> f64 {
        let gate = self.wmsq * 10.0f64.powf(0.1 * gate);

        let mut sum: f64 = 0.0;
        let mut count: u64 = 0;

        for bin in self.bins.iter() {
            if bin.count > 0 && gate < bin.x {
                sum += bin.count as f64 * bin.x;
                count += bin.count;
            }
        }

        Util::lufs_hist(count, sum, SILENCE)
    }

    fn get_range(&self, gate: f64, lower: f64, upper: f64) -> f64 {
        let gate = self.wmsq * 10.0f64.powf(0.1 * gate);

        // Ensure lower < upper.
        let (lower, upper) = {
            if lower > upper { (upper, lower) }
            else { (lower, upper) }
        };

        // Ensure lower and upper are clipped to [0.0, 1.0].
        let lower = 0.0f64.max(lower);
        let upper = 1.0f64.min(upper);

        let mut count: u64 = 0;

        for bin in self.bins.iter() {
            if bin.count > 0 && gate < bin.x {
                count += bin.count;
            }
        }

        if count > 0 {
            let lower_count: u64 = (count as f64 * lower) as u64;
            let upper_count: u64 = (count as f64 * upper) as u64;
            let mut prev_count: u64 = u64::max_value();

            let mut min_db = 0.0f64;
            let mut max_db = 0.0f64;

            // Reuse the count variable.
            count = 0;

            for bin in self.bins.iter() {
                if bin.x > gate {
                    count += bin.count;
                }

                if prev_count < lower_count && lower_count <= count {
                    min_db = bin.db;
                }

                if prev_count < upper_count && upper_count <= count {
                    max_db = bin.db;
                    break;
                }

                prev_count = count;
            }

            max_db - min_db
        }
        else {
            0.0
        }
    }
}
