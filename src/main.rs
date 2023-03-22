//! Test application to try out live recorded audio in a realtime stream.
//! The quality of the audio has been decreased for issues with the application's size, but it's not at all the limit of what the 3DS can do.

#![feature(allocator_api)]

use ctru::linear::LinearAllocator;
use ctru::prelude::*;
use ctru::romfs::RomFS;
use ctru::services::ndsp::wave::WaveInfo;
use ctru::services::ndsp::{OutputMode, Ndsp, AudioFormat, InterpolationType};

use std::fs::File;

mod decode;
use decode::Decoder;

fn main() {
    ctru::use_panic_handler();

    let apt = Apt::init().unwrap();
    let hid = Hid::init().unwrap();
    let gfx = Gfx::init().unwrap();
    let mut ndsp = Ndsp::init().unwrap();
    let _romfs = RomFS::init().unwrap();
    let _console = Console::init(gfx.top_screen.borrow_mut());

    ndsp.set_output_mode(OutputMode::Stereo);
    let channel = ndsp.channel(0).unwrap();
    channel.set_interpolation(InterpolationType::Linear);
    channel.set_sample_rate(44100.);
    channel.set_format(AudioFormat::PCM16Stereo);

    // Output at 100% on the first pair of left and right channels.

    let mut mix: [f32; 12] = [0f32; 12];
    mix[0] = 1.0;
    mix[1] = 1.0;
    channel.set_mix(&mix);

    // Open the media source.
    let src = File::open("romfs:/output.mp3").expect("failed to open media");

    let mut decoder = Decoder::new(src);

    const LENGTH: usize = 5000000;

    let mut buffer = Vec::with_capacity_in(LENGTH, LinearAllocator);

    loop {
        let samples = decoder.decode_next();

        // The check is done internally to avoid unwanted re-allocations of `buffer`
        if decoder.sample_count() >= LENGTH {
            break;
        }

        let bytes = samples.as_bytes();
        let mut vector = bytes.to_vec_in(LinearAllocator);

        buffer.append(&mut vector);

        print!("\rDecoded {} samples", decoder.sample_count());
    }

    let mut wave = WaveInfo::new(buffer.into(), AudioFormat::PCM16Stereo, false);

    channel.queue_wave(&mut wave).unwrap();

    while apt.main_loop() {
        hid.scan_input();

        if hid.keys_down().contains(KeyPad::KEY_START) {
            break;
        }
    }
}
