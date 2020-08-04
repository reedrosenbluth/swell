use super::oscillators::{ConstOsc, Oscillator, SignalFn};
use super::signal::*;
use super::utils::RingBuffer;
use crate::{as_any_mut, std_signal};
use std::any::Any;
use std::ops::{Index, IndexMut};

/// The `Union` module holds a vector of oscillators and plays one based on the
/// active tag. The `level` field is used to set the volume of whichever signal
/// is playing.
#[derive(Clone)]
pub struct Union {
    tag: Tag,
    waves: Vec<Tag>,
    active: Tag,
    level: In,
    out: Real,
}

impl Union {
    pub fn new(id_gen: &mut IdGen, waves: Vec<Tag>) -> Self {
        let active = waves[0];
        Union {
            tag: id_gen.id(),
            waves,
            active,
            level: 1.into(),
            out: 0.0,
        }
    }

    pub fn waves(&mut self, arg: Vec<Tag>) -> &mut Self {
        self.waves = arg;
        self
    }

    pub fn active(&mut self, arg: Tag) -> &mut Self {
        self.active = arg;
        self
    }

    pub fn level<T: Into<In>>(&mut self, arg: T) -> &mut Self {
        self.level = arg.into();
        self
    }
}

impl Builder for Union {}

impl Signal for Union {
    std_signal!();
    fn signal(&mut self, rack: &Rack, _sample_rate: Real) -> Real {
        self.out = In::val(rack, self.level) * rack.output(self.active);
        self.out
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
/// `Product` multiplies the signals of a vector of synth modules.
#[derive(Clone)]
pub struct Product {
    tag: Tag,
    waves: Vec<Tag>,
    out: Real,
}

impl Product {
    pub fn new(id_gen: &mut IdGen, waves: Vec<Tag>) -> Self {
        Product {
            tag: id_gen.id(),
            waves,
            out: 0.0,
        }
    }

    pub fn waves(&mut self, arg: Vec<Tag>) -> &mut Self {
        self.waves = arg;
        self
    }
}

impl Builder for Product {}

impl Signal for Product {
    std_signal!();
    fn signal(&mut self, rack: &Rack, _sample_rate: Real) -> Real {
        self.out = self.waves.iter().fold(1.0, |acc, n| acc * rack.output(*n));
        self.out
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

/// "Voltage controlled amplifier" multiplies the volume of the `wave` by the
/// value of `level`.
#[derive(Copy, Clone)]
pub struct Vca {
    tag: Tag,
    wave: Tag,
    level: In,
    out: Real,
}

impl Vca {
    pub fn new(id_gen: &mut IdGen, wave: Tag) -> Self {
        Self {
            tag: id_gen.id(),
            wave,
            level: 1.into(),
            out: 0.0,
        }
    }

    pub fn wave(&mut self, arg: Tag) -> &mut Self {
        self.wave = arg;
        self
    }

    pub fn level<T: Into<In>>(&mut self, arg: T) -> &mut Self {
        self.level = arg.into();
        self
    }
}

impl Builder for Vca {}

impl Signal for Vca {
    std_signal!();
    fn signal(&mut self, rack: &Rack, _sample_rate: Real) -> Real {
        self.out = rack.output(self.wave) * In::val(rack, self.level);
        self.out
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

/// Mixer with individual attenuverters for each wave plus an overall attenuverter.
#[derive(Clone)]
pub struct Mixer {
    tag: Tag,
    waves: Vec<Tag>,
    levels: Vec<In>,
    level: In,
    out: Real,
}

impl Mixer {
    pub fn new(id_gen: &mut IdGen, waves: Vec<Tag>) -> Self {
        let levels = waves.iter().map(|_| 1.into()).collect();
        Mixer {
            tag: id_gen.id(),
            waves,
            levels,
            level: 1.into(),
            out: 0.0,
        }
    }

    pub fn waves(&mut self, arg: Vec<Tag>) -> &mut Self {
        self.waves = arg;
        self.levels.resize_with(self.waves.len(), || 0.5.into());
        self
    }

    pub fn levels<T: Into<In>>(&mut self, arg: Vec<T>) -> &mut Self {
        assert_eq!(
            arg.len(),
            self.waves.len(),
            "Levels must have same length as waves"
        );
        let v = arg.into_iter().map(|x| x.into());
        self.levels = v.collect();
        self
    }

    pub fn level<T: Into<In>>(&mut self, arg: T) -> &mut Self {
        self.level = arg.into();
        self
    }

    pub fn level_nth<T: Into<In>>(&mut self, n: usize, arg: T) -> &mut Self {
        self.levels[n] = arg.into();
        self
    }
}

impl Builder for Mixer {}

impl Signal for Mixer {
    std_signal!();
    fn signal(&mut self, rack: &Rack, _sample_rate: Real) -> Real {
        self.out = self.waves.iter().enumerate().fold(0.0, |acc, (i, n)| {
            acc + rack.output(*n) * In::val(rack, self.levels[i])
        }) * In::val(rack, self.level);
        self.out
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

/// A cross fade synth module, alpha = 0 means 100% wave 1.
#[derive(Copy, Clone)]
pub struct CrossFade {
    tag: Tag,
    wave1: In,
    wave2: In,
    alpha: In,
    out: Real,
}

impl CrossFade {
    pub fn new(id_gen: &mut IdGen, wave1: Tag, wave2: Tag) -> Self {
        CrossFade {
            tag: id_gen.id(),
            wave1: wave1.into(),
            wave2: wave2.into(),
            alpha: (0.5).into(),
            out: 0.0,
        }
    }

    pub fn wave1<T: Into<In>>(&mut self, arg: T) -> &mut Self {
        self.wave1 = arg.into();
        self
    }

    pub fn wave2<T: Into<In>>(&mut self, arg: T) -> &mut Self {
        self.wave2 = arg.into();
        self
    }

    pub fn alpha<T: Into<In>>(&mut self, arg: T) -> &mut Self {
        self.alpha = arg.into();
        self
    }
}

impl Builder for CrossFade {}

impl Signal for CrossFade {
    std_signal!();
    fn signal(&mut self, rack: &Rack, _sample_rate: Real) -> Real {
        let alpha = In::val(rack, self.alpha);
        self.out = alpha * In::val(rack, self.wave2) + (1.0 - alpha) * In::val(rack, self.wave1);
        self.out
    }
}

impl Index<&str> for CrossFade {
    type Output = In;

    fn index(&self, index: &str) -> &Self::Output {
        match index {
            "wave1" => &self.wave1,
            "wave2" => &self.wave2,
            "alpha" => &self.alpha,
            _ => panic!("CrossFade does not have a field named: {}", index),
        }
    }
}

impl IndexMut<&str> for CrossFade {
    fn index_mut(&mut self, index: &str) -> &mut Self::Output {
        match index {
            "wave1" => &mut self.wave1,
            "wave2" => &mut self.wave2,
            "alpha" => &mut self.alpha,
            _ => panic!("CrossFade does not have a field named: {}", index),
        }
    }
}

/// A `Modulator` is designed to be the input to the `hz` field of a carrier
/// wave. It takes control of the carriers frequency and modulates it's base
/// hz by adding mod_idx * mod_hz * output of modulator wave.
#[derive(Clone)]
pub struct Modulator {
    tag: Tag,
    wave: ArcMutex<Oscillator>,
    hz: In,
    ratio: In,
    index: In,
    hz_osc: ArcMutex<ConstOsc>,
    ratio_osc: ArcMutex<ConstOsc>,
    index_osc: ArcMutex<ConstOsc>,
    mod_hz: ArcMutex<Product>,
    amp_factor: ArcMutex<Product>,
    mod_amp: ArcMutex<Mixer>,
    carrier_hz: ArcMutex<Mixer>,
    rack: Rack,
    out: Real,
}

impl Modulator {
    pub fn new<H, R, I>(id_gen: &mut IdGen, signal_fn: SignalFn, hz: H, ratio: R, index: I) -> Self
    where
        H: Into<In> + Copy,
        R: Into<In> + Copy,
        I: Into<In> + Copy,
    {
        let mut rack = Rack::new();
        let mut id = IdGen::new();
        let hz_osc = ConstOsc::new(&mut id, hz.into()).rack(&mut rack);
        let ratio_osc = ConstOsc::new(&mut id, ratio.into()).rack(&mut rack);
        let index_osc = ConstOsc::new(&mut id, index.into()).rack(&mut rack);
        let mod_hz = Product::new(&mut id, vec![ratio_osc.tag(), hz_osc.tag()]).rack(&mut rack);
        let amp_factor = Product::new(
            &mut id,
            vec![index_osc.tag(), hz_osc.tag(), ratio_osc.tag()],
        )
        .rack(&mut rack);
        let mod_amp = Mixer::new(&mut id, vec![hz_osc.tag(), amp_factor.tag()]).rack(&mut rack);
        let wave = Oscillator::new(&mut id, signal_fn)
            .hz(mod_hz.tag())
            .amplitude(mod_amp.tag())
            .rack(&mut rack);
        let carrier_hz = Mixer::new(&mut id, vec![wave.tag(), hz_osc.tag()]).rack(&mut rack);
        Modulator {
            tag: id_gen.id(),
            wave,
            hz: hz.into(),
            /// modulator frequency / carrier frequency
            ratio: ratio.into(),
            index: index.into(),
            hz_osc,
            ratio_osc,
            index_osc,
            mod_hz,
            amp_factor,
            mod_amp,
            carrier_hz,
            rack,
            out: 0.0,
        }
    }
}

impl Builder for Modulator {}

impl Signal for Modulator {
    std_signal!();
    fn signal(&mut self, _rack: &Rack, sample_rate: Real) -> Real {
        self.out = self.rack.signal(sample_rate);
        self.out
    }
}

impl Index<&str> for Modulator {
    type Output = In;

    fn index(&self, index: &str) -> &Self::Output {
        match index {
            "hz" => &self.hz,
            "ratio" => &self.ratio,
            "index" => &self.index,
            _ => panic!("Modulator only does not have a field named:  {}", index),
        }
    }
}

impl IndexMut<&str> for Modulator {
    fn index_mut(&mut self, index: &str) -> &mut Self::Output {
        match index {
            "hz" => &mut self.hz,
            "ratio" => &mut self.ratio,
            "index" => &mut self.index,
            _ => panic!("Modulator only does not have a field named:  {}", index),
        }
    }
}

/// A variable length delay line.
#[derive(Clone)]
pub struct Delay {
    tag: Tag,
    wave: Tag,
    delay_time: In,
    ring_buffer: RingBuffer<Real>,
    out: Real,
}

impl Delay {
    pub fn new(id_gen: &mut IdGen, wave: Tag, delay_time: In) -> Self {
        let ring = RingBuffer::<Real>::new(0.0, 0);
        Self {
            tag: id_gen.id(),
            wave,
            delay_time,
            ring_buffer: ring,
            out: 0.0,
        }
    }

    pub fn wave(&mut self, arg: Tag) -> &mut Self {
        self.wave = arg;
        self
    }

    pub fn delay_time<T: Into<In>>(&mut self, arg: T) -> &mut Self {
        self.delay_time = arg.into();
        self
    }
}

impl Builder for Delay {}

impl Signal for Delay {
    std_signal!();
    fn signal(&mut self, rack: &Rack, sample_rate: Real) -> Real {
        let delay = In::val(rack, self.delay_time) * sample_rate;
        let rp = self.ring_buffer.read_pos;
        let wp = (delay + rp).ceil();
        self.ring_buffer.set_write_pos(wp as usize);
        self.ring_buffer.set_read_pos(wp - delay);
        if delay > self.ring_buffer.len() as Real - 3.0 {
            self.ring_buffer.resize(delay as usize + 3);
        }
        let val = rack.output(self.wave);
        self.ring_buffer.push(val);
        self.out = self.ring_buffer.get_cubic();
        self.out
    }
}