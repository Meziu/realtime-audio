use std::cell::RefCell;

use ctru::services::ndsp::{AudioFormat, AudioMix, InterpolationType, Ndsp, OutputMode};

use crate::audio::Music;

/// Audio playback handler
///
/// Based on [`Ndsp`], it can hold 1 music track at a time.
pub struct Player {
    ndsp: RefCell<Ndsp>,
    music: RefCell<Option<Music>>,
}

impl Player {
    pub fn new() -> Self {
        let mut ndsp = Ndsp::new().unwrap();
        ndsp.set_output_mode(OutputMode::Stereo);

        let mut player = Self {
            ndsp: RefCell::new(ndsp),
            music: RefCell::new(None),
        };

        player.setup_channels();

        player
    }

    /// Loads a music track into the player for playback.
    pub fn load_music(&mut self, music: Music) {
        *self.music.borrow_mut() = Some(music)
    }

    /// Helper to setup channels during initialization.
    fn setup_channels(&mut self) {
        // Setup the first 2 channels for music playback
        let ndsp = self.ndsp.borrow_mut();
        let mut channel0 = ndsp.channel(0).unwrap();

        channel0.set_interpolation(InterpolationType::Linear);
        channel0.set_sample_rate(44100.);
        channel0.set_format(AudioFormat::PCM16Mono);

        let mut mix = AudioMix::zeroed();
        mix.set_front(1., 0.);
        channel0.set_mix(&mix);

        let mut channel1 = ndsp.channel(1).unwrap();

        channel1.set_interpolation(InterpolationType::Linear);
        channel1.set_sample_rate(44100.);
        channel1.set_format(AudioFormat::PCM16Mono);

        let mut mix = AudioMix::zeroed();
        mix.set_front(0., 1.);
        channel1.set_mix(&mix);
    }

    /// Audio frame handler.
    ///
    /// # Notes
    ///
    /// This function is supposed to be run in a constant loop to update the audio without annoying gaps.
    /// That should be done in a separate thread to ensure performance indipendence.
    pub fn play(&mut self) {
        let ndsp = self.ndsp.borrow_mut();
        let mut channel0 = ndsp.channel(0).unwrap();
        let mut channel1 = ndsp.channel(1).unwrap();

        if let Some(music) = self.music.borrow_mut().as_mut() {
            music.play(&mut channel0, &mut channel1);
        }
    }
}
