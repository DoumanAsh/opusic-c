use crate::{sys, mem, ErrorCode, Channels, SampleRate, Bandwidth};

use core::{ptr, num};
use core::convert::TryInto;

#[repr(transparent)]
///OPUS Decoder
pub struct Decoder<const CH: u8> {
    inner: mem::Unique<sys::OpusDecoder>
}

impl<const CH: u8> Decoder<CH> {
    const CHANNELS: Channels = match CH {
        1 => Channels::Mono,
        2 => Channels::Stereo,
        _ => panic!("Unsupported number of channels. Allowed range: 1..=2"),
    };

    ///Creates new decoder instance
    pub fn new(rate: SampleRate) -> Result<Self, ErrorCode> {
        let size = unsafe {
            sys::opus_decoder_get_size(Self::CHANNELS as _)
        };

        if size == 0 {
            return Err(ErrorCode::Internal);
        }

        let mut decoder = match mem::Unique::new(size as _) {
            Some(inner) => Decoder {
                inner,
            },
            None => return Err(ErrorCode::AllocFail)
        };

        let result = unsafe {
            sys::opus_decoder_init(decoder.inner.as_mut(), rate as _, Self::CHANNELS as _)
        };

        map_sys_error!(result => decoder)
    }

    ///Decodes input packet, returning number of decoded samples.
    ///
    ///If more than 1 channel is configured, then input must be interleaved.
    ///
    ///Output size must correspond to sampling rate.
    ///For example, at 48 kHz allowed frame sizes are 120, 240, 480, 960, 1920, and 2880.
    ///
    ///Maximum packet duration is 120ms therefore maximum `frame size` must be
    ///`frame_bytes_size(SampleRate::Hz48000, Channels::Stereo, 120)`
    ///
    ///When `input` size is 0, libopus shall treat it as packet loss, in which case `output` size must
    ///match expected output of next packet to know how much frames is skipped
    ///
    ///When `decode_fec` is `true`, requests that any in-band forward error correction data be decoded.
    ///If no such data is available, the frame is decoded as if it were lost.
    pub fn decode_to(&mut self, input: &[u8], output: &mut [mem::MaybeUninit<i16>], decode_fec: bool) -> Result<usize, ErrorCode> {
        let (input_ptr, input_len) = match input.len() {
            0 => (ptr::null(), 0),
            len => (input.as_ptr(), len as _)
        };

        let fec = match decode_fec {
            true => 1,
            false => 0,
        };
        let result = unsafe {
            sys::opus_decode(self.inner.as_mut(),
                             input_ptr, input_len,
                             output.as_mut_ptr() as _, (output.len() / CH as usize) as _,
                             fec)
        };

        map_sys_error!(result => result as _)
    }

    #[inline(always)]
    ///Decodes input packet, returning number of decoded samples.
    ///
    ///Refer to `decode_to` for details
    pub fn decode_to_slice(&mut self, input: &[u8], output: &mut [u16], decode_fec: bool) -> Result<usize, ErrorCode> {
        self.decode_to(input, unsafe { mem::transmute(output) }, decode_fec)
    }

    ///Decodes input packet, returning number of decoded samples.
    ///
    ///If more than 1 channel is configured, then input must be interleaved.
    ///
    ///Output size must correspond to sampling rate.
    ///For example, at 48 kHz allowed frame sizes are 120, 240, 480, 960, 1920, and 2880.
    ///
    ///Maximum packet duration is 120ms therefore maximum `frame size` must be
    ///`frame_bytes_size(SampleRate::Hz48000, Channels::Stereo, 120)`
    ///
    ///When `input` size is 0, libopus shall treat it as packet loss, in which case `output` size must
    ///match expected output of next packet to know how much frames is skipped
    ///
    ///When `decode_fec` is `true`, requests that any in-band forward error correction data be decoded.
    ///If no such data is available, the frame is decoded as if it were lost.
    pub fn decode_float_to(&mut self, input: &[u8], output: &mut [mem::MaybeUninit<f32>], decode_fec: bool) -> Result<usize, ErrorCode> {
        let (input_ptr, input_len) = match input.len() {
            0 => (ptr::null(), 0),
            len => (input.as_ptr(), len as _)
        };
        let fec = match decode_fec {
            true => 1,
            false => 0,
        };

        let result = unsafe {
            sys::opus_decode_float(self.inner.as_mut(),
                                   input_ptr, input_len,
                                   output.as_mut_ptr() as _, (output.len() / CH as usize) as _,
                                   fec)
        };

        map_sys_error!(result => result as _)
    }

    #[inline(always)]
    ///Decodes input packet, returning number of decoded samples.
    ///
    ///Refer to `decode_to` for details
    pub fn decode_float_to_slice(&mut self, input: &[u8], output: &mut [f32], decode_fec: bool) -> Result<usize, ErrorCode> {
        self.decode_float_to(input, unsafe { mem::transmute(output) }, decode_fec)
    }

