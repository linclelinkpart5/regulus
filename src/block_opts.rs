use crate::types::Partition;

#[derive(Clone, Copy, Debug)]
pub struct BlockOptions {
    pub ms: f64,
    pub partition: Partition,
    pub mean_gate: f64,
    pub range_gate: f64,
    pub range_lower_bound: f64,
    pub range_upper_bound: f64,
}

pub const MOMENTARY_BLOCK_OPTS: BlockOptions = BlockOptions {
    ms: 400.0,
    partition: 4,
    mean_gate: -10.0,
    range_gate: -20.0,
    range_lower_bound: 0.1,
    range_upper_bound: 0.95,
};

pub const SHORTTERM_BLOCK_OPTS: BlockOptions = BlockOptions {
    ms: 3000.0,
    partition: 3,
    mean_gate: -10.0,
    range_gate: -20.0,
    range_lower_bound: 0.1,
    range_upper_bound: 0.95,
};
