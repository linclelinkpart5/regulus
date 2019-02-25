const BUF_SIZE: usize = 9;
const MAX_CHANNELS: usize = 5;

const SILENCE: f64 = -70.0;
const SILENCE_GATE: f64 = 1.1724653e-7; // 10.0.powf(0.1 * (0.691 + SILENCE));

const DEN_THRESHOLD: f64 = 1.0e-15;

const HIST_MIN: i32 = -70;
const HIST_MAX: i32 = 5;
const HIST_GRAIN: i32 = 100;
const HIST_NBINS: usize = HIST_GRAIN as usize * (HIST_MAX - HIST_MIN) as usize + 1;

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

#[derive(Clone, Copy, Debug)]
struct BiquadPs {
    k: f64,
    q: f64,
    vb: f64,
    vl: f64,
    vh: f64,
}

#[derive(Clone, Copy, Debug)]
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

    fn requantize(&self, new_sample_rate: f64) -> Biquad {
        if new_sample_rate == self.sample_rate {
            // No work needed, return a copy of the original biquad.
            return *self
        }

        let ps = self.get_ps();

        let k       = ((self.sample_rate / new_sample_rate) * ps.k.atan()).tan();
        let k_sq    = k * k;
        let k_by_q  = k / ps.q;
        let a0      = 1.0 + k_by_q + k_sq;

        let sample_rate = new_sample_rate;
        let a1 = den((2.0 * (k_sq - 1.0)) / a0);
        let a2 = den((1.0 - k_by_q + k_sq) / a0);
        let b0 = den((ps.vh + ps.vb * k_by_q + ps.vl * k_sq) / a0);
        let b1 = den((2.0 * (ps.vl * k_sq - ps.vh)) / a0);
        let b2 = den((ps.vh - ps.vb * k_by_q + ps.vl * k_sq) / a0);

        Biquad {
            sample_rate,
            a1,
            a2,
            b0,
            b1,
            b2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Bin {
    db: f64,
    x: f64,
    y: f64,
    count: u64,
}

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
