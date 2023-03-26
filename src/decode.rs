use std::fs::File;

use ctru::services::ndsp::wave::WaveInfo;

use symphonia::core::audio::RawSampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct Decoder {
    track_id: u32,
    raw_decoder: Box<dyn symphonia::core::codecs::Decoder>,
    format: Box<dyn FormatReader>,
    sample_count: usize,
}

impl Decoder {
    pub fn new(src: File) -> Self {
        // Create the media source stream.
        let mss = MediaSourceStream::new(Box::new(src), Default::default());

        // Create a probe hint using the file's extension. [Optional]
        let mut hint = Hint::new();
        hint.with_extension("mp3");

        // Use the default options for metadata and format readers.
        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        // Probe the media source.
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &fmt_opts, &meta_opts)
            .expect("unsupported format");

        // Get the instantiated format reader.
        let format = probed.format;

        // Find the first audio track with a known (decodeable) codec.
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .expect("no supported audio tracks");

        // Use the default options for the decoder.
        let dec_opts: DecoderOptions = Default::default();

        // Create a decoder for the track.
        let raw_decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &dec_opts)
            .expect("unsupported codec");

        // Store the track identifier, we'll use it to filter packets.
        let track_id = track.id;

        let sample_count = 0;

        Self {
            track_id,
            raw_decoder,
            format,
            sample_count,
        }
    }

    /// Return the next decoded packet
    pub fn decode_next(&mut self) -> Option<RawSampleBuffer<i16>> {
        // Get the next packet from the format reader.
        let packet = match self.format.next_packet() {
            Ok(p) => p,
            // In theory we should handle the error depending on the type, but for the sake of this example we'll just treat every error as an "end of stream"
            Err(_) => return None,
        };

        // If the packet does not belong to the selected track, skip it.
        if packet.track_id() != self.track_id {
            panic!("packet from different track")
        };

        // Decode the packet into audio samples, ignoring any decode errors.
        match self.raw_decoder.decode(&packet) {
            Ok(audio_buf) => {
                // Get the audio buffer specification.
                let spec = *audio_buf.spec();

                // Get the capacity of the decoded buffer. Note: This is capacity, not length!
                let duration = audio_buf.capacity() as u64;

                // Create the f32 sample buffer.
                let mut sample_buf = RawSampleBuffer::new(duration, spec);

                // Copy the decoded audio buffer into the sample buffer in an interleaved format.
                sample_buf.copy_interleaved_ref(audio_buf);

                self.sample_count += sample_buf.len();

                Some(sample_buf)
            }
            Err(Error::DecodeError(e)) => panic!("decode error: {e}"),
            Err(e) => panic!("generic error: {e}"),
        }
    }

    pub fn sample_count(&self) -> usize {
        self.sample_count
    }

    /// Decode the next packet from the [Decoder] and copy it to the double buffer.
    ///
    /// # Return
    ///
    /// Returns [Ok] if the packet has been decoded correctly or [Err] if the packet couldn't be retrieved/decoded.
    pub fn decode_into_wave(
        &mut self,
        wave: &mut WaveInfo,
        leftovers: &mut Vec<u8>,
    ) -> Result<(), ()> {
        let buf = wave.get_buffer_mut().unwrap();

        let mut result = Vec::with_capacity(buf.len());
        result.append(leftovers);

        loop {
            let samples = match self.decode_next() {
                Some(s) => s,
                None => return Err(()),
            };

            let mut bytes = samples.as_bytes().to_vec();

            result.append(&mut bytes);

            print!("\rDecoded {} samples", self.sample_count());

            if result.len() > buf.len() {
                buf.copy_from_slice(&result[..buf.len()]);

                *leftovers = result.split_off(buf.len());
                break;
            } else if result.len() == buf.len() {
                buf.copy_from_slice(&result[..buf.len()]);
                break;
            }
        }

        Ok(())
    }
}
