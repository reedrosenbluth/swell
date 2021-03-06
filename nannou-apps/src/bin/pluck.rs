use crossbeam::crossbeam_channel::{unbounded, Receiver, Sender};
use nannou::{prelude::*, ui::prelude::*};
use nannou_audio as audio;
use nannou_audio::Buffer;
use oscen::instruments::*;
use oscen::midi::*;
use oscen::operators::*;
use oscen::oscillators::*;
use oscen::rack::*;
use std::{sync::Arc, thread};

fn main() {
    nannou::app(model).update(update).run();
}

#[allow(dead_code)]
struct Model {
    ui: Ui,
    stream: audio::Stream<Synth>,
    receiver: Receiver<f32>,
    amps: Vec<f32>,
    max_amp: f32,
}

#[derive(Clone)]
struct Midi {
    midi_pitch: Arc<MidiPitch>,
}

struct Synth {
    midi: Midi,
    midi_receiver: Receiver<Vec<u8>>,
    rack: Rack,
    controls: Box<Controls>,
    state: Box<State>,
    outputs: Box<Outputs>,
    buffers: Box<Buffers>,
    karplus: Arc<WaveGuide>,
    sender: Sender<f32>,
}

fn build_synth(midi_receiver: Receiver<Vec<u8>>, sender: Sender<f32>) -> Synth {
    let (mut rack, mut controls, mut state, outputs, mut buffers) = tables();

    //  Midi
    let midi_pitch = MidiPitchBuilder::new().rack(&mut rack, &mut controls);
    // MidiControlBuilder::new(64).rack(&mut rack, &mut controls);
    let excite = OscBuilder::new(square_osc)
        .hz(110.0)
        .rack(&mut rack, &mut controls, &mut state);
    let hz_inv = InverseBuilder::new(midi_pitch.tag()).rack(&mut rack);

    let karplus = WaveGuideBuilder::new(excite.tag())
        // let karplus = WaveGuide::new(&mut id_gen, excite.tag())
        .hz_inv(hz_inv.tag())
        .decay(0.95)
        .rack(&mut rack, &mut controls, &mut buffers);
    karplus.set_adsr_attack(&mut controls, 0.005.into());
    karplus.set_adsr_release(&mut controls, 0.005.into());

    Synth {
        midi: Midi { midi_pitch },
        midi_receiver,
        rack,
        controls,
        state,
        outputs,
        buffers,
        karplus,
        sender,
    }
}

fn model(app: &App) -> Model {
    let (sender, receiver) = unbounded();
    let (midi_sender, midi_receiver) = unbounded();

    thread::spawn(|| match listen_midi(midi_sender) {
        Ok(_) => (),
        Err(err) => println!("Error: {}", err),
    });

    let _window = app.new_window().size(900, 520).view(view).build().unwrap();

    let ui = app.new_ui().build().unwrap();

    let audio_host = audio::Host::new();
    let synth = build_synth(midi_receiver, sender);
    let stream = audio_host
        .new_output_stream(synth)
        .render(audio)
        .build()
        .unwrap();

    Model {
        ui,
        stream,
        receiver,
        amps: vec![],
        max_amp: 0.,
    }
}

// A function that renders the given `Audio` to the given `Buffer`.
fn audio(synth: &mut Synth, buffer: &mut Buffer) {
    let midi_messages: Vec<Vec<u8>> = synth.midi_receiver.try_iter().collect();
    for message in midi_messages {
        if message.len() == 3 {
            let step = message[1] as f32;
            if message[0] == 144 {
                synth
                    .midi
                    .midi_pitch
                    .set_step(&mut synth.controls, step.into());
                synth.karplus.on(&mut synth.controls, &mut synth.state);
            } else if message[0] == 128 {
                synth.karplus.off(&mut synth.controls);
            }
        }
    }

    let sample_rate = buffer.sample_rate() as f32;
    for frame in buffer.frames_mut() {
        let amp = synth.rack.mono(
            &synth.controls,
            &mut synth.state,
            &mut synth.outputs,
            &mut synth.buffers,
            sample_rate,
        );

        for channel in frame {
            *channel = amp;
        }
        synth.sender.send(amp).unwrap();
    }
}

fn update(_app: &App, model: &mut Model, _update: Update) {
    let amps: Vec<f32> = model.receiver.try_iter().collect();
    model.amps = amps;
}

fn view(app: &App, model: &Model, frame: Frame) {
    use nannou_apps::scope;
    scope(app, &model.amps, frame);
}
