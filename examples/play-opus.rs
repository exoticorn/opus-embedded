use std::{env, fs::File, time::Duration};

use anyhow::{anyhow, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, SampleRate, StreamConfig,
};

const CHANNEL_COUNT: usize = 2;

fn main() -> Result<()> {
    // use ogg crate to demuc ogg/opus file to simplify testing
    // note that this crate might not be viable for embedded
    let mut ogg_reader = ogg::PacketReader::new(File::open(
        env::args()
            .nth(1)
            .ok_or_else(|| anyhow!("Opus file parameter missing"))?,
    )?);

    // skip id and comment headers
    _ = ogg_reader.read_packet_expected()?;
    _ = ogg_reader.read_packet_expected()?;

    // create &'static mut buffer for the opus decoder object
    // on embedded, you'd probably just use an array in a static_cell for this
    let decoder_object_buffer =
        vec![0u8; opus_embedded::Decoder::required_buffer_size(CHANNEL_COUNT)].leak();
    let mut opus_decoder = opus_embedded::Decoder::new(decoder_object_buffer, 48000, CHANNEL_COUNT)
        .map_err(|err| anyhow!("Failed to create opus decoder: {}", err))?;
    opus_decoder.set_gain(-256 * 18);

    // buffer for decoded opus frame, contains data in range read_position..write_position
    let mut sample_buffer = [0i16; 8192 * CHANNEL_COUNT];
    let mut read_position = 0;
    let mut write_position = 0;

    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();
    let stream = device
        .build_output_stream(
            &StreamConfig {
                channels: CHANNEL_COUNT as u16,
                sample_rate: SampleRate(48000),
                buffer_size: BufferSize::Default,
            },
            move |mut output_buffer: &mut [i16], _: &cpal::OutputCallbackInfo| {
                while !output_buffer.is_empty() {
                    if read_position == write_position {
                        // no more data in buffer, read and decode next packet
                        if let Some(packet) = ogg_reader.read_packet().unwrap() {
                            write_position = opus_decoder
                                .decode(Some(&packet.data), &mut sample_buffer, false)
                                .unwrap()
                                * CHANNEL_COUNT;
                            read_position = 0;
                        } else {
                            for sample in output_buffer {
                                *sample = 0;
                            }
                            break;
                        }
                    } else {
                        // copy samples from sample_buffer into output_buffer
                        let copy_size = output_buffer.len().min(write_position - read_position);
                        output_buffer[..copy_size].copy_from_slice(
                            &sample_buffer[read_position..read_position + copy_size],
                        );
                        read_position += copy_size;
                        output_buffer = &mut output_buffer[copy_size..];
                    }
                }
            },
            move |err| {
                eprintln!("Output stream error: {}", err);
            },
            None,
        )
        .unwrap();

    stream.play().unwrap();

    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
