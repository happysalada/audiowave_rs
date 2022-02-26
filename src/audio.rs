use crate::utils::{div_up, get_extension_from_filename};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::io::Cursor;
use symphonia::core::{
    audio::{AudioBufferRef, Signal},
    codecs::{DecoderOptions, CODEC_TYPE_NULL},
    errors::Error,
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};
use worker::console_log;

#[derive(Default, Debug, Clone, Serialize)]
pub struct AmplitudeMinMax {
    min: f32,
    max: f32,
}

impl AmplitudeMinMax {
    pub fn add(&mut self, sample: f32) {
        if sample <= self.min {
            self.min = sample;
            return;
        }
        if sample >= self.max {
            self.max = sample;
        }
    }
}

pub fn get_waveform(name: String, bytes: Vec<u8>) -> Result<Vec<AmplitudeMinMax>> {
    // Create a probe hint using the file's extension. [Optional]
    let mut hint = Hint::new();
    if let Some(extension) = get_extension_from_filename(&name) {
        hint.with_extension(extension);
    }

    let cursor = Cursor::new(bytes);
    let media_source_stream = MediaSourceStream::new(Box::new(cursor), Default::default());

    // Use the default options for metadata and format readers.
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    // Probe the media source.
    let probed = symphonia::default::get_probe()
        .format(&hint, media_source_stream, &fmt_opts, &meta_opts)
        .expect("unsupported format");

    // Get the instantiated format reader.
    let mut format = probed.format;

    // Find the first audio track with a known (decodeable) codec.
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .expect("no supported audio tracks");

    // Use the default options for the decoder.
    let dec_opts: DecoderOptions = Default::default();

    // Create a decoder for the track.
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)
        .expect("unsupported codec");

    // Store the track identifier, it will be used to filter packets.
    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap();
    let n_frames = track.codec_params.n_frames.unwrap();
    let n_seconds = div_up(n_frames, sample_rate);
    let mut waveform: Vec<AmplitudeMinMax> = vec![AmplitudeMinMax::default(); n_seconds];

    // The decode loop.
    loop {
        // Get the next packet from the media format.
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::ResetRequired) => {
                // The track list has been changed. Re-examine it and create a new set of decoders,
                // then restart the decode loop. This is an advanced feature and it is not
                // unreasonable to consider this "the end." As of v0.5.0, the only usage of this is
                // for chained OGG physical streams.
                unimplemented!();
            }
            //
            Err(Error::IoError(_)) => {
                break;
            }
            Err(err) => {
                // A unrecoverable error occured, halt decoding.
                console_log!("{:?}", err);
                return Err(err).context("error getting next packet");
            }
        };

        // Consume any new metadata that has been read since the last packet.
        while !format.metadata().is_latest() {
            // Pop the old head of the metadata queue.
            format.metadata().pop();

            // Consume the new metadata at the head of the metadata queue.
            if let Some(rev) = format.metadata().current() {
                console_log!("{:?}", rev);
            }
        }

        // If the packet does not belong to the selected track, skip over it.
        if packet.track_id() != track_id {
            continue;
        }

        // Decode the packet into audio samples.
        match decoder.decode(&packet) {
            Ok(decoded) => {
                use AudioBufferRef::*;
                match decoded {
                    F32(buf) => {
                        for (index, sample) in buf.chan(0).iter().enumerate() {
                            let second = ((packet.ts + index as u64) / sample_rate as u64) as usize;
                            let amplitude = waveform.get_mut(second).unwrap();
                            amplitude.add(*sample);
                        }
                    }
                    U8(_) | U16(_) | U24(_) | U32(_) | S8(_) | S16(_) | S24(_) | S32(_)
                    | F64(_) => bail!("encountered unhandled format"),
                }
                // Consume the decoded audio samples (see below).
            }
            Err(Error::IoError(_)) => {
                // The packet failed to decode due to an IO error, skip the packet.
                continue;
            }
            Err(Error::DecodeError(_)) => {
                // The packet failed to decode due to invalid data, skip the packet.
                continue;
            }
            Err(err) => {
                // A unrecoverable error occured, halt decoding.
                console_log!("{:?}", err);
                return Err(err).context("irrecoverable error while decoding the packet");
            }
        }
    }

    Ok(waveform)
}
