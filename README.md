# realtime-audio
Small homebrew program for the Nintendo 3DS written in Rust to showcase realtime MP3 audio decoding and playback.

# About

This is a very rough example. This was mainly developed to showcase the functionality in [`ctru-rs`](https://github.com/rust3ds/ctru-rs) and
check if the APIs were good enough to write some real audio code. In a real application, the audio should be handled with as little overhead as possible
in a separate thread.

# Known issues
The example doesn't run in `debug` mode, so you'll need to build it via `cargo 3ds [run/build] --release`
