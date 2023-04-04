use std::cmp::Ordering;
use std::fs::File;

use symphonia::core::audio::{Channels, RawSampleBuffer};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct Decoder {
    // `symphonia`-related members
    track_id: u32,
    raw_decoder: Box<dyn symphonia::core::codecs::Decoder>,
    format: Box<dyn FormatReader>,
    sample_count: usize,
    // Custom decoding members
    /// Leftovers (left channel, right channel)
    leftovers: (Vec<u8>, Vec<u8>),
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
            leftovers: (Vec::new(), Vec::new()),
        }
    }

    /// Returns the next decoded packet (if there is one).
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

                // Must be "stereo" audio
                assert!(
                    spec.channels
                        .contains(Channels::FRONT_LEFT & Channels::FRONT_RIGHT),
                    "decoded audio was not stereo"
                );

                // Get the capacity of the decoded buffer. Note: This is capacity, not length!
                let duration = audio_buf.capacity() as u64;

                // Create the f32 sample buffer.
                let mut sample_buf = RawSampleBuffer::new(duration, spec);

                // Copy the decoded audio buffer into the sample buffer in an interleaved format.
                sample_buf.copy_planar_ref(audio_buf);

                self.sample_count += sample_buf.len();

                Some(sample_buf)
            }
            Err(Error::DecodeError(e)) => panic!("decode error: {e}"),
            Err(e) => panic!("generic error: {e}"),
        }
    }

    /// Runs [`Decoder::decode_next`] until the desired length (in bytes) is reached.
    ///
    /// # Returns
    ///
    /// Returns the decoded data split in "left" and "right" channel.
    pub fn decode_until(&mut self, max_len: usize) -> (Vec<u8>, Vec<u8>) {
        let mut result = (Vec::new(), Vec::new());

        result.0.append(&mut self.leftovers.0);
        result.1.append(&mut self.leftovers.1);

        loop {
            assert_eq!(result.0.len(), result.1.len());

            match result.0.len().cmp(&max_len) {
                // We have more than we can handle: leave some leftovers
                Ordering::Greater => {
                    self.leftovers = (result.0.split_off(max_len), result.1.split_off(max_len));

                    return result;
                }
                // Just perfect: we can return
                Ordering::Equal => {
                    return result;
                }
                // We can keep on decoding
                _ => {
                    let samples = match self.decode_next() {
                        Some(s) => s,
                        // Premature EOF: we can return what we have
                        None => {
                            return result;
                        }
                    };

                    let (left_channel, right_channel) =
                        samples.as_bytes().split_at(samples.len() / 2);

                    // If the lengths aren't equal it's definetly symphonia's fault
                    assert_eq!(left_channel.len(), right_channel.len());

                    result.0.append(&mut left_channel.to_vec());
                    result.1.append(&mut right_channel.to_vec());

                    print!("\rDecoded {} samples", self.sample_count());
                }
            }
        }
    }

    pub fn sample_count(&self) -> usize {
        self.sample_count
    }
}
