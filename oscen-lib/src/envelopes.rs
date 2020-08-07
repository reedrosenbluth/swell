use super::signal::*;
use super::utils::ExpInterp;
use crate::{as_any_mut, gate, std_signal};
use std::{
    any::Any,
    ops::{Index, IndexMut},
};

#[derive(Copy, Clone)]
pub struct Adsr {
    tag: Tag,
    attack: In,
    decay: In,
    sustain: In,
    release: In,
    clock: Real,
    sustain_time: Real,
    triggered: bool,
    level: Real,
    a_param: Real,
    d_param: Real,
    r_param: Real,
    a_interp: ExpInterp,
    d_interp: ExpInterp,
    r_interp: ExpInterp,
    out: Real,
}

impl Adsr {
    pub fn new(id_gen: &mut IdGen, a_param: Real, d_param: Real, r_param: Real) -> Self {
        let a_interp = ExpInterp::new(0.0, 0.5, 1.0);
        let d_interp = ExpInterp::new(0.0, 0.5, 1.0);
        let r_interp = ExpInterp::new(0.0, 0.5, 1.0);
        Self {
            tag: id_gen.id(),
            attack: (0.01).into(),
            decay: 0.into(),
            sustain: 1.into(),
            release: (0.1).into(),
            clock: 0.0,
            sustain_time: 0.0,
            triggered: false,
            level: 0.0,
            a_param,
            d_param,
            r_param,
            a_interp,
            d_interp,
            r_interp,
            out: 0.0,
        }
    }

    pub fn linear(id_gen: &mut IdGen) -> Self {
        Self::new(id_gen, 0.5, 0.5, 0.5)
    }

    pub fn exp_20(id_gen: &mut IdGen) -> Self {
        Self::new(id_gen, 0.2, 0.2, 0.2)
    }

    pub fn attack<T: Into<In>>(&mut self, arg: T) -> &mut Self {
        self.attack = arg.into();
        self
    }

    pub fn decay<T: Into<In>>(&mut self, arg: T) -> &mut Self {
        self.decay = arg.into();
        self
    }

    pub fn sustain<T: Into<In>>(&mut self, arg: T) -> &mut Self {
        self.sustain = arg.into();
        self
    }

    pub fn release<T: Into<In>>(&mut self, arg: T) -> &mut Self {
        self.release = arg.into();
        self
    }

    pub fn calc_level(&mut self, rack: &Rack) -> Real {
        fn max01(a: f64) -> f64 {
            if a > 0.01 {
                a
            } else {
                0.01
            }
        }

        let a = max01(In::val(rack, self.attack));
        let d = max01(In::val(rack, self.decay));
        let s = In::val(rack, self.sustain);
        let r = max01(In::val(rack, self.release));

        if self.triggered {
            match self.clock {
                // Attack
                t if t < a => self.a_interp.interp(t / a),
                // Decay
                t if t < a + d => self.d_interp.interp((t - a) / d),
                // Sustain
                t => {
                    self.sustain_time = t - a - d;
                    s
                }
            }
        } else {
            match self.clock {
                // Attack
                t if t < a => self.a_interp.interp(t / a),
                // Decay
                t if t < a + d => self.d_interp.interp((t - a) / d),
                // Release
                t if t < a + d + r + self.sustain_time => {
                    self.r_interp.interp((t - a - d - self.sustain_time) / r)
                }
                // Off
                _ => 0.,
            }
        }
    }

    pub fn on(&mut self) {
        self.triggered = true;
        self.sustain_time = 0.0;
        self.clock = self.a_interp.interp_inv(self.level);
    }

    pub fn off(&mut self) {
        self.triggered = false;
    }
}

impl Builder for Adsr {}

gate!(Adsr);

impl Signal for Adsr {
    std_signal!();
    fn signal(&mut self, rack: &Rack, sample_rate: Real) -> Real {
        self.a_interp.update(0.0, 1.0 - self.a_param, 1.0);
        let s = In::val(rack, self.sustain);
        self.d_interp.update(1.0, s + self.d_param * (1.0 - s), s);
        self.r_interp.update(s, self.r_param * s, 0.0);
        self.out = self.calc_level(rack);
        self.clock += 1. / sample_rate;
        self.out
    }
}

impl Index<&str> for Adsr {
    type Output = In;

    fn index(&self, index: &str) -> &Self::Output {
        match index {
            "attack" => &self.attack,
            "decay" => &self.decay,
            "sustain" => &self.sustain,
            "release" => &self.release,
            _ => panic!("Adsr does not have a field named: {}", index),
        }
    }
}

impl IndexMut<&str> for Adsr {
    fn index_mut(&mut self, index: &str) -> &mut Self::Output {
        match index {
            "attack" => &mut self.attack,
            "decay" => &mut self.decay,
            "sustain" => &mut self.sustain,
            "release" => &mut self.release,
            _ => panic!("Adsr does not have a field named: {}", index),
        }
    }
}
