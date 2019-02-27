use crate::types::Partition;

pub const SILENCE: f64 = -70.0;
pub const SILENCE_GATE: f64 = 1.1724653e-7; // 10.0.powf(0.1 * (0.691 + SILENCE));

pub const BUF_SIZE: usize = 9;
pub const MAX_CHANNELS: usize = 5;

pub const DEN_THRESHOLD: f64 = 1.0e-15;

pub const MOMENTARY_MS: f64 = 400.0;
pub const MOMENTARY_PARTITION: Partition = 4;
pub const MOMENTARY_MEAN_GATE: f64 = -10.0;
pub const MOMENTARY_RANGE_GATE: f64 = -20.0;
pub const MOMENTARY_RANGE_LOWER_BOUND: f64 = 0.1;
pub const MOMENTARY_RANGE_UPPER_BOUND: f64 = 0.95;

pub const SHORTTERM_MS: f64 = 3000.0;
pub const SHORTTERM_PARTITION: Partition = 3;
pub const SHORTTERM_MEAN_GATE: f64 = -10.0;
pub const SHORTTERM_RANGE_GATE: f64 = -20.0;
pub const SHORTTERM_RANGE_LOWER_BOUND: f64 = 0.1;
pub const SHORTTERM_RANGE_UPPER_BOUND: f64 = 0.95;
