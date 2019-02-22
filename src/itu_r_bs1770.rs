const BUF_SIZE: usize = 9;
const MAX_CHANNELS: usize = 5;

const SILENCE: f64 = -70.0;
const SILENCE_GATE: f64 = 1.1724653e-7; // 10.0.powf(0.1 * (0.691 + SILENCE));

const DEN_THRESHOLD: f64 = 1.0e-15;

type Sample = [f64; MAX_CHANNELS];

fn lufs(x: f64) -> f64 {
    -0.691 + 10.0 * x.log10()
}

fn lufs_hist(count: u64, sum: f64, reference: f64) -> f64 {
    match count == 0 {
        false => lufs(sum / count as f64),
        true => reference,
    }
}

fn den(x: f64) -> f64 {
    if x.abs() < DEN_THRESHOLD { 0.0 }
    else { x }
}

struct Biquad {
    sample_rate: f64,
    a1: f64,
    a2: f64,
    b0: f64,
    b1: f64,
    b2: f64,
}

impl Biquad {
    fn get_ps(&self) -> BiquadPs {
        let x11 =  self.a1 - 2.0;
        let x12 =  self.a1;
        let x1  = -self.a1 - 2.0;

        let x21 =  self.a2 - 1.0;
        let x22 =  self.a2 + 1.0;
        let x2  = -self.a2 + 1.0;

        let dx      = (x22 * x11) - (x12 * x21);
        let k_sq    = ((x22 * x1) - (x12 * x2)) / dx;
        let k_by_q  = ((x11 * x2) - (x21 * x1)) / dx;
        let a0      = 1.0 + k_by_q + k_sq;

        let k   = k_sq.sqrt();
        let q   = k / k_by_q;
        let vb  = 0.5 * a0 * (self.b0 - self.b2) / k_by_q;
        let vl  = 0.25 * a0 * (self.b0 + self.b1 + self.b2) / k_sq;
        let vh  = 0.25 * a0 * (self.b0 - self.b1 + self.b2);

        BiquadPs {
            k,
            q,
            vb,
            vl,
            vh,
        }
    }
}

struct BiquadPs {
    k: f64,
    q: f64,
    vb: f64,
    vl: f64,
    vh: f64,
}

struct Bin {
    db: f64,
    x: f64,
    y: f64,
    count: u64,
}

struct Stats {

}
