#![no_std]
use core::ffi::c_int;

pub fn decoder_size(channel_count: usize) -> usize {
    assert!(channel_count == 1 || channel_count == 2);
    let channels = channel_count as c_int;
    let size = unsafe { opus_sys::opus_decoder_get_size(channels as c_int) };
    size.try_into().expect("size should fit into usize")
}

pub struct Decoder<'a>(&'a mut [u8], usize);

impl<'a> Decoder<'a> {
    pub fn new(buffer: &'a mut [u8], sample_rate: u32, channel_count: usize) -> Decoder<'a> {
        assert!(buffer.len() >= decoder_size(channel_count));
        let result = unsafe {
            opus_sys::opus_decoder_init(
                buffer.as_mut_ptr(),
                sample_rate as i32,
                channel_count as c_int,
            )
        };
        assert!(result == opus_sys::OPUS_OK);
        Decoder(buffer, channel_count)
    }

    pub fn decode(&mut self, data: &[u8], samples: &mut [i16], fec: bool) -> usize {
        let frame_size = samples.len() / self.1;
        let result = unsafe {
            opus_sys::opus_decode(
                self.0.as_mut_ptr(),
                data.as_ptr(),
                data.len() as i32,
                samples.as_mut_ptr(),
                frame_size as i32,
                fec as c_int,
            )
        };
        assert!(result >= 0);
        result as usize
    }
}

mod opus_sys {
    #![allow(dead_code)]
    use core::ffi::{c_int, c_uchar};

    extern "C" {
        pub fn opus_decoder_get_size(channels: c_int) -> c_int;
        pub fn opus_decoder_init(st: *mut u8, fs: i32, channels: c_int) -> c_int;
        pub fn opus_decode(
            st: *mut u8,
            data: *const c_uchar,
            len: i32,
            pcm: *mut i16,
            frame_size: c_int,
            decode_fec: c_int,
        ) -> c_int;
    }

    pub const OPUS_OK: c_int = 0;
    pub const OPUS_BAD_ARG: c_int = -1;
    pub const OPUS_BUFFER_TOO_SMALL: c_int = -2;
    pub const OPUS_INTERNAL_ERROR: c_int = -3;
    pub const OPUS_INVALID_PACKET: c_int = -4;
    pub const OPUS_UNIMPLEMENTED: c_int = -5;
    pub const OPUS_INVALID_STATE: c_int = -6;
    pub const OPUS_ALLOC_FAIL: c_int = -7;
}
