use std::{env, fs::File, time::Duration};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, SampleRate, StreamConfig,
};

fn main() {
    let decoder_object_buffer = vec![0u8; opus_embedded::decoder_size(1)].leak();
    let mut opus_decoder = opus_embedded::Decoder::new(decoder_object_buffer, 48000, 1);

    let mut ogg_reader = ogg::PacketReader::new(File::open(env::args().nth(1).unwrap()).unwrap());
    let mut sample_buffer = [0i16; 8192];
    let mut read_position = 0;
    let mut write_position = 0;

    _ = ogg_reader.read_packet().unwrap(); // id
    _ = ogg_reader.read_packet().unwrap(); // comment

    let host = cpal::default_host();
    let device = host.default_output_device().unwrap();
    let stream = device
        .build_output_stream(
            &StreamConfig {
                channels: 1,
                sample_rate: SampleRate(48000),
                buffer_size: BufferSize::Default,
            },
            move |mut buffer: &mut [i16], _: &cpal::OutputCallbackInfo| {
                while !buffer.is_empty() {
                    if write_position == 0 {
                        if let Some(packet) = ogg_reader.read_packet().unwrap() {
                            write_position =
                                opus_decoder.decode(&packet.data, &mut sample_buffer, false);
                        } else {
                            for sample in buffer {
                                *sample = 0;
                            }
                            break;
                        }
                    } else {
                        let copy_size = buffer.len().min(write_position - read_position);
                        buffer[..copy_size].copy_from_slice(
                            &sample_buffer[read_position..read_position + copy_size],
                        );
                        read_position += copy_size;
                        if read_position == write_position {
                            read_position = 0;
                            write_position = 0;
                        }
                        buffer = &mut buffer[copy_size..];
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
