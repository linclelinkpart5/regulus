#[cfg(test)] #[macro_use] extern crate approx;

pub mod bin;
pub mod constants;
pub mod stats;
pub mod types;
pub mod util;
pub mod filter;
pub mod mean_sq;
pub mod gating;
#[cfg(test)] pub mod wave;

// #[derive(Clone, Copy, Debug)]
// enum NormKind {
//     ReplayGain,
//     ATSC,
//     EBU,
//     Custom(f64),
// }

// impl Default for NormKind {
//     fn default() -> Self {
//         NormKind::ReplayGain
//     }
// }

// impl NormKind {
//     fn level(&self) -> f64 {
//         match *self {
//             NormKind::ReplayGain => -18.0,
//             NormKind::ATSC => -24.0,
//             NormKind::EBU => -23.0,
//             NormKind::Custom(n) => n,
//         }
//     }

//     fn units(&self) -> &'static str {
//         match *self {
//             NormKind::ReplayGain => "dB",
//             _ => "LU",
//         }
//     }
// }

fn main() {
}
