use super::signal::*;
use super::utils::RingBuffer;
use crate::{as_any_mut, std_signal};
use std::any::Any;
use std::ops::{Index, IndexMut};

#[derive(Clone)]
pub struct Union {
    tag: Tag,
    pub waves: Vec<Tag>,
    pub active: Tag,
    pub level: In,
}

impl Union {
    pub fn new(waves: Vec<Tag>) -> Self {
        let active = waves[0];
        Union {
            tag: mk_tag(),
            waves,
            active,
            level: 1.into(),
        }
    }

    pub fn level(&mut self, arg: In) -> &mut Self {
        self.level = arg;
        self
    }
}

impl Builder for Union {}

impl Signal for Union {
    std_signal!();
    fn signal(&mut self, rack: &Rack, _sample_rate: Real) -> Real {
        In::val(rack, self.level) * rack.output(self.active)
    }
}

impl Index<usize> for Union {
    type Output = Tag;

    fn index(&self, index: usize) -> &Self::Output {
        &self.waves[index]
    }
}

impl IndexMut<usize> for Union {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.waves[index]
    }
}
#[derive(Clone)]
pub struct Product {
    pub tag: Tag,
    pub waves: Vec<Tag>,
}

impl Product {
    pub fn new(waves: Vec<Tag>) -> Self {
        Product {
            tag: mk_tag(),
            waves,
        }
    }
}

impl Builder for Product {}

impl Signal for Product {
    std_signal!();
    fn signal(&mut self, rack: &Rack, _sample_rate: Real) -> Real {
        self.waves.iter().fold(1.0, |acc, n| acc * rack.output(*n))
    }
}

impl Index<usize> for Product {
    type Output = Tag;

    fn index(&self, index: usize) -> &Self::Output {
        &self.waves[index]
    }
}

impl IndexMut<usize> for Product {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.waves[index]
    }
}

#[derive(Copy, Clone)]
pub struct Vca {
    pub tag: Tag,
    pub wave: Tag,
    pub level: In,
}

impl Vca {
    pub fn new(wave: Tag) -> Self {
        Self {
            tag: mk_tag(),
            wave,
            level: 1.into(),
        }
    }

    pub fn level(&mut self, arg: In) -> &mut Self {
        self.level = arg;
        self
    }
}

impl Builder for Vca {}

impl Signal for Vca {
    std_signal!();
    fn signal(&mut self, rack: &Rack, _sample_rate: Real) -> Real {
        rack.output(self.wave) * In::val(rack, self.level)
    }
}

impl Index<&str> for Vca {
    type Output = In;

    fn index(&self, index: &str) -> &Self::Output {
        match index {
            "level" => &self.level,
            _ => panic!("Vca does not have a field named: {}", index),
        }
    }
}

impl IndexMut<&str> for Vca {
    fn index_mut(&mut self, index: &str) -> &mut Self::Output {
        match index {
            "level" => &mut self.level,
            _ => panic!("Vca does not have a field named: {}", index),
        }
    }
}

#[derive(Clone)]
pub struct Mixer {
    pub tag: Tag,
    pub waves: Vec<Tag>,
    pub levels: Vec<In>,
    pub level: In,
}

impl Mixer {
    pub fn new(waves: Vec<Tag>) -> Self {
        let levels = waves.iter().map(|_| 1.into()).collect();
        Mixer {
            tag: mk_tag(),
            waves,
            levels,
            level: 1.into(),
        }
    }

    pub fn levels(&mut self, arg: Vec<In>) -> &mut Self {
        self.levels = arg;
        self
    }

    pub fn level(&mut self, arg: In) -> &mut Self {
        self.level = arg;
        self
    }
}

impl Builder for Mixer {}

impl Signal for Mixer {
    std_signal!();
    fn signal(&mut self, rack: &Rack, _sample_rate: Real) -> Real {
        self.waves.iter().enumerate().fold(0.0, |acc, (i, n)| {
            acc + rack.output(*n) * In::val(rack, self.levels[i])
        }) * In::val(rack, self.level)
    }
}

impl Index<usize> for Mixer {
    type Output = Tag;

    fn index(&self, index: usize) -> &Self::Output {
        &self.waves[index]
    }
}

impl IndexMut<usize> for Mixer {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.waves[index]
    }
}
#[derive(Copy, Clone)]
pub struct Lerp {
    pub tag: Tag,
    pub wave1: In,
    pub wave2: In,
    pub alpha: In,
}

impl Lerp {
    pub fn new(wave1: Tag, wave2: Tag) -> Self {
        Lerp {
            tag: mk_tag(),
            wave1: wave1.into(),
            wave2: wave2.into(),
            alpha: (0.5).into(),
        }
    }

    pub fn alpha(&mut self, arg: In) -> &mut Self {
        self.alpha = arg;
        self
    }
}

impl Builder for Lerp {}

