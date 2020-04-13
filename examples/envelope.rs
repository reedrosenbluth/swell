use core::cmp::Ordering;
use crossbeam::crossbeam_channel::{unbounded, Receiver, Sender};
use derive_more::Constructor;
use nannou::prelude::*;
use nannou_audio as audio;
use nannou_audio::Buffer;

use swell::*;

fn main() {
    // nannou::app(model).update(update).simple_window(view).run();
    nannou::app(model).update(update).run();
}

#[derive(Constructor)]
struct Model {
    stream: audio::Stream<Synth>,
    receiver: Receiver<f32>,
    amps: Vec<f32>,
    max_amp: f32,
}

#[derive(Constructor)]
struct Synth {
    voice: Box<PolyWave>,
    sender: Sender<f32>,
}

fn model(app: &App) -> Model {
    let (sender, receiver) = unbounded();

    // Create a window to receive key pressed events.
    // Reduces CPU significantly...unsure how this affects the audio generation
    // app.set_loop_mode(LoopMode::Wait);

    let _window = app
        .new_window()
        .key_pressed(key_pressed)
        .key_released(key_released)
        .view(view)
        .build()
        .unwrap();

    let audio_host = audio::Host::new();
    let voice = voices();

    let synth = Synth {
        voice,
        sender,
    };

    let stream = audio_host
        .new_output_stream(synth)
        .render(audio)
        .build()
        .unwrap();

    Model {
        stream,
        receiver,
        amps: vec![],
        max_amp: 0.,
    }
}

fn audio(synth: &mut Synth, buffer: &mut Buffer) {
    let sample_rate = buffer.sample_rate() as f64;
    for frame in buffer.frames_mut() {
        let mut amp = 0.;
        amp += synth.voice.sample();
        synth.voice.update_phase(sample_rate);
        for channel in frame {
            *channel = amp;
        }
        synth.sender.send(amp).unwrap();
    }
}

fn voices() -> Box<PolyWave> {
    let mut vs: Vec<BoxedWave> = Vec::new();
    let freqs = [
        131., 139., 147., 156., 165., 175., 185., 196., 208., 220., 233., 247., 262., 277., 294.,
    ];
    for f in freqs.iter() {
        let w = TriggeredWave {
            wave: SineWave::boxed(*f),
            attack: 0.05,
            decay: 0.5,
            sustain_level: 1.0,
            release: 3.0,
            clock: 0.0,
            triggered: false,
        };
        vs.push(Box::new(w));
    }
    PolyWave::boxed(vs, 1.)
}

fn key_to_index(key: Key) -> Option<usize> {
    match key {
        // ------ Freq ---- Midi -- Note -------- //
        Key::A => Some(0),
        Key::W => Some(1),
        Key::S => Some(2),
        Key::E => Some(3),
        Key::D => Some(4),
        Key::F => Some(5),
        Key::T => Some(6),
        Key::G => Some(7),
        Key::Y => Some(8),
        Key::H => Some(9),
        Key::U => Some(10),
        Key::J => Some(11),
        Key::K => Some(12),
        Key::O => Some(13),
        Key::L => Some(14),
        _ => None,
    }
}

fn key_pressed(_app: &App, model: &mut Model, key: Key) {
    model.max_amp = 0.;
    model
        .stream
        .send(
            move |synth| match (synth.voice.as_ref(), key_to_freq(key)) {
                (None, Some(idx)) => {
                    synth.voice.on() = create_voice(hz);
                }
                _ => {}
            },
        )
        .unwrap();
}

fn key_released(_app: &App, model: &mut Model, key: Key) {
    // model
    //     .stream
    //     .send(move |synth| match synth.current_key {
    //         Some(current_key) if current_key == key => synth.voice = None,
    //         _ => {}
    //     })
    //     .unwrap();
}

fn update(_app: &App, model: &mut Model, _update: Update) {
    let amps: Vec<f32> = model.receiver.try_iter().collect();
    let clone = amps.clone();

    // find max amplitude in waveform
    let max = amps.iter().max_by(|x, y| {
        if x > y {
            Ordering::Greater
        } else {
            Ordering::Less
        }
    });

    // store if it's greater than the previously stored max
    if max.is_some() && *max.unwrap() > model.max_amp {
        model.max_amp = *max.unwrap();
    }

    model.amps = clone;
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    let c = rgb(9. / 255., 9. / 255., 44. / 255.);
    draw.background().color(c);
    let mut shifted: Vec<f32> = vec![];
    let mut iter = model.amps.iter().peekable();

    let mut i = 0;
    while iter.len() > 0 {
        let amp = iter.next().unwrap_or(&0.);
        if amp.abs() < 0.01 && **iter.peek().unwrap_or(&amp) > *amp {
            shifted = model.amps[i..].to_vec();
            break;
        }
        i += 1;
    }

    let l = 600;
    let mut points: Vec<Point2> = vec![];
    for (i, amp) in shifted.iter().enumerate() {
        if i == l {
            break;
        }
        points.push(pt2(i as f32, amp * 120.));
    }

    // only draw if we got enough info back from the audio thread
    if points.len() == 600 {
        draw.path()
            .stroke()
            .weight(2.)
            .points(points)
            .color(CORNFLOWERBLUE)
            .x_y(-300., 0.);
    }

    draw.to_frame(app, &frame).unwrap();
}