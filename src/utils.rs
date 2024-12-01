//! Utility functions

use crate::{sys, mem, SampleRate, Channels, ErrorCode};

#[inline]
///Gets the number of frames in an Opus packet.
pub fn get_nb_frames(input: &[u8]) -> Result<usize, ErrorCode> {
    let result = unsafe {
        sys::opus_packet_get_nb_frames(input.as_ptr(), input.len() as _)
    };

    map_sys_error!(result => result as _)
}

#[inline]
///Gets the number of samples of an Opus packet.
pub fn get_nb_samples(input: &[u8], rate: SampleRate) -> Result<usize, ErrorCode> {
    let result = unsafe {
        sys::opus_packet_get_nb_samples(input.as_ptr(), input.len() as _, rate as _)
    };

    map_sys_error!(result => result as _)
}

#[inline]
///Applies soft-clipping to bring a float signal within the [-1,1] range.
///
///If the signal is already in that range, nothing is done.
///
///If there are values outside of [-1,1],
///then the signal is clipped as smoothly as possible to both fit in the range and
///avoid creating excessive distortion in the process.
pub fn soft_clip(input: &mut [f32], channels: Channels) {
    let mut soft_clip_mem = mem::MaybeUninit::<[f32; 2]>::uninit();
    unsafe {
        sys::opus_pcm_soft_clip(
            input.as_mut_ptr(), (input.len() / channels as usize) as _,
            channels as _,
            soft_clip_mem.as_mut_ptr() as _
        )
    }
}
