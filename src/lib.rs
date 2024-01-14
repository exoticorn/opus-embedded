#![no_std]
#![deny(missing_docs)]

//! An embedded-fiendly wrapper around libopus.
//!
//! Allows decoding Opus audio in `no_std` environments without allocator.

use core::ffi::c_int;

#[derive(Debug)]
/// libopus error codes
pub enum Error {
    /// One or more invalid/out of range arguments
    BadArg,
    /// Not enough bytes allocated in the buffer
    BufferTooSmall,
    /// An internal error was detected
    InternalError,
    /// The compressed data passed is corrupted
    InvalidPacket,
    /// Invalid/unsupported request number
    Unimplemented,
    // InvalidState,
    // AllocFail,
}

impl Error {
    fn from_c(err: c_int) -> Error {
        match err {
            -1 => Error::BadArg,
            -2 => Error::BufferTooSmall,
            -3 => Error::InternalError,
            -4 => Error::InvalidPacket,
            -5 => Error::Unimplemented,
            // -6 => Error::InvalidState,
            // -7 => Error::AllocFail,
            _ => Error::InternalError,
        }
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let err = match self {
            &Error::BadArg => "One or more invalid/out of range arguments",
            &Error::BufferTooSmall => "Not enough bytes allocated in the buffer",
            &Error::InternalError => "An internal error was detected",
            &Error::InvalidPacket => "The compressed data passed is corrupted",
            &Error::Unimplemented => "Invalid/unsupported request number",
            // &Error::InvalidState => "An encoder or decode structure is invalid or already freed",
            // &Error::AllocFail => "Memory allocation has failed",
        };
        write!(f, "{}", err)
    }
}

type Result<T> = core::result::Result<T, Error>;

/// An Opus decoder, allocated from a given buffer to facilitate using it in `no_std`
/// environments with no allocator.
pub struct Decoder<'a>(&'a mut [u8], usize);

impl<'a> Decoder<'a> {
    /// Create Opus decoder.
    ///
    /// This initializes the Opus decoder in the given buffer. The buffer has to be at least
    /// `Decoder::required_buffer_size(channel_count)` bytes in size.
    ///
    /// # Parameters
    ///
    /// - `buffer`: buffer for the decoder object
    /// - `sample_rate`: sampling rate to decode to (Hz). This must be one of 8000, 12000,
    ///                  16000, 24000, or 48000.
    /// - `channel_count`: number of channels (1 or 2) to decode
    ///
    /// # Returns
    ///
    /// Either tje newly initialized `Decoder` or an `Error`
    pub fn new(
        buffer: &'a mut [u8],
        sample_rate: u32,
        channel_count: usize,
    ) -> Result<Decoder<'a>> {
        if buffer.len() < Decoder::required_buffer_size(channel_count) {
            return Err(Error::BufferTooSmall);
        }
        let result = unsafe {
            opus_sys::opus_decoder_init(
                buffer.as_mut_ptr(),
                sample_rate as i32,
                channel_count as c_int,
            )
        };
        if result < 0 {
            return Err(Error::from_c(result));
        }
        Ok(Decoder(buffer, channel_count))
    }

    /// Decode on Opus packet
    ///
    /// # Parameters
    ///
    /// - `data`: the Opus packet, or `None` to indicate packet loss
    /// - `samples`: output sample buffer (interleaved if 2 channels). If the size is less than
    ///              the maximum packet duration (120ms; 5760 for 48kHz), this function will not
    ///              be capable of decoding some packets. In the case of PLC (`data == None`) or
    ///              FEC (`fec == true`), then the size needs to be exactly the duration of audio
    ///              that is missing, otherwise the decoder will not be in the optimal state to
    ///              decode the next incoming packet. For the PLC and FEC cases, size must be a
    ///              multiple of 2.5 ms.
    /// - `fec`: flag to request that any in-band forward error correction data be decoded. If no
    ///          such data is available, the frame is decoded as if it were lost.
    pub fn decode(&mut self, data: Option<&[u8]>, samples: &mut [i16], fec: bool) -> Result<usize> {
        let frame_size = samples.len() / self.1;
        if frame_size * self.1 != samples.len() {
            return Err(Error::BufferTooSmall);
        }
        let (data_ptr, data_len) = data
            .map(|d| (d.as_ptr(), d.len()))
            .unwrap_or((0 as *const u8, 0));
        let result = unsafe {
            opus_sys::opus_decode(
                self.0.as_mut_ptr(),
                data_ptr,
                data_len as i32,
                samples.as_mut_ptr(),
                frame_size as i32,
                fec as c_int,
            )
        };
        if result >= 0 {
            Ok(result as usize)
        } else {
            Err(Error::from_c(result))
        }
    }

    /// Configures decoder gain adjustment.
    ///
    /// Scales the decoded output by a factor specified in Q8 dB units. The default is zero indicating no adjustment. This setting survives decoder reset.
    ///
    /// gain = pow(10, x/(20.0*256))
    ///
    /// # Parameters
    ///
    /// `gain`: gain in Q8 db units
    pub fn set_gain(&mut self, gain: i16) {
        unsafe {
            opus_sys::opus_decoder_ctl(
                self.0.as_mut_ptr(),
                opus_sys::OPUS_SET_GAIN_REQUEST,
                gain as c_int,
            );
        }
    }

    /// Returns the size of the Decoder object in bytes.
    ///
    /// Use this to query the size of the buffer you need to pass into `Decoder::new`.
    /// It only depends on the number of channels and is otherwise constant for a given
    /// compile of this library.
    ///
    /// # Parameters
    ///
    /// - `channel_count`: The number of channels, same as in `Decoder::new`
    ///
    /// # Returns
    ///
    /// The number of bytes required fo create the decoder.
    pub fn required_buffer_size(channel_count: usize) -> usize {
        assert!(channel_count == 1 || channel_count == 2);
        let channels = channel_count as c_int;
        let size = unsafe { opus_sys::opus_decoder_get_size(channels as c_int) };
        size.try_into().expect("size should fit into usize")
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
        pub fn opus_decoder_ctl(st: *mut u8, request: c_int, ...) -> c_int;
    }

    pub const OPUS_SET_GAIN_REQUEST: c_int = 4034;
}
