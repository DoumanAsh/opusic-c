//! Opus packet manipulation
use crate::{sys, mem, ErrorCode};

use core::convert::TryInto;

///Pads a given Opus packet to a larger size (possibly changing the TOC sequence).
///
///Returns `ErrorCode::BadArg` if size cannot fit `u32` or new size is less than `input.len()`
pub fn pad_packet(input: &mut [u8], new_len: usize) -> Result<(), ErrorCode> {
    let len = match input.len().try_into() {
        Ok(data_len) => data_len,
        Err(_) => return Err(ErrorCode::bad_arg()),
    };
    let new_len = match new_len.try_into() {
        Ok(data_len) => data_len,
        Err(_) => return Err(ErrorCode::bad_arg()),
    };
    let data = input.as_mut_ptr();

    let result = unsafe {
        sys::opus_packet_pad(data, len, new_len)
    };

    map_sys_error!(result => ())
}

///Remove all padding from a given Opus packet and rewrite the TOC sequence to minimize space usage.
///
///Returns `ErrorCode::BadArg` if size cannot fit `u32`
///
///On success returns new size of the `input` data
pub fn unpad_packet(input: &mut [u8]) -> Result<usize, ErrorCode> {
    let len = match input.len().try_into() {
        Ok(data_len) => data_len,
        Err(_) => return Err(ErrorCode::bad_arg()),
    };
    let data = input.as_mut_ptr();

    let result = unsafe {
        sys::opus_packet_unpad(data, len)
    };

    map_sys_error!(result => result as usize)
}

#[must_use]
///Currently being processed packet
///
///User must ensure that all packets are accessible until `Repacketizer` is no longer in use or `reset`
///
///Dropping this guard without ensuring `Repacketizer` is reset, leaves opportunity for undefined behavior
pub struct PacketHolder<'a>(pub &'a [u8]);

#[repr(transparent)]
///Repacketizer can be used to merge multiple Opus packets into a single packet or alternatively to split Opus packets that have previously been merged
pub struct Repacketizer {
    inner: mem::Unique<sys::OpusRepacketizer>
}

impl Repacketizer {
    ///Creates new instance, allocating necessary memory
    pub fn new() -> Result<Self, ErrorCode> {
        let size = unsafe {
            sys::opus_repacketizer_get_size()
        };

        if size == 0 {
            return Err(ErrorCode::Internal);
        }

        let mut this = match mem::Unique::new(size as _) {
            Some(inner) => Self {
                inner,
            },
            None => return Err(ErrorCode::AllocFail)
        };

        this.reset();

        Ok(this)
    }

    #[inline(always)]
    ///Re-initializes this instance, resetting any ongoing progress, if any.
    pub fn reset(&mut self) {
        unsafe {
            sys::opus_repacketizer_init(self.inner.as_mut());
        }
    }

    #[inline(always)]
    ///Return the total number of frames contained in packet data submitted to the repacketizer state
    ///since the last time `reset` has been called
    pub fn get_nb_frames(&self) -> u32 {
        unsafe {
            sys::opus_repacketizer_get_nb_frames(self.inner.as_pseudo_mut()) as _
        }
    }

    ///Add a packet to the current repacketizer state.
    ///
    ///This packet must match the configuration of any packets already submitted for
    ///repacketization since the last call to `reset()`. This means that it must
    ///have the same coding mode, audio bandwidth, frame size, and channel count. This can be
    ///checked in advance by examining the top 6 bits of the first byte of the packet, and ensuring
    ///they match the top 6 bits of the first byte of any previously submitted packet. The total
    ///duration of audio in the repacketizer state also must not exceed 120 ms, the maximum
    ///duration of a single packet, after adding this packet.
    ///
    ///In order to add a packet with a different configuration or to add more audio beyond 120 ms,
    ///you must clear the repacketizer state by calling `reset()`. If a packet is
    ///too large to add to the current repacketizer state, no part of it is added, even if it
    ///contains multiple frames, some of which might fit. If you wish to be able to add parts of
    ///such packets, you should first use another repacketizer to split the packet into pieces and
    ///add them individually.
    pub fn add_packet<'a>(&mut self, input: &'a [u8]) -> Result<PacketHolder<'a>, ErrorCode> {
        let len = match input.len().try_into() {
            Ok(data_len) => data_len,
            Err(_) => return Err(ErrorCode::bad_arg()),
        };
        let data = input.as_ptr();

        let result = unsafe {
            sys::opus_repacketizer_cat(self.inner.as_mut(), data, len)
        };

        map_sys_error!(result => PacketHolder(input))
    }

    ///Construct a new packet from data previously submitted to the repacketizer state
    ///
    ///## Parameters
    ///
    ///- `range` - Should contain range of frames to encode in range `0..=get_nb_frames()`
    ///- `out` - Output buffer to store new packet. Max required size can be calculated as `1277*(range.1 - range.0)`. Optimal required size is `(range.1 - range.0) + [packet1 length]...`
    ///
    ///## Return value
    ///
    ///Number of bytes written in `out` buffer
    pub fn create_packet(&self, range: (u32, u32), out: &mut [mem::MaybeUninit<u8>]) -> Result<usize, ErrorCode> {
        let begin = match range.0.try_into() {
            Ok(begin) => begin,
            Err(_) => return Err(ErrorCode::bad_arg()),
        };
        let end = match range.1.try_into() {
            Ok(end) => end,
            Err(_) => return Err(ErrorCode::bad_arg()),
        };

        let out_len = match out.len().try_into() {
            Ok(out_len) => out_len,
            Err(_) => return Err(ErrorCode::bad_arg()),
        };

        let result = unsafe {
            sys::opus_repacketizer_out_range(self.inner.as_pseudo_mut(), begin, end, out.as_mut_ptr() as _, out_len)
        };

        map_sys_error!(result => result as _)
    }

    #[inline(always)]
    ///Construct a new packet from data previously submitted to the repacketizer state using all
    ///frames available
    ///
    ///This is the same as calling `create_packet((0, nb_frames), ...)`
    pub fn create_full_packet(&self, out: &mut [mem::MaybeUninit<u8>]) -> Result<usize, ErrorCode> {
        let out_len = match out.len().try_into() {
            Ok(out_len) => out_len,
            Err(_) => return Err(ErrorCode::bad_arg()),
        };

        let result = unsafe {
            sys::opus_repacketizer_out(self.inner.as_pseudo_mut(), out.as_mut_ptr() as _, out_len)
        };

        map_sys_error!(result => result as _)
    }
}

unsafe impl Send for Repacketizer {}
