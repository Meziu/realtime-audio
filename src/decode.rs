use std::fs::File;

use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::audio::RawSampleBuffer;

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

    pub fn decode_next(&mut self) -> RawSampleBuffer<i16> {
        // Get the next packet from the format reader.
        let packet = self.format.next_packet().unwrap();

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

                sample_buf
            }
            Err(Error::DecodeError(e)) => panic!("decode error: {e}"),
            Err(e) => panic!("generic error: {e}"),
        }
    }

    pub fn sample_count(&self) -> usize {
        self.sample_count
    }
}
