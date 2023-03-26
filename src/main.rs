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
    const LENGTH: usize = 44100 * 1 * 2 * 2; // We'll buffer about 1 second of audio at a time
    let mut wave_buffer = DoubleBuffer::new(LENGTH);

    let mut leftovers = Vec::new();

    // For the sake of the example we will assume we can get the double buffer filled.
    decoder
        .decode_into_wave(wave_buffer.current_mut(), &mut leftovers)
        .expect("Audio file too short");
    decoder
        .decode_into_wave(wave_buffer.altern(), &mut leftovers)
        .expect("Audio file too short");

    // Queue the first two packets
    channel.queue_wave(wave_buffer.altern()).unwrap();
    channel.queue_wave(wave_buffer.altern()).unwrap();
    wave_buffer.altern();

    while apt.main_loop() {
        hid.scan_input();

        if hid.keys_down().contains(KeyPad::KEY_START) {
            break;
        }

        if wave_buffer.should_altern() {
            match decoder.decode_into_wave(wave_buffer.current_mut(), &mut leftovers) {
                Ok(_) => {
                    channel.queue_wave(wave_buffer.current_mut()).unwrap();
                    wave_buffer.altern();
                }
                Err(_) => println!("\x1b[10;1HAudio stream over"),
            }
        }
    }
}
