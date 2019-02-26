pub const SILENCE: f64 = -70.0;
pub const SILENCE_GATE: f64 = 1.1724653e-7; // 10.0.powf(0.1 * (0.691 + SILENCE));

pub const BUF_SIZE: usize = 9;
pub const MAX_CHANNELS: usize = 5;

pub const DEN_THRESHOLD: f64 = 1.0e-15;
