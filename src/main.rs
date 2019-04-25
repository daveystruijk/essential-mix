#![allow(unused_variables)]
extern crate dsp;
extern crate portaudio;
extern crate simplemad;

use dsp::{Graph, Frame, Node, FromSample, Sample};
use dsp::sample::ToFrameSliceMut;
use portaudio as pa;
use simplemad::Decoder;

use std::fs::File;
use std::path::Path;

const CHANNELS: usize = 2;
const FRAMES: u32 = 1152;
const SAMPLE_HZ: f64 = 44_100.0;

fn main() {
    run().unwrap()
}

fn run() -> Result<(), pa::Error> {
    let path = Path::new("/Users/daveystruijk/Drive/Music/9 Techno/Pan-Pot - Sleepless (Stephan Bodzin Remix).mp3");
    let file = File::open(&path).unwrap();
    let decoder = Decoder::decode(file).unwrap();

    // TODO: Move loading & conversion of mp3 to seperate file
    let buffers: Vec<Vec<[f32; CHANNELS]>> = decoder.filter_map(|r| match r {
            Ok(buffer) => {
                let it = buffer.samples[0].iter().zip(buffer.samples[1].iter());
                match it.len() {
                    1152 => {
                        let frames = it.map(|(x, y)| {
                            let bar = [x.to_f32(), y.to_f32()];
                            let sample = bar.map(Sample::to_sample);
                            sample
                        }).collect();
                        Some(frames)
                    }
                    _ => None,
                }
            }
            Err(_) => None,
        })
    .collect();

    println!("a buffer");
    println!("{:?}", buffers[0].len());

    // let mut samples: Vec<Vec<simplemad::MadFixed32>> = decoder.map(|result|
    //     match result {
    //         Err(e) => {
    //         },
    //         Ok(frame) => {
    //             return frame.samples[0];
    //         },
    //     }
    // );

    let mut graph = Graph::new();

    let track = graph.add_node(DspNode::Track(0));

    // Output our synth to a marvellous volume node.
    let (_, volume) = graph.add_output(track, DspNode::Volume(1.0));

    // Set the synth as the master node for the graph.
    graph.set_master(Some(volume));

    // We'll use this to count down from three seconds and then break from the loop.
    let mut timer: f64 = 0.0;

    // This will be used to determine the delta time between calls to the callback.
    let mut prev_time = None;

    let mut i = 0;

    // The callback we'll use to pass to the Stream. It will request audio from our graph.
    let callback = move |pa::OutputStreamCallbackArgs { buffer, time, .. }| {

        let buffer: &mut [[f32; CHANNELS]] = buffer.to_frame_slice_mut().unwrap();

        println!("the buffer");
        println!("{:?}", buffer.len());

        // TODO: Make sure this data comes from an actual Track within the dsp graph
        buffer.copy_from_slice(&buffers[i]);

        i += 1;

        // graph.audio_requested(buffer, SAMPLE_HZ);

        if let &mut DspNode::Track(ref mut pos) = &mut graph[track] {
            *pos += 1;
        }

        let last_time = prev_time.unwrap_or(time.current);
        let dt = time.current - last_time;
        timer += dt;
        prev_time = Some(time.current);
        if timer <= 5000.0 {
            pa::Continue
        } else {
            pa::Complete
        }
    };

    // Construct PortAudio and the stream.
    let pa = pa::PortAudio::new()?;
    let settings = pa.default_output_stream_settings::<f32>(
        CHANNELS as i32,
        SAMPLE_HZ,
        FRAMES,
    )?;
    let mut stream = pa.open_non_blocking_stream(settings, callback)?;
    stream.start()?;

    // Wait for our stream to finish.
    while let Ok(true) = stream.is_active() {
        ::std::thread::sleep(::std::time::Duration::from_millis(16));
    }

    Ok(())
}


/// Our Node to be used within the Graph.
enum DspNode {
    Track(i32),
    Volume(f32),
}

const SYNTH_HZ: f64 = 440.0;
/// Implement the `Node` trait for our DspNode.
impl Node<[f32; CHANNELS]> for DspNode {
    fn audio_requested(&mut self, buffer: &mut [[f32; CHANNELS]], sample_hz: f64) {
        match *self {
            DspNode::Track(ref mut pos) => {
                dsp::slice::map_in_place(buffer, |_| {
                    let val = mp3_at(*pos);
                    *pos += 1;
                    Frame::from_fn(|_| val)
                })
            }
            DspNode::Volume(vol) => {
                dsp::slice::map_in_place(buffer, |frame| {
                    frame
                });
            }
        }
    }
}

/// Return a sine wave for the given phase.
fn mp3_at<S: Sample>(pos: i32) -> S
where
    S: Sample + FromSample<f32>,
{
    // TODO: Load from mp3 buffer
    (((pos as f32) * 2.0).sin() as f32).to_sample::<S>()
    // ((pos * PI * 2.0).sin() as f32).to_sample::<S>()

}
