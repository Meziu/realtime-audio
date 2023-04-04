//! Test application to try out live recorded audio in a realtime stream.
//! The quality of the audio has been decreased to avoid an enormous application size, but it's not at all the limit of what the 3DS can do.

#![feature(allocator_api)]
#![feature(new_uninit)]

use ctru::prelude::*;
use ctru::services::romfs::RomFS;

use std::fs::File;

mod audio;
use audio::Music;

mod play;
use play::Player;

mod decode;

fn main() {
    ctru::use_panic_handler();

    let apt = Apt::init().unwrap();
    let hid = Hid::init().unwrap();
    let gfx = Gfx::init().unwrap();
    let _romfs = RomFS::init().unwrap();
    let _console = Console::init(gfx.top_screen.borrow_mut());

    let mut player = Player::new();

    // Open the media source.
    let src = File::open("romfs:/TakeOver.mp3").expect("failed to open media");
    let music = Music::new(src);

    player.load_music(music);

    while apt.main_loop() {
        hid.scan_input();

        if hid.keys_down().contains(KeyPad::START) {
            break;
        }

        player.play();
    }
}
