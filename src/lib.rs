//!High level bindings to [libopus](https://github.com/xiph/opus)
//!
//!Target version [1.5.2](https://github.com/xiph/opus/releases/tag/v1.5.2)
//!
//!## Allocator
//!
//!This library uses Rust's allocator whenever possible
//!
//!## Features
//!
//!- `dred` - Enables experimental DRED decoder

#![no_std]
#![warn(missing_docs)]
#![allow(clippy::style)]
#![allow(clippy::missing_transmute_annotations)]

use core::{slice, str};

pub use opusic_sys as sys;

macro_rules! map_sys_error {
    ($result:expr => $ok:expr) => {{
        let result = $result;
        if result < 0 {
            Err(result.into())
        } else {
            Ok($ok)
        }
    }};
}

mod mem;
mod encoder;
pub use encoder::*;
mod decoder;
pub use decoder::*;
#[cfg(feature = "dred")]
pub mod dred;
pub mod utils;

///Computes OPUS frame size in bytes for specified duration
pub const fn frame_bytes_size(sample_rate: SampleRate, channels: Channels, duration_ms: usize) -> usize {
    ((sample_rate as usize) * (channels as usize) * duration_ms) / 1000
}

const _FRAME_SIZE_TEST: () = {
    assert!(frame_bytes_size(SampleRate::Hz48000, Channels::Mono, 10) == 480);
    assert!(frame_bytes_size(SampleRate::Hz48000, Channels::Stereo, 10) == 960);
};

#[repr(i32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
///Underlying libopus error codes
pub enum ErrorCode {
    ///No error
    Ok = sys::OPUS_OK,
    ///One or more invalid/out of range arguments
    BadArg = sys::OPUS_BAD_ARG,
    ///Memory allocation has failed
    AllocFail = sys::OPUS_ALLOC_FAIL,
    ///An encoder or decoder structure is invalid or already freed
    InvalidState = sys::OPUS_INVALID_STATE,
    ///The compressed data passed is corrupted
    InvalidPacket = sys::OPUS_INVALID_PACKET,
    ///Not enough bytes allocated in the buffer
    BufferTooSmall = sys::OPUS_BUFFER_TOO_SMALL,
    ///An internal error was detected
    Internal = sys::OPUS_INTERNAL_ERROR,
    ///Invalid/unsupported request number
    Unimplemented = sys::OPUS_UNIMPLEMENTED,
    ///Unknown error variant. Should not be possible
    Unknown = -200,
}

impl ErrorCode {
    #[cold]
    #[inline(never)]
    const fn unknown() -> Self {
        Self::Unknown
    }

    #[cold]
    #[inline(never)]
    const fn invalid_packet() -> Self {
        Self::InvalidPacket
    }

    #[inline]
    ///Returns text representation of error
    pub const fn message(&self) -> &'static str {
        match self {
            Self::Ok => "No error",
            Self::BadArg => "One or more invalid/out of range arguments",
            Self::AllocFail => "Memory allocation has failed",
            Self::InvalidState => "An encoder or decoder structure is invalid or already freed",
            Self::InvalidPacket => "The compressed data passed is corrupted",
            Self::BufferTooSmall => "Not enough bytes allocated in the buffer",
            Self::Internal => "An internal error was detected",
            Self::Unimplemented => "Invalid/unsupported request number",
            Self::Unknown => "Unknown error",
        }
    }
}

impl From<i32> for ErrorCode {
    #[inline]
    fn from(value: i32) -> Self {
        match value {
            sys::OPUS_OK => Self::Ok,
            sys::OPUS_UNIMPLEMENTED => Self::Unimplemented,
            sys::OPUS_INVALID_STATE => Self::InvalidState,
            sys::OPUS_INVALID_PACKET => Self::InvalidPacket,
            sys::OPUS_INTERNAL_ERROR => Self::Internal,
            sys::OPUS_BUFFER_TOO_SMALL => Self::BufferTooSmall,
            sys::OPUS_BAD_ARG => Self::BadArg,
            sys::OPUS_ALLOC_FAIL => Self::AllocFail,
            _ => Self::unknown(),
        }
    }
}

///Codec's bitrate configuration
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Bitrate {
    ///Value set in bits rates per second
    Value(u32),
    ///Default setting. Determined by number of channels and sample rate
    Auto,
    ///Specifies to the codec to use as much rate as it can,
    ///which is useful for controlling the rate by adjusting the output buffer size
    Max,
}

impl From<i32> for Bitrate {
    #[inline(always)]
    fn from(value: i32) -> Self {
        match value {
            sys::OPUS_AUTO => Self::Auto,
            //This actually cannot happen (because it is only instruction to set max value)
            //But just in case have it
            sys::OPUS_BITRATE_MAX => Self::Max,
            value => Self::Value(value as _)
        }
    }
}

