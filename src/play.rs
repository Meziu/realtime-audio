use ctru::linear::LinearAllocator;
use ctru::services::ndsp::wave::WaveStatus;
use ctru::services::ndsp::{
    wave::WaveInfo, AudioFormat, Channel, InterpolationType, Ndsp, NdspError, OutputMode,
};

pub struct Player {
    ndsp: Ndsp,
}

impl Player {
    pub fn new() -> Self {
        let mut ndsp = Ndsp::init().unwrap();
        ndsp.set_output_mode(OutputMode::Stereo);

        Self { ndsp }
    }

    pub fn initialize_channel(channel: &mut Channel) {
        channel.set_interpolation(InterpolationType::Linear);
        channel.set_sample_rate(44100.);
        channel.set_format(AudioFormat::PCM16Stereo);

        // Output at 100% on the first pair of left and right channels.

        let mut mix: [f32; 12] = [0f32; 12];
        mix[0] = 1.0;
        mix[1] = 1.0;
        channel.set_mix(&mix);
    }

    pub fn channel(&mut self, id: usize) -> Result<Channel, NdspError> {
        self.ndsp.channel(id as u8)
    }
}

/// Audio double-buffering
pub struct DoubleBuffer {
    altern: bool,
    wave1: WaveInfo,
    wave2: WaveInfo,
}

impl DoubleBuffer {
    /// Creates a new [DoubleBuffer] object, capable of switching between 2 wavebuffers.
    /// The first wavebuffer is selected by default.
    pub fn new(len: usize) -> Self {
        let buffer1 = unsafe { Box::new_zeroed_slice_in(len, LinearAllocator).assume_init() };
        let buffer2 = buffer1.clone();

        let wave1 = WaveInfo::new(buffer1, AudioFormat::PCM16Stereo, false);
        let wave2 = WaveInfo::new(buffer2, AudioFormat::PCM16Stereo, false);

        Self {
            altern: false,
            wave1,
            wave2,
        }
    }

    /// Returns whether the current buffer has finished playing or not.
    pub fn should_altern(&self) -> bool {
        matches!(self.current().get_status(), WaveStatus::Done)
    }

    /// Returns a reference to the current buffer, without alternating.
    pub fn current(&self) -> &WaveInfo {
        match self.altern {
            false => &self.wave1,
            true => &self.wave2,
        }
    }

    /// Returns a mutable reference to the current buffer, without alternating.
    pub fn current_mut(&mut self) -> &mut WaveInfo {
        match self.altern {
            false => &mut self.wave1,
            true => &mut self.wave2,
        }
    }

    /// Returns a reference to the wave buffer AFTER alternating.
    pub fn altern(&mut self) -> &mut WaveInfo {
        self.altern = !self.altern;

        self.current_mut()
    }
}