impl Signal for Lerp {
    std_signal!();
    fn signal(&mut self, rack: &Rack, _sample_rate: Real) -> Real {
        let alpha = In::val(rack, self.alpha);
        alpha * In::val(rack, self.wave2) + (1.0 - alpha) * In::val(rack, self.wave1)
    }
}

impl Index<&str> for Lerp {
    type Output = In;

    fn index(&self, index: &str) -> &Self::Output {
        match index {
            "wave1" => &self.wave1,
            "wave2" => &self.wave2,
            "alpha" => &self.alpha,
            _ => panic!("Lerp does not have a field named: {}", index),
        }
    }
}

impl IndexMut<&str> for Lerp {
    fn index_mut(&mut self, index: &str) -> &mut Self::Output {
        match index {
            "wave1" => &mut self.wave1,
            "wave2" => &mut self.wave2,
            "alpha" => &mut self.alpha,
            _ => panic!("Lerp does not have a field named: {}", index),
        }
    }
}

pub fn set_alpha(rack: &Rack, k: In, a: Real) {
    match k {
        In::Cv(n) => {
            if let Some(v) = rack.nodes[&n]
                .module
                .lock()
                .unwrap()
                .as_any_mut()
                .downcast_mut::<Lerp>()
            {
                v.alpha = a.into()
            }
        }
        In::Fix(_) => panic!("Lerp wave can only be a In::Var"),
    }
}

/// A `Modulator` is designed to be the input to the `hz` field of a carrier
/// wave. It takes control of the carriers frequency and modulates it's base
/// hz by adding mod_idx * mod_hz * output of modulator wave.
#[derive(Copy, Clone)]
pub struct Modulator {
    pub tag: Tag,
    pub wave: In,
    pub base_hz: In,
    pub mod_hz: In,
    pub mod_idx: In,
}

impl Modulator {
    pub fn new(wave: Tag) -> Self {
        Modulator {
            tag: mk_tag(),
            wave: wave.into(),
            base_hz: 0.into(),
            mod_hz: 0.into(),
            mod_idx: 0.into(),
        }
    }

    pub fn base_hz(&mut self, arg: In) -> &mut Self {
        self.base_hz = arg;
        self
    }

    pub fn mod_hz(&mut self, arg: In) -> &mut Self {
        self.mod_hz = arg;
        self
    }

    pub fn mod_idx(&mut self, arg: In) -> &mut Self {
        self.mod_idx = arg;
        self
    }
}

impl Builder for Modulator {}

impl Signal for Modulator {
    std_signal!();
    fn signal(&mut self, rack: &Rack, _sample_rate: Real) -> Real {
        let mod_hz = In::val(rack, self.mod_hz);
        let mod_idx = In::val(rack, self.mod_idx);
        let base_hz = In::val(rack, self.base_hz);
        base_hz + mod_idx * mod_hz * In::val(rack, self.wave)
    }
}

impl Index<&str> for Modulator {
    type Output = In;

    fn index(&self, index: &str) -> &Self::Output {
        match index {
            "wave" => &self.wave,
            "base_hz" => &self.base_hz,
            "mod_hz" => &self.mod_hz,
            "mod_idx" => &self.mod_idx,
            _ => panic!("Modulator only does not have a field named:  {}", index),
        }
    }
}

impl IndexMut<&str> for Modulator {
    fn index_mut(&mut self, index: &str) -> &mut Self::Output {
        match index {
            "wave" => &mut self.wave,
            "base_hz" => &mut self.base_hz,
            "mod_hz" => &mut self.mod_hz,
            "mod_idx" => &mut self.mod_idx,
            _ => panic!("Modulator only does not have a field named:  {}", index),
        }
    }
}

#[derive(Clone)]
pub struct Delay {
    pub tag: Tag,
    pub wave: Tag,
    pub delay_time: In,
    ring_buffer: RingBuffer<Real>,
}

impl Delay {
    pub fn new(wave:Tag, delay_time: In) -> Self {
        let ring = RingBuffer::<Real>::new(0.0, 0);
        Self {
            tag: mk_tag(),
            wave,
            delay_time,
            ring_buffer: ring,
        }
    }

    pub fn delay_time(&mut self, arg: In) -> &mut Self {
        self.delay_time = arg;
        self
    }
}

impl Builder for Delay {}

impl Signal for Delay {
    std_signal!();
    fn signal(&mut self, rack: &Rack, sample_rate: Real) -> Real {
        let delay = In::val(rack, self.delay_time) * sample_rate;
        if delay > self.ring_buffer.len() as Real - 3.0 {
            self.ring_buffer.resize(delay as usize + 3);
            let wp = delay.ceil();
            self.ring_buffer.write_pos = wp as usize;
            self.ring_buffer.read_pos = delay - wp;
        }
        let val = rack.output(self.wave);
        self.ring_buffer.push(val);
        self.ring_buffer.get_linear()
    }
}