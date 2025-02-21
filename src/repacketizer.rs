//! Opus packet manipulation
use crate::{sys, mem, ErrorCode};

use core::marker;
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
    ///Re-initializes this Repacketizer state, resetting ongoing progress, if any.
    pub fn reset(&mut self) {
        unsafe {
            sys::opus_repacketizer_init(self.inner.as_mut());
        }
    }

    #[inline(always)]
    ///Starts new repacketizer process, resetting `self` in the process.
    pub fn start<'a, 'buf>(&'a mut self) -> OngoingRepacketizer<'a, 'buf> {
        OngoingRepacketizer {
            inner: self,
            _buf_lifetime: marker::PhantomData
        }
    }

    ///Takes all `bufs` combining it into single packet
    ///
    ///This is shortcut to using [start](struct.Repacketizer.html#method.start).
    pub fn combine_all(&mut self, bufs: &[&[u8]], out: &mut [mem::MaybeUninit<u8>]) -> Result<usize, ErrorCode> {
        let mut state = self.start();
        for buf in bufs {
            state.add_packet(buf)?;
        }
        state.create_full_packet(out)
    }
}

unsafe impl Send for Repacketizer {}

#[repr(transparent)]
///Ongoing repacketizer process
///
///Lifetime parameters:
///
///- `a` - Depends on original instance of [Repacketizer](struct.Repacketizer.html)
///- `buf` - Lifetime of the last buffer added to the state. Note that all previous lifetimes must fit it too
///
///Dropping state will reset [Repacketizer](struct.Repacketizer.html)
pub struct OngoingRepacketizer<'a, 'buf> {
    inner: &'a mut Repacketizer,
    _buf_lifetime: marker::PhantomData<&'buf [u8]>
}

impl<'a, 'buf> OngoingRepacketizer<'a, 'buf> {
    #[inline(always)]
    fn as_state(&self) -> &mem::Unique<sys::OpusRepacketizer> {
        &self.inner.inner
    }

    #[inline(always)]
    fn as_state_mut(&mut self) -> &mut mem::Unique<sys::OpusRepacketizer> {
        &mut self.inner.inner
    }

    #[inline(always)]
    ///Re-initializes this Repacketizer state, resetting ongoing progress, if any.
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    #[inline(always)]
    ///Return the total number of frames contained in packet data submitted to the repacketizer state
    ///since the last time `reset` has been called
    pub fn get_nb_frames(&self) -> u32 {
        unsafe {
            sys::opus_repacketizer_get_nb_frames(self.as_state().as_pseudo_mut()) as _
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
    pub fn add_packet(&mut self, input: &'buf [u8]) -> Result<(), ErrorCode> {
        let len = match input.len().try_into() {
            Ok(data_len) => data_len,
            Err(_) => return Err(ErrorCode::bad_arg()),
        };
        let data = input.as_ptr();

        let result = unsafe {
            sys::opus_repacketizer_cat(self.as_state_mut().as_mut(), data, len)
        };

        map_sys_error!(result => ())
    }

    #[inline(always)]
    ///Adds packet to the ongoing state, returning `Self` with modified lifetime
    ///
    ///Refers to [add_packet](struct.OngoingRepacketizer.html#method.add_packet) for details
    pub fn with_packet<'new_buf>(self, input: &'new_buf [u8]) -> Result<OngoingRepacketizer<'a, 'new_buf>, ErrorCode> where 'buf: 'new_buf {
        let mut new = self;
        new.add_packet(input)?;
        Ok(new)
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
            sys::opus_repacketizer_out_range(self.as_state().as_pseudo_mut(), begin, end, out.as_mut_ptr() as _, out_len)
        };

        map_sys_error!(result => result as _)
    }

    #[inline(always)]
    ///Construct a new packet from data previously submitted to the repacketizer state using all frames available
    ///
    ///This is the same as calling `create_packet((0, nb_frames), ...)`
    pub fn create_full_packet(&self, out: &mut [mem::MaybeUninit<u8>]) -> Result<usize, ErrorCode> {
        let out_len = match out.len().try_into() {
            Ok(out_len) => out_len,
            Err(_) => return Err(ErrorCode::bad_arg()),
        };

        let result = unsafe {
            sys::opus_repacketizer_out(self.as_state().as_pseudo_mut(), out.as_mut_ptr() as _, out_len)
        };

        map_sys_error!(result => result as _)
    }
}

impl<'a, 'buf> Drop for OngoingRepacketizer<'a, 'buf> {
    #[inline(always)]
    fn drop(&mut self) {
        self.inner.reset();
    }
}