    ///Gets the number of samples of an Opus packet.
    pub fn get_nb_samples(&self, input: &[u8]) -> Result<usize, ErrorCode> {
        let len = match input.len().try_into() {
            Ok(len) => len,
            //This is very unlikely if user gets valid packets fed
            Err(_) => return Err(ErrorCode::invalid_packet()),
        };

        let result = unsafe {
            sys::opus_decoder_get_nb_samples(self.inner.as_ptr(), input.as_ptr() as *const _, len)
        };

        map_sys_error!(result => result as _)
    }

    #[inline]
    ///Resets state to initial
    pub fn reset(&mut self) -> Result<(), ErrorCode> {
        let result = unsafe {
            sys::opus_decoder_ctl(self.inner.as_mut(), sys::OPUS_RESET_STATE)
        };

        map_sys_error!(result => ())
    }

    #[inline]
    ///Gets the pitch of the last decoded frame, if available.
    ///
    ///This can be used for any post-processing algorithm requiring the use of pitch, e.g. time
    ///stretching/shortening. If the last frame was not voiced, or if the pitch was not coded in
    ///the frame, then zero is returned.
    pub fn get_pitch(&mut self) -> Result<Option<num::NonZeroU32>, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_decoder_ctl(self.inner.as_mut(), sys::OPUS_GET_PITCH_REQUEST, &mut value)
        };

        map_sys_error!(result => num::NonZeroU32::new(result as _))
    }

    #[inline]
    ///Gets the duration (in samples) of the last packet successfully decoded or concealed.
    pub fn get_last_packet_duration(&mut self) -> Result<u32, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_decoder_ctl(self.inner.as_mut(), sys::OPUS_GET_LAST_PACKET_DURATION_REQUEST, &mut value)
        };

        map_sys_error!(result => value as _)
    }

    #[inline]
    ///Gets the decoder's gain configuration
    pub fn get_gain(&mut self) -> Result<i32, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_decoder_ctl(self.inner.as_mut(), sys::OPUS_GET_GAIN_REQUEST, &mut value)
        };

        map_sys_error!(result => value)
    }

    #[inline]
    ///Configures decoder gain adjustment.
    ///
    ///Scales the decoded output by a factor specified in Q8 dB units.
    ///This has a maximum range of -32768 to 32767 inclusive, and returns OPUS_BAD_ARG otherwise.
    ///
    ///The default is zero indicating no adjustment.
    ///
    ///_This setting survives decoder reset_.
    ///
    ///Formula:
    ///
    ///`gain = pow(10, x/(20.0*256))`
    pub fn set_gain(&mut self, value: i32) -> Result<(), ErrorCode> {
        let result = unsafe {
            sys::opus_decoder_ctl(self.inner.as_mut(), sys::OPUS_SET_GAIN_REQUEST, value)
        };

        map_sys_error!(result => ())
    }

    #[inline]
    ///Gets the decoder's last bandpass
    pub fn get_bandwidth(&mut self) -> Result<Bandwidth, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_decoder_ctl(self.inner.as_mut(), sys::OPUS_GET_BANDWIDTH_REQUEST, &mut value)
        };

        map_sys_error!(result => value.into())
    }

    #[inline]
    ///Gets configured sample rate of this instance
    pub fn get_sample_rate(&mut self) -> Result<SampleRate, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_decoder_ctl(self.inner.as_mut(), sys::OPUS_GET_SAMPLE_RATE_REQUEST, &mut value)
        };

        map_sys_error!(result => match value {
            8000 => SampleRate::Hz8000,
            12000 => SampleRate::Hz12000,
            16000 => SampleRate::Hz16000,
            24000 => SampleRate::Hz24000,
            48000 => SampleRate::Hz48000,
            _ => return Err(ErrorCode::unknown())
        })
    }

    #[inline]
    ///Gets the decoder's configured phase inversion status.
    pub fn get_phase_inversion_disabled(&mut self) -> Result<bool, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_decoder_ctl(self.inner.as_mut(), sys::OPUS_GET_PHASE_INVERSION_DISABLED_REQUEST, &mut value)
        };

        map_sys_error!(result => value == 1)
    }

    #[inline]
    ///Configures phase inversion.
    ///
    ///If set to `true`, disables the use of phase inversion for intensity stereo, improving the quality
    ///of mono downmixes, but slightly reducing normal stereo quality.
    ///
    ///Disabling phase inversion in the decoder does not comply with RFC 6716, although it does not
    ///cause any interoperability issue and is expected to become part of the Opus standard once
    ///RFC 6716 is updated by draft-ietf-codec-opus-update.
    pub fn set_phase_inversion_disabled(&mut self, value: bool) -> Result<(), ErrorCode> {
        let value: i32 = match value {
            true => 1,
            false => 0,
        };

        let result = unsafe {
            sys::opus_decoder_ctl(self.inner.as_mut(), sys::OPUS_SET_PHASE_INVERSION_DISABLED_REQUEST, value)
        };

        map_sys_error!(result => ())
    }
}
