use std::{env, fs::File, io::Read, time::Duration};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, SampleRate, StreamConfig,
};

fn main() {
    let mut opus_file = File::open(env::args().nth(1).unwrap()).unwrap();
    let mut ogg_reader = ogg_embedded::Reader::<16384, 1024>::new();

    let decoder_object_buffer = vec![0u8; opus_embedded::decoder_size(1)].leak();
    let mut opus_decoder = opus_embedded::Decoder::new(decoder_object_buffer, 48000, 1);

    let mut sample_buffer = [0i16; 8192];
    let mut read_position = 0;
    let mut write_position = 0;
    let mut packet_index = 0;

    let mut file_buffer = [0u8; 1024];
    let mut file_buffer_size = 0;
    let mut file_buffer_pos = 0;

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
                        if let Some(packet) = ogg_reader.next_packet() {
                            if packet_index >= 2 {
                                write_position =
                                    opus_decoder.decode(packet, &mut sample_buffer, false);
                            }
                            packet_index += 1;
                        } else {
                            if file_buffer_pos < file_buffer_size {
                                file_buffer_pos += ogg_reader
                                    .write(&file_buffer[file_buffer_pos..file_buffer_size]);
                            } else {
                                file_buffer_size = opus_file.read(&mut file_buffer).unwrap();
                                file_buffer_pos = 0;
                                if file_buffer_size == 0 {
                                    for sample in buffer {
                                        *sample = 0;
                                    }
                                    break;
                                }
                            }
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

mod ogg_embedded {
    use heapless::Vec;

    pub struct Reader<const PAGE: usize, const PACKET: usize> {
        input: Vec<u8, PAGE>,
        packet: Vec<u8, PACKET>,
        page_sequence_no: u32,
        packet_offset: usize,
        segment_index: usize,
        packet_state: PacketState,
    }

    #[derive(PartialEq, Eq)]
    enum PacketState {
        New,
        Continued,
        Dropping,
    }

    impl<const PAGE: usize, const PACKET: usize> Reader<PAGE, PACKET> {
        pub fn new() -> Reader<PAGE, PACKET> {
            Reader {
                input: Vec::new(),
                packet: Vec::new(),
                page_sequence_no: 0,
                packet_offset: 0,
                segment_index: 0,
                packet_state: PacketState::New,
            }
        }

        pub fn write(&mut self, data: &[u8]) -> usize {
            if self.input.len() == self.input.capacity() {
                println!("input buffer full, discarding first page");
                self.discard_first_page();
            }
            let read_size = data.len().min(self.input.capacity() - self.input.len());
            self.input.extend_from_slice(&data[..read_size]).unwrap();

            if let Some(pos) = find_potential_capture_pattern(&self.input) {
                if pos > 0 {
                    self.input.copy_within(pos.., 0);
                    self.input.resize_default(self.input.len() - pos).unwrap();
                }

                self.try_parse_page();
            }

            read_size
        }

        pub fn next_packet(&mut self) -> Option<&[u8]> {
            if self.packet_offset == 0 {
                return None;
            }

            let segment_count = self.input[26] as usize;

            while self.segment_index < segment_count {
                if self.packet_state == PacketState::New {
                    self.packet.clear();
                }

                let size = self.input[27 + self.segment_index] as usize;
                if self.packet_state != PacketState::Dropping {
                    if let Err(_) = self.packet.extend_from_slice(
                        &self.input[self.packet_offset..self.packet_offset + size],
                    ) {
                        self.packet_state = PacketState::Dropping;
                    }
                }
                self.segment_index += 1;
                self.packet_offset += size;
                if size < 255 {
                    if core::mem::replace(&mut self.packet_state, PacketState::New)
                        != PacketState::Dropping
                    {
                        return Some(&self.packet);
                    }
                } else {
                    self.packet_state = PacketState::Continued;
                }
            }

            self.discard_first_page();
            self.try_parse_page();
            return self.next_packet();
        }

        fn discard_first_page(&mut self) {
            self.packet_offset = 0;
            if let Some(pos) = find_potential_capture_pattern(&self.input[1..]) {
                self.input.copy_within(pos + 1.., 0);
                self.input
                    .resize_default(self.input.len() - 1 - pos)
                    .unwrap();
            } else {
                self.input.clear();
            }
        }

        fn try_parse_page(&mut self) {
            if self.input.len() < 27 {
                return;
            }

            if &self.input[..4] != b"OggS" || self.input[4] != 0 {
                return;
            }

            let segment_count = self.input[26] as usize;
            if self.input.len() < 27 + segment_count {
                return;
            }

            let total_size = 27
                + segment_count
                + self.input[27..27 + segment_count]
                    .iter()
                    .map(|&s| s as usize)
                    .sum::<usize>();

            if self.input.len() < total_size {
                return;
            }

            let header_flags = self.input[5];
            let continued_packet = header_flags & 1 != 0;
            // let granule_position = u64::from_le_bytes(self.input[6..14].try_into().unwrap());
            // let stream_serial = u32::from_le_bytes(self.input[14..18].try_into().unwrap());
            let page_sequence_no = u32::from_le_bytes(self.input[18..22].try_into().unwrap());
            let checksum_bytes: [u8; 4] = self.input[22..26].try_into().unwrap();
            let page_checksum = u32::from_le_bytes(checksum_bytes.clone());

            self.input[22..26].copy_from_slice(&[0, 0, 0, 0]); // set crc bytes to zero
            let actual_checksum = vorbis_crc32(&self.input[..total_size]);
            self.input[22..26].copy_from_slice(&checksum_bytes); // restore crc bytes

            if actual_checksum != page_checksum {
                println!(
                    "actual_checksum: {:08x}, page_checksum: {:08x}",
                    actual_checksum, page_checksum
                );
                self.discard_first_page();
                return self.try_parse_page(); // tail-call, pretty please? ;)
            }

            if continued_packet {
                if self.packet_state != PacketState::Continued
                    || page_sequence_no != self.page_sequence_no + 1
                {
                    self.packet_state = PacketState::Dropping;
                }
            } else {
                self.packet_state = PacketState::New;
            }
            self.page_sequence_no = page_sequence_no;
            self.packet_offset = 27 + segment_count;
            self.segment_index = 0;
        }
    }

    fn find_potential_capture_pattern(buffer: &[u8]) -> Option<usize> {
        buffer
            .windows(4)
            .enumerate()
            .find(|(_, p)| p == b"OggS")
            .map(|(i, _)| i)
            .or_else(|| {
                (buffer.len().saturating_sub(3)..buffer.len()).find(|&i| {
                    buffer[i] == b'O'
                        && buffer.get(i + 1).copied().unwrap_or(b'g') == b'g'
                        && buffer.get(i + 2).copied().unwrap_or(b'g') == b'g'
                })
            })
    }

    // TODO: license or rewrite
    // Lookup table to enable bytewise CRC32 calculation
    static CRC_LOOKUP_ARRAY: &[u32] = &lookup_array();

    const fn get_tbl_elem(idx: u32) -> u32 {
        let mut r: u32 = idx << 24;
        let mut i = 0;
        while i < 8 {
            r = (r << 1) ^ (-(((r >> 31) & 1) as i32) as u32 & 0x04c11db7);
            i += 1;
        }
        return r;
    }

    const fn lookup_array() -> [u32; 0x100] {
        let mut lup_arr: [u32; 0x100] = [0; 0x100];
        let mut i = 0;
        while i < 0x100 {
            lup_arr[i] = get_tbl_elem(i as u32);
            i += 1;
        }
        lup_arr
    }

    pub fn vorbis_crc32(array: &[u8]) -> u32 {
        let mut ret: u32 = 0;
        for av in array {
            ret = (ret << 8) ^ CRC_LOOKUP_ARRAY[(*av as u32 ^ (ret >> 24)) as usize];
        }
        return ret;
    }
}
