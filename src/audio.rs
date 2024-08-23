use std::fs::File;

use ctru::linear::LinearAllocator;
use ctru::services::ndsp::{self, Channel, Error};

use ndsp::wave::{Status, Wave};
use ndsp::AudioFormat;

use crate::decode::Decoder;

pub enum ChannelID {
    FrontLeft,
    FrontRight,
}

/// Struct to hold stereo audio and info to pass to a [crate::play::Player]
pub struct Music {
    decoder: Decoder,

    // Data buffers
    wave_left: DoubleBuffer,
    wave_right: DoubleBuffer,
}

impl Music {
    pub fn new(src: File) -> Self {
        Self::with_capacity(src, 44100 * AudioFormat::PCM16Mono.size() * 2)
    }

    pub fn with_capacity(src: File, capacity: usize) -> Self {
        let format = AudioFormat::PCM16Mono;

        let wave_left = DoubleBuffer::new(format, capacity);
        let wave_right = DoubleBuffer::new(format, capacity);

        let decoder = Decoder::new(src);

        let mut music = Self {
            decoder,
            wave_left,
            wave_right,
        };

        // Decode into the first buffer, switch, decode into the second buffer and then switch back to the first.
        music.decode_within().unwrap();
        music.altern();
        music.decode_within().unwrap();
        music.altern();

        music
    }

    /// Copies the two buffers into the "current" [Wave] buffers.
    ///
    /// # Errors
    ///
    /// Returns an error if the [Music] is busy playing.
    fn write_stereo(&mut self, left_audio: &[u8], right_audio: &[u8]) -> Result<(), Error> {
        self.write_single_channel(ChannelID::FrontLeft, left_audio)?;
        self.write_single_channel(ChannelID::FrontRight, right_audio)?;

        Ok(())
    }

    /// Write audio data to a single channel of the [Song].
    ///
    /// This function will write to the "current" buffer of the two [DoubleBuffer] held.
    ///
    /// # Returns
    ///
    /// Returns an [`Option`] which holds leftover data (audio data that couldn't be copied within the [`Song`]).
    ///
    /// # Errors
    ///
    /// Returns an error if the [`Song`] is busy playing.
    pub fn write_single_channel(&mut self, channel_id: ChannelID, src: &[u8]) -> Result<(), Error> {
        // The source buffer must be the representation of a i16 buffer
        assert_eq!(src.len() % 2, 0);

        let wave = match channel_id {
            ChannelID::FrontLeft => self.wave_left.current_mut(),
            ChannelID::FrontRight => self.wave_right.current_mut(),
        };

        let dst = wave.get_buffer_mut()?;

        assert!(src.len() <= dst.len());

        let dst = &mut dst[..src.len()];

        dst.copy_from_slice(src);

        Ok(())
    }

    /// Alterns the internal [`DoubleBuffer`] members.
    fn altern(&mut self) {
        self.wave_left.altern();
        self.wave_right.altern();
    }

    /// Decodes available data inside its [DoubleBuffer] members.
    fn decode_within(&mut self) -> Result<(), crate::decode::DecodeError> {
        let (left, right) = self.decoder.decode_until(self.buffer_len())?;

        self.write_stereo(&left, &right).unwrap();

        Ok(())
    }

    fn queue(&mut self, left_channel: &mut Channel, right_channel: &mut Channel) {
        left_channel
            .queue_wave(self.wave_left.current_mut())
            .unwrap();
        right_channel
            .queue_wave(self.wave_right.current_mut())
            .unwrap();
    }

    /// Decodes data into itself (if possible) and queues itself into the specified channels.
    pub(crate) fn play(&mut self, left_channel: &mut Channel, right_channel: &mut Channel) {
        if self.wave_left.is_free() && self.wave_right.is_free() {
            self.queue(left_channel, right_channel);

            self.altern();

            self.queue(left_channel, right_channel);

            self.altern();
        }

        if self.wave_left.should_altern()
            && self.wave_right.should_altern()
            && self.decode_within().is_ok()
        {
            self.queue(left_channel, right_channel);

            self.altern();
        }
    }

    /// Returns the length of the internal buffer (in bytes).
    pub fn buffer_len(&self) -> usize {
        self.wave_left.current().get_buffer().len()
    }
}

/// Audio double-buffering
pub struct DoubleBuffer {
    altern: bool,
    wave1: Wave<Box<[u8], LinearAllocator>>,
    wave2: Wave<Box<[u8], LinearAllocator>>,
}

impl DoubleBuffer {
    /// Creates a new [DoubleBuffer] object, capable of switching between 2 wavebuffers.
    /// The first wavebuffer is selected by default.
    pub fn new(format: AudioFormat, len: usize) -> Self {
        let buffer1 = unsafe { Box::new_zeroed_slice_in(len, LinearAllocator).assume_init() };
        let buffer2 = buffer1.clone();

        let wave1 = Wave::new(buffer1, format, false);
        let wave2 = Wave::new(buffer2, format, false);

        Self {
            altern: false,
            wave1,
            wave2,
        }
    }

    /// Returns whether the current buffer has finished playing or not.
    pub fn should_altern(&self) -> bool {
        matches!(self.current().status(), Status::Done)
    }

    /// Returns whether the current buffer is free to use.
    pub fn is_free(&self) -> bool {
        matches!(self.current().status(), Status::Free)
    }

    /// Returns a reference to the current buffer, without alternating.
    pub fn current(&self) -> &Wave<Box<[u8], LinearAllocator>> {
        match self.altern {
            false => &self.wave1,
            true => &self.wave2,
        }
    }

    /// Returns a mutable reference to the current buffer, without alternating.
    pub fn current_mut(&mut self) -> &mut Wave<Box<[u8], LinearAllocator>> {
        match self.altern {
            false => &mut self.wave1,
            true => &mut self.wave2,
        }
    }

    /// Returns a reference to the wave buffer AFTER alternating.
    pub fn altern(&mut self) -> &mut Wave<Box<[u8], LinearAllocator>> {
        self.altern = !self.altern;

        self.current_mut()
    }
}

impl TryFrom<symphonia::core::audio::Channels> for ChannelID {
    type Error = ();

    fn try_from(value: symphonia::core::audio::Channels) -> Result<Self, Self::Error> {
        match value {
            symphonia::core::audio::Channels::FRONT_LEFT => Ok(ChannelID::FrontLeft),
            symphonia::core::audio::Channels::FRONT_RIGHT => Ok(ChannelID::FrontRight),
            _ => Err(()),
        }
    }
}
