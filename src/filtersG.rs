use super::graph::*;
use std::f64::consts::PI;
use std::any::Any;

pub struct BiquadFilter
{
    pub wave: Tag,
    pub b1: Real,
    pub b2: Real,
    pub a0: Real,
    pub a1: Real,
    pub a2: Real,
    x1: Real,
    x2: Real,
    y1: Real,
    y2: Real,
    pub off: bool,
}

// See "Audio Processes, Musical Analysis, Modification, Synthesis, and Control"
// by David Creasy, 2017. pages 164-183.
pub fn lpf(sample_rate: Real, fc: Real, q: Real) -> (Real, Real, Real, Real, Real) {
    let phi = TAU * fc / sample_rate;
    let b2 = (2.0 * q - phi.sin()) / (2.0 * q + phi.sin());
    let b1 = -(1.0 + b2) * phi.cos();
    let a0 = 0.25 * (1.0 + b1 + b2);
    let a1 = 2.0 * a0;
    let a2 = a0;
    (b1, b2, a0, a1, a2)
}

pub fn hpf(sample_rate: Real, fc: Real, q: Real) -> (Real, Real, Real, Real, Real) {
    let phi = TAU * fc / sample_rate;
    let b2 = (2.0 * q - phi.sin()) / (2.0 * q + phi.sin());
    let b1 = -(1.0 + b2) * phi.cos();
    let a0 = 0.25 * (1.0 - b1 + b2);
    let a1 = -2.0 * a0;
    let a2 = a0;
    (b1, b2, a0, a1, a2)
}

pub fn lphpf(sample_rate: Real, fc: Real, q: Real, t: Real) -> (Real, Real, Real, Real, Real) {
    let (b1, b2, a0l, a1l, a2l) = lpf(sample_rate, fc, q);
    let (_, _, a0h, a1h, a2h) = hpf(sample_rate, fc, q);
    (
        b1,
        b2,
        t * a0l + (1. - t) * a0h,
        t * a1l + (1. - t) * a1h,
        t * a2l + (1. - t) * a2h,
    )
}

pub fn bpf(sample_rate: Real, fc: Real, q: Real) -> (Real, Real, Real, Real, Real) {
    let phi = TAU * fc / sample_rate;
    let b2 = (PI / 4.0 - phi / (2.0 * q)).tan();
    let b1 = -(1.0 + b2) * phi.cos();
    let a0 = 0.5 * (1.0 - b2);
    let a1 = 0.0;
    let a2 = -a0;
    (b1, b2, a0, a1, a2)
}

pub fn notch(sample_rate: Real, fc: Real, q: Real) -> (Real, Real, Real, Real, Real) {
    let phi = TAU * fc / sample_rate;
    let b2 = (PI / 4.0 - phi / (2.0 * q)).tan();
    let b1 = -(1.0 + b2) * phi.cos();
    let a0 = 0.5 * (1.0 + b2);
    let a1 = b1;
    let a2 = a0;
    (b1, b2, a0, a1, a2)
}

impl BiquadFilter
{
    pub fn new(wave: Tag, b1: Real, b2: Real, a0: Real, a1: Real, a2: Real) -> Self {
        Self {
            wave,
            b1,
            b2,
            a0,
            a1,
            a2,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            off: false,
        }
    }

    pub fn wrapped(
        wave: Tag,
        b1: Real,
        b2: Real,
        a0: Real,
        a1: Real,
        a2: Real,
    ) -> ArcMutex<Self> {
        arc(Self::new(wave, b1, b2, a0, a1, a2))
    }

    pub fn lpf(wave: Tag, sample_rate: Real, fc: Real, q: Real) -> Self {
        let (b1, b2, a0, a1, a2) = lpf(sample_rate, fc, q);
        Self::new(wave, b1, b2, a0, a1, a2)
    }

    pub fn hpf(wave: Tag, sample_rate: Real, fc: Real, q: Real) -> Self {
        let (b1, b2, a0, a1, a2) = hpf(sample_rate, fc, q);
        Self::new(wave, b1, b2, a0, a1, a2)
    }

    pub fn lphpf(wave: Tag, sample_rate: Real, fc: Real, q: Real, t: Real) -> Self {
        let (b1, b2, a0, a1, a2) = lphpf(sample_rate, fc, q, t);
        Self::new(wave, b1, b2, a0, a1, a2)
    }

    pub fn bpf(wave: Tag, sample_rate: Real, fc: Real, q: Real) -> Self {
        let (b1, b2, a0, a1, a2) = bpf(sample_rate, fc, q);
        Self::new(wave, b1, b2, a0, a1, a2)
    }

    pub fn notch(wave: Tag, sample_rate: Real, fc: Real, q: Real) -> Self {
        let (b1, b2, a0, a1, a2) = notch(sample_rate, fc, q);
        Self::new(wave, b1, b2, a0, a1, a2)
    }
}

impl Signal for BiquadFilter
{
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn signal(&mut self, graph: &Graph, _sample_rate: Real) -> Real {
        let x0 = graph.output(self.wave);
        if self.off {
            return x0;
        };
        let a0 = self.a0 as Real;
        let a1 = self.a1 as Real;
        let a2 = self.a2 as Real;
        let b1 = self.b1 as Real;
        let b2 = self.b2 as Real;
        let amp = a0 * x0 + a1 * self.x1 + a2 * self.x2 - b1 * self.y1 - b2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x0;
        self.y2 = self.y1;
        self.y1 = amp;
        amp
    }
}

pub struct Comb
{
    pub wave: Tag,
    buffer: Vec<Real>,
    index: usize,
    pub feedback: Real,
    pub filter_state: Real,
    pub dampening: Real,
    pub dampening_inverse: Real,
}

impl Comb
{
    pub fn new(wave: Tag, length: usize) -> Self {
        Self {
            wave,
            buffer: vec![0.0; length],
            index: 0,
            feedback: 0.5,
            filter_state: 0.0,
            dampening: 0.5,
            dampening_inverse: 0.5,
        }
    }

    pub fn wrapped(wave:Tag, length: usize) -> ArcMutex<Self> {
        arc(Self::new(wave, length))
    }
}

impl Signal for Comb
{
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn signal(&mut self, graph: &Graph, _sample_rate: Real) -> Real {
        let input = graph.output(self.wave);
        let output = self.buffer[self.index] as Real;
        self.filter_state =
            output * self.dampening_inverse + self.filter_state * self.dampening;
        self.buffer[self.index] = input + (self.filter_state * self.feedback) as Real;
        self.index += 1;
        if self.index == self.buffer.len() {
            self.index = 0
        }
        output as Real
    }
}


pub struct AllPass
{
    pub wave: Tag,
    buffer: Vec<Real>,
    index: usize,
}

impl AllPass
{
    pub fn new(wave: Tag, length: usize) -> Self {
        Self {
            wave,
            buffer: vec![0.0; length],
            index: 0,
        }
    }

    pub fn wrapped(wave:Tag, length: usize) -> ArcMutex<Self> {
        arc(Self::new(wave, length))
    }
}

impl Signal for AllPass
{
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn signal(&mut self, graph: &Graph, _sample_rate: Real) -> Real {
        let input = graph.output(self.wave);
        let delayed = self.buffer[self.index];
        let output = delayed - input;
        self.buffer[self.index] = input + (0.5 * delayed) as Real;
        self.index += 1;
        if self.index == self.buffer.len() {
            self.index = 0
        }
        output as Real
    }
}