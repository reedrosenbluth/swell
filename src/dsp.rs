use math::round::floor;
use std::{
    f64::consts::PI,
    sync::{Arc, Mutex},
};

pub const TAU64: f64 = 2.0 * PI;
pub const TAU32: f32 = TAU64 as f32;

pub type Phase = f64;
pub type Hz = f64;
pub type Amp = f32;

pub trait Signal {
    fn signal_add(&mut self, sample_rate: f64, add: Phase) -> Amp;

    fn signal(&mut self, sample_rate: f64) -> Amp {
        self.signal_add(sample_rate, 0.0)
    }
}

pub type ArcWave = Arc<Mutex<dyn Signal + Send>>;
pub type ArcMutex<T> = Arc<Mutex<T>>;

pub fn arc<T>(x: T) -> Arc<Mutex<T>> {
    Arc::new(Mutex::new(x))
}

#[derive(Clone)]
pub struct SineWave {
    pub hz: Hz,
    pub amplitude: Amp,
    pub phase: Phase,
}

impl SineWave {
    pub fn new(hz: Hz) -> Self {
        Self {
            hz,
            amplitude: 1.0,
            phase: 0.0,
        }
    }

    pub fn wrapped(hz: Hz) -> ArcMutex<Self> {
        arc(Self::new(hz))
    }
}

impl Signal for SineWave {
    fn signal_add(&mut self, sample_rate: f64, add: Phase) -> Amp {
        let amp = self.amplitude * (TAU32 * self.phase as f32).sin();
        self.phase += (self.hz + add * self.hz) / sample_rate;
        self.phase %= sample_rate;
        amp
    }
}

#[derive(Clone)]
pub struct SquareWave {
    pub hz: Hz,
    pub amplitude: Amp,
    pub phase: Phase,
}

impl SquareWave {
    pub fn new(hz: Hz) -> Self {
        Self {
            hz,
            amplitude: 1.0,
            phase: 0.0,
        }
    }

    pub fn wrapped(hz: Hz) -> ArcMutex<Self> {
        arc(Self::new(hz))
    }
}

impl Signal for SquareWave {
    fn signal_add(&mut self, sample_rate: f64, add: Phase) -> Amp {
        let t = self.phase - floor(self.phase, 0);
        let amp = if t < 0.001 {
            0.0
        } else if t <= 0.5 {
            self.amplitude
        } else {
            -self.amplitude
        };
        self.phase += (self.hz + add * self.hz) / sample_rate;
        self.phase %= sample_rate;
        amp
    }
}

#[derive(Clone)]
pub struct SawWave {
    pub hz: Hz,
    pub amplitude: Amp,
    pub phase: Phase,
}

impl SawWave {
    pub fn new(hz: Hz) -> Self {
        Self {
            hz,
            amplitude: 1.0,
            phase: 0.0,
        }
    }

    pub fn wrapped(hz: Hz) -> ArcMutex<Self> {
        arc(Self::new(hz))
    }
}

impl Signal for SawWave {
    fn signal_add(&mut self, sample_rate: f64, add: Phase) -> Amp {
        let t = self.phase - 0.5;
        let s = -t - floor(0.5 - t, 0);
        let amp = if s < -0.499 {
            0.0
        } else {
            self.amplitude * 2. * s as f32
        };
        self.phase += (self.hz + add * self.hz) / sample_rate;
        self.phase %= sample_rate;
        amp
    }
}

#[derive(Clone)]
pub struct TriangleWave {
    pub hz: Hz,
    pub amplitude: Amp,
    pub phase: Phase,
}

impl TriangleWave {
    pub fn new(hz: Hz) -> Self {
        Self {
            hz,
            amplitude: 1.0,
            phase: 0.0,
        }
    }

    pub fn wrapped(hz: Hz) -> ArcMutex<Self> {
        arc(Self::new(hz))
    }
}

impl Signal for TriangleWave {
    fn signal_add(&mut self, sample_rate: f64, add: Phase) -> Amp {
        let t = self.phase - 0.75;
        let saw_amp = (2. * (-t - floor(0.5 - t, 0))) as f32;
        let amp = (2. * saw_amp.abs() - self.amplitude) * self.amplitude;
        self.phase += (self.hz + add * self.hz) / sample_rate;
        self.phase %= sample_rate;
        amp
    }
}

// pub struct FourierWave(pub PolyWave);
pub struct FourierWave {
    pub hz: Hz,
    pub amplitude: Amp,
    pub phase: Phase,
    pub sines: Vec<SineWave>,
}

impl FourierWave {
    pub fn new(coefficients: &[f32], hz: Hz) -> Self {
        let mut wwaves: Vec<SineWave> = Vec::new();
        for (n, c) in coefficients.iter().enumerate() {
            let mut s = SineWave::new(hz * n as f64);
            s.amplitude = *c;
            wwaves.push(s);
        }
        FourierWave {
            hz,
            amplitude: 1.0,
            phase: 0.0,
            sines: wwaves,
        }
    }

    pub fn wrapped(coefficients: &[Amp], hz: Hz) -> ArcMutex<Self> {
        arc(FourierWave::new(coefficients, hz))
    }

    pub fn set_hz(&mut self, hz: Hz) {
        self.hz = hz;
        for n in 0..self.sines.len() {
            self.sines[n].hz = hz * n as f64;
        }
    }
}

impl Signal for FourierWave {
    fn signal_add(&mut self, sample_rate: f64, add: Phase) -> Amp {
        self.amplitude
            * self
                .sines
                .iter_mut()
                .fold(0., |acc, x| acc + x.signal_add(sample_rate, add))
    }
}

pub fn square_wave(n: u32, hz: Hz) -> ArcMutex<FourierWave> {
    let mut coefficients: Vec<f32> = Vec::new();
    for i in 0..=n {
        if i % 2 == 1 {
            coefficients.push(1. / i as f32);
        } else {
            coefficients.push(0.);
        }
    }
    FourierWave::wrapped(coefficients.as_ref(), hz)
}

pub fn triangle_wave(n: u32, hz: Hz) -> ArcMutex<FourierWave> {
    let mut coefficients: Vec<Amp> = Vec::new();
    for i in 0..=n {
        if i % 2 == 1 {
            let sgn = if i % 4 == 1 { -1.0 } else { 1.0 };
            coefficients.push(sgn / (i * i) as f32);
        } else {
            coefficients.push(0.);
        }
    }
    FourierWave::wrapped(coefficients.as_ref(), hz)
}