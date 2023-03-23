//! Test application to try out live recorded audio in a realtime stream.
//! The quality of the audio has been decreased to avoid an enormous application size, but it's not at all the limit of what the 3DS can do.

#![feature(allocator_api)]
#![feature(new_uninit)]

use ctru::prelude::*;
use ctru::romfs::RomFS;

use std::fs::File;

mod decode;
use decode::Decoder;

mod play;
use play::{DoubleBuffer, Player};

fn main() {
    ctru::use_panic_handler();

    let apt = Apt::init().unwrap();
    let hid = Hid::init().unwrap();
    let gfx = Gfx::init().unwrap();
    let _romfs = RomFS::init().unwrap();
    let _console = Console::init(gfx.top_screen.borrow_mut());

    let mut music_player = Player::new();
    let mut channel = music_player.channel(0).unwrap();
    Player::initialize_channel(&mut channel);

    // Open the media source.
    let src = File::open("romfs:/BlueDanube.mp3").expect("failed to open media");

    let mut decoder = Decoder::new(src);

    // SAMPLE RATE * TIME(s) * BYTES * CHANNELS
    const LENGTH: usize = 44100 * 5 * 2 * 2; // We'll buffer about 5 seconds of audio at a time
    let mut wave_buffer = DoubleBuffer::new(LENGTH);

    decode_into_double_buffer(&mut wave_buffer, &mut decoder);

    channel.queue_wave(wave_buffer.current_mut()).unwrap();

    while apt.main_loop() {
        hid.scan_input();

        if hid.keys_down().contains(KeyPad::KEY_START) {
            break;
        }
    }
}

/// Decode the next packet from the Decoder and copy it to the double buffer.
fn decode_into_double_buffer(double_buffer: &mut DoubleBuffer, decoder: &mut Decoder) {
    let wave = double_buffer.current_mut();
    let buf = wave.get_buffer_mut().unwrap();

    let mut result = Vec::with_capacity(buf.len());

    while decoder.sample_count() < buf.len() {
        let samples = decoder.decode_next();
        let mut bytes = samples.as_bytes().to_vec();

        result.append(&mut bytes);

        print!("\rDecoded {} samples", decoder.sample_count());
    }

    buf.copy_from_slice(&result[..buf.len()]);
}