impl From<Bitrate> for i32 {
    #[inline(always)]
    fn from(value: Bitrate) -> Self {
        match value {
            Bitrate::Max => sys::OPUS_BITRATE_MAX,
            Bitrate::Auto => sys::OPUS_AUTO,
            Bitrate::Value(value) => value as _,
        }
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
///Coding mode
pub enum Application {
    ///Best for most VoIP/videoconference applications where listening quality and intelligibility matter most.
    Voip = sys::OPUS_APPLICATION_VOIP,
    ///Best for broadcast/high-fidelity application where the decoded audio should be as close as possible to the input.
    Audio = sys::OPUS_APPLICATION_AUDIO,
    ///Only use when lowest-achievable latency is what matters most.
    ///
    ///Voice-optimized modes cannot be used.
    LowDelay = sys::OPUS_APPLICATION_RESTRICTED_LOWDELAY,
}

impl Application {
    #[inline(always)]
    const fn from_sys(value: i32) -> Option<Self> {
        match value {
            sys::OPUS_APPLICATION_AUDIO => Some(Self::Audio),
            sys::OPUS_APPLICATION_VOIP => Some(Self::Voip),
            sys::OPUS_APPLICATION_RESTRICTED_LOWDELAY => Some(Self::LowDelay),
            _ => None,
        }
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
///Possible sample rates to use
pub enum SampleRate {
    ///8000
    Hz8000 = 8000,
    ///12000
    Hz12000 = 12000,
    ///16000
    Hz16000 = 16000,
    ///24000
    Hz24000 = 24000,
    ///48000
    Hz48000 = 48000,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
///The available bandwidth level settings.
pub enum Bandwidth {
    ///Auto/default setting.
    Auto = sys::OPUS_AUTO,
    ///4kHz bandpass.
    Narrow = sys::OPUS_BANDWIDTH_NARROWBAND,
    ///6kHz bandpass.
    Medium = sys::OPUS_BANDWIDTH_MEDIUMBAND,
    ///8kHz bandpass.
    Wide = sys::OPUS_BANDWIDTH_WIDEBAND,
    ///12kHz bandpass.
    Superwide = sys::OPUS_BANDWIDTH_SUPERWIDEBAND,
    ///20kHz bandpass.
    Full = sys::OPUS_BANDWIDTH_FULLBAND,
}

impl From<i32> for Bandwidth {
    #[inline(always)]
    fn from(value: i32) -> Self {
        match value {
            sys::OPUS_BANDWIDTH_FULLBAND => Self::Full,
            sys::OPUS_BANDWIDTH_SUPERWIDEBAND => Self::Superwide,
            sys::OPUS_BANDWIDTH_WIDEBAND => Self::Wide,
            sys::OPUS_BANDWIDTH_MEDIUMBAND => Self::Medium,
            sys::OPUS_BANDWIDTH_NARROWBAND => Self::Narrow,
            _ => Self::Auto
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
///Number of channels
pub enum Channels {
    ///Single channel
    Mono = 1,
    ///Two channels
    Stereo = 2,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
///Signal type
pub enum Signal {
    ///Default value
    Auto = sys::OPUS_AUTO,
    ///Bias thresholds towards choosing LPC or Hybrid modes
    Voice = sys::OPUS_SIGNAL_VOICE,
    ///Bias thresholds towards choosing MDCT modes
    Music = sys::OPUS_SIGNAL_MUSIC,
}

impl From<i32> for Signal {
    #[inline(always)]
    fn from(value: i32) -> Self {
        match value {
            sys::OPUS_SIGNAL_MUSIC => Self::Music,
            sys::OPUS_SIGNAL_VOICE => Self::Voice,
            _ => Self::Auto
        }
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
///Possible values of inband forward error correction configuration.
pub enum InbandFec {
    ///Inband FEC disabled (default)
    Off = 0,
    ///Inband FEC enabled.
    ///
    ///If the packet loss rate is sufficiently high,
    ///Opus will automatically switch to SILK even at high rates to enable use of that FEC.
    Mode1 = 1,
    ///Inband FEC enabled, but does not necessarily switch to SILK if we have music.
    Mode2 = 2,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
///Frame duration configuration values
pub enum FrameDuration {
    ///Select frame size from the argument (default)
    SizeArg = sys::OPUS_FRAMESIZE_ARG,
    ///Use 2.5 ms frames
    Size2_5 = sys::OPUS_FRAMESIZE_2_5_MS,
    ///Use 5 ms frames
    Size5 = sys::OPUS_FRAMESIZE_5_MS,
    ///Use 10 ms frames
    Size10 = sys::OPUS_FRAMESIZE_10_MS,
    ///Use 20 ms frames
    Size20 = sys::OPUS_FRAMESIZE_20_MS,
    ///Use 40 ms frames
    Size40 = sys::OPUS_FRAMESIZE_40_MS,
    ///Use 60 ms frames
    Size60 = sys::OPUS_FRAMESIZE_60_MS,
    ///Use 80 ms frames
    Size80 = sys::OPUS_FRAMESIZE_80_MS,
    ///Use 100 ms frames
    Size100 = sys::OPUS_FRAMESIZE_100_MS,
    ///Use 120 ms frames
    Size120 = sys::OPUS_FRAMESIZE_120_MS,
}

///Returns libopus version
pub fn version() -> &'static str {
    //Version string is always valid ASCII string so no need to worry about utf-8 validity
    unsafe {
        let ptr = sys::opus_get_version_string();
        let mut len = 0usize;

        while *ptr.add(len) != 0 {
            len = len.saturating_add(1);
        }

        let slice = slice::from_raw_parts(ptr as _, len);
        core::str::from_utf8_unchecked(slice)
    }
}
