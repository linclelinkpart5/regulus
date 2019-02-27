use crate::types::Partition;
use crate::constants::SILENCE_GATE;
use crate::constants::MOMENTARY_PARTITION;
use crate::constants::SHORTTERM_PARTITION;

#[derive(Debug)]
pub enum Error {
    InvalidPartition(Partition),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::InvalidPartition(p) => write!(f, "invalid partition: {}", p),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Error::InvalidPartition(..) => None,
        }
    }
}

pub enum BlockRing {
    Momentary([f64; MOMENTARY_PARTITION]),
    ShortTerm([f64; SHORTTERM_PARTITION]),
}

#[derive(Clone, Copy)]
/// ITU BS.1770 sliding block/aggregator.
pub struct Block {
    gate: f64,              // ITU BS.1770 silence gate
    length: f64,            // ITU BS.1170 block length in ms
    // partition: i32,         // ITU BS.1770 partition, e.g. 4 (75%)
    partition: Partition,   // ITU BS.1770 partition, e.g. 4 (75%)

    sample_rate: f64,
    overlap_size: usize,    // Depends on sample rate
    block_size: usize,      // Depends on sample rate
    scale: f64,             // Depends on block size, and thus sample rate
}

impl Block {
    // pub fn new(sample_rate: f64, ms: f64, partition: Partition) -> Result<Self, Error> {
    //     if partition == 0 {
    //         return Err(Error::InvalidPartition(partition));
    //     }

    //     let gate = SILENCE_GATE;
    //     let length = 0.001 * ms;

    //     let overlap_size = (length * sample_rate / partition as f64).round();
    //     let block_size = partition as f64 * overlap_size;
    //     let scale = 1.0 / block_size;

    //     let ring_size = partition;
    //     let ring_offset = 0;
    //     let ring_wmsq[block->ring.offs] = 0.0;
    //     let ring_count = 0;
    //     let ring_used = 1;

    //     unreachable!();
    // }
}

// struct lib1770_block {
//   lib1770_block_t *next;
//   lib1770_stats_t *stats;

//   double gate;          // ITU BS.1770 silence gate.
//   double length;        // ITU BS.1170 block length in ms
//   int partition;        // ITU BS.1770 partition, e.g. 4 (75%)

//   double samplerate;
//   size_t overlap_size;  // depends on samplerate
//   size_t block_size;    // depends on samplerate
//   double scale;         // depends on block size, i.e. on samplerate

//   struct {
//     size_t size;        // number of blocks in ring buffer.
//     size_t used;        // number of blocks used in ring buffer.
//     size_t count;       // number of samples processed in front block.
//     size_t offs;        // offset of front block.
//     double wmsq[0];     // allocated blocks.
//   } ring;
// };
