pub mod bin;
pub mod biquad;
pub mod block;
pub mod constants;
pub mod stats;
pub mod types;
pub mod util;
pub mod filter;

#[derive(Clone, Copy, Debug)]
struct MBlockOptions {
    ms: f64,
    partition: i32,
    mean_gate: f64,
    range_gate: f64,
    range_lower_bound: f64,
    range_upper_bound: f64,
}

impl Default for MBlockOptions {
    fn default() -> Self {
        MBlockOptions {
            ms: 400.0,
            partition: 4,
            mean_gate: -10.0,
            range_gate: -20.0,
            range_lower_bound: 0.1,
            range_upper_bound: 0.95,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct SBlockOptions {
    ms: f64,
    partition: i32,
    mean_gate: f64,
    range_gate: f64,
    range_lower_bound: f64,
    range_upper_bound: f64,
}

impl Default for SBlockOptions {
    fn default() -> Self {
        SBlockOptions {
            ms: 3000.0,
            partition: 3,
            mean_gate: -10.0,
            range_gate: -20.0,
            range_lower_bound: 0.1,
            range_upper_bound: 0.95,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct CalcOptions {
    momentary_mean: bool,  // "-i/--integrated"
    momentary_max: bool,  // "-m/--momentary"
    momentary_range: bool,
    shortterm_mean: bool,
    shortterm_max: bool,  // "-s/--shortterm"
    shortterm_range: bool,  // "-r/--range"
    samplepeak: bool,
    truepeak: bool,
}

impl Default for CalcOptions {
    fn default() -> Self {
        CalcOptions {
            momentary_mean: true,
            momentary_max: false,
            momentary_range: false,
            shortterm_mean: false,
            shortterm_max: false,
            shortterm_range: false,
            samplepeak: false,
            truepeak: false,
        }
    }
}

impl CalcOptions {
    fn is_noop(&self) -> bool {
        match *self {
            Self {
                momentary_mean: false,
                momentary_max: false,
                momentary_range: false,
                shortterm_mean: false,
                shortterm_max: false,
                shortterm_range: false,
                samplepeak: false,
                truepeak: false,
            } => false,
            _ => true,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum NormKind {
    ReplayGain,
    ATSC,
    EBU,
    Custom(f64),
}

impl Default for NormKind {
    fn default() -> Self {
        NormKind::ReplayGain
    }
}

impl NormKind {
    fn level(&self) -> f64 {
        match *self {
            NormKind::ReplayGain => -18.0,
            NormKind::ATSC => -24.0,
            NormKind::EBU => -23.0,
            NormKind::Custom(n) => n,
        }
    }

    fn units(&self) -> &'static str {
        match *self {
            NormKind::ReplayGain => "dB",
            _ => "LU",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Options {
    norm: NormKind,
    // preamp: f64,
    // drc: f64,
    // begin: u64,
    // duration: u64,
    calc: CalcOptions,
    momentary: MBlockOptions,
    shortterm: SBlockOptions,
}

fn main() {
}
