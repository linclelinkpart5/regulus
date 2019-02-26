use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Default)]
pub struct Bin {
    pub db: f64,
    pub x: f64,
    pub y: f64,
    pub count: u64,
}

impl Bin {
    pub fn wmsq_cmp(&self, wmsq: f64) -> Ordering {
        if wmsq < self.x {
            Ordering::Less
        }
        else if self.y == 0.0 {
            Ordering::Equal
        }
        else if self.y <= wmsq {
            Ordering::Greater
        }
        else {
            Ordering::Equal
        }
    }
}
