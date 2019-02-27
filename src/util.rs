use crate::constants::DEN_THRESHOLD;
use crate::constants::MAX_CHANNELS;

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
}
