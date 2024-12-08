//! DRED decoder
use crate::{sys, mem, Decoder, ErrorCode, SampleRate, Bandwidth};

use core::num;

///Opus DRED packet size (used to alloc memory)
pub const DRED_PACKET_SIZE: usize = 10592;

///Retrieves OPUS DRED packet size as per libopus requirements
pub fn dred_packet_size() -> usize {
    unsafe {
        sys::opus_dred_get_size() as _
    }
}

#[repr(transparent)]
///Opus DRED packet state
pub struct DredPacket {
    inner: mem::Unique<sys::OpusDRED>
}

impl DredPacket {
    ///Creates new uninitialized packet
    pub fn new() -> Result<Self, ErrorCode> {
        match mem::Unique::new(DRED_PACKET_SIZE) {
            Some(inner) => Ok(DredPacket {
                inner,
            }),
            None => Err(ErrorCode::AllocFail),
        }
    }
}

unsafe impl Send for DredPacket {}

///OPUS DRED Decoder
///
///## Parameters
///
///`CH` - Number of channels to use
pub struct Dred<const CH: u8> {
    inner: mem::Unique<sys::OpusDREDDecoder>,
    decoder: Decoder<CH>,
    packet: DredPacket,
}

impl<const CH: u8> Dred<CH> {
    ///Creates new decoder instance
    pub fn new(decoder: Decoder<CH>) -> Result<Self, ErrorCode> {
        let size = unsafe {
            sys::opus_dred_decoder_get_size()
        };

        if size == 0 {
            return Err(ErrorCode::Internal);
        }

        let packet = DredPacket::new()?;
        let mut decoder = match mem::Unique::new(size as _) {
            Some(inner) => Dred {
                inner,
                decoder,
                packet,
            },
            None => return Err(ErrorCode::AllocFail)
        };

        let result = unsafe {
            sys::opus_dred_decoder_init(decoder.inner.as_mut())
        };

        map_sys_error!(result => decoder)
    }

    ///Access underlying decoder
    pub fn decoder(&mut self) -> &Decoder<CH> {
        &self.decoder
    }

    ///Access underlying decoder
    pub fn decoder_mut(&mut self) -> &mut Decoder<CH> {
        &mut self.decoder
    }

    ///Decodes input packet, returning number of decoded samples.
    ///
    ///Output size must correspond to sampling rate.
    ///For example, at 48 kHz allowed frame sizes are 120, 240, 480, 960, 1920, and 2880.
    ///
    ///Maximum packet duration is 120ms therefore maximum `frame size` must be
    ///`frame_bytes_size(SampleRate::Hz48000, Channels::Stereo, 120)`
    pub fn decode_to(&mut self, input: &[u8], output: &mut [mem::MaybeUninit<i16>]) -> Result<usize, ErrorCode> {
        const MAX_SAMPLE_RATE: i32 = SampleRate::Hz48000 as _;

        let mut _dred_end = 0;
        let input_ptr = input.as_ptr();
        let input_len = input.len() as _;

        let frame_size = (output.len() / CH as usize) as _;

        let result = unsafe {
            sys::opus_dred_parse(self.inner.as_mut(), self.packet.inner.as_mut(),
                                 input_ptr, input_len,
                                 MAX_SAMPLE_RATE, MAX_SAMPLE_RATE,
                                 &mut _dred_end, 0)
        };

        if result < 0 {
            return Err(result.into());
        }

        let result = unsafe {
            sys::opus_decoder_dred_decode(
                self.decoder.inner.as_mut(), self.packet.inner.as_ptr(),
                frame_size, output.as_ptr() as _, frame_size
            )
        };

        map_sys_error!(result => result as _)
    }

    #[inline(always)]
    ///Decodes input packet, returning number of decoded samples.
    ///
    ///Refer to `decode_to` for details
    pub fn decode_to_slice(&mut self, input: &[u8], output: &mut [u16]) -> Result<usize, ErrorCode> {
        self.decode_to(input, unsafe { mem::transmute(output) })
    }

    ///Decodes input packet, returning number of decoded samples.
    ///
    ///Output size must correspond to sampling rate.
    ///For example, at 48 kHz allowed frame sizes are 120, 240, 480, 960, 1920, and 2880.
    ///
    ///Maximum packet duration is 120ms therefore maximum `frame size` must be
    ///`frame_bytes_size(SampleRate::Hz48000, Channels::Stereo, 120)`
    pub fn decode_float_to(&mut self, input: &[u8], output: &mut [mem::MaybeUninit<f32>]) -> Result<usize, ErrorCode> {
        const MAX_SAMPLE_RATE: i32 = SampleRate::Hz48000 as _;

        let mut _dred_end = 0;
        let input_ptr = input.as_ptr();
        let input_len = input.len() as _;

        let frame_size = (output.len() / CH as usize) as _;

        let result = unsafe {
            sys::opus_dred_parse(self.inner.as_mut(), self.packet.inner.as_mut(),
                                 input_ptr, input_len,
                                 MAX_SAMPLE_RATE, MAX_SAMPLE_RATE,
                                 &mut _dred_end, 0)
        };

        if result < 0 {
            return Err(result.into());
        }

        let result = unsafe {
            sys::opus_decoder_dred_decode_float(
                self.decoder.inner.as_mut(), self.packet.inner.as_ptr(),
                frame_size, output.as_ptr() as _, frame_size
            )
        };

        map_sys_error!(result => result as _)
    }

    #[inline(always)]
    ///Decodes input packet, returning number of decoded samples.
    ///
    ///Refer to `decode_to` for details
    pub fn decode_float_to_slice(&mut self, input: &[u8], output: &mut [f32]) -> Result<usize, ErrorCode> {
        self.decode_float_to(input, unsafe { mem::transmute(output) })
    }

    #[inline]
    ///Resets state to initial
    pub fn reset(&mut self) -> Result<(), ErrorCode> {
        self.decoder.reset()?;
        let result = unsafe {
            sys::opus_dred_decoder_ctl(self.inner.as_mut(), sys::OPUS_RESET_STATE)
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
            sys::opus_dred_decoder_ctl(self.inner.as_mut(), sys::OPUS_GET_PITCH_REQUEST, &mut value)
        };

        map_sys_error!(result => num::NonZeroU32::new(result as _))
    }

    #[inline]
    ///Gets the duration (in samples) of the last packet successfully decoded or concealed.
    pub fn get_last_packet_duration(&mut self) -> Result<u32, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_dred_decoder_ctl(self.inner.as_mut(), sys::OPUS_GET_LAST_PACKET_DURATION_REQUEST, &mut value)
        };

        map_sys_error!(result => value as _)
    }

    #[inline]
    ///Gets the decoder's gain configuration
    pub fn get_gain(&mut self) -> Result<i32, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_dred_decoder_ctl(self.inner.as_mut(), sys::OPUS_GET_GAIN_REQUEST, &mut value)
        };

        map_sys_error!(result => value)
    }

    #[inline]
    ///Configures decoder gain adjustment.
    ///
    ///Scales the decoded output by a factor specified in Q8 dB units.
    ///This has a maximum range of -32768 to 32767 inclusive, and returns `BadArg` otherwise.
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
            sys::opus_dred_decoder_ctl(self.inner.as_mut(), sys::OPUS_SET_GAIN_REQUEST, value)
        };

        map_sys_error!(result => ())
    }

    #[inline]
    ///Gets the decoder's last bandpass
    pub fn get_bandwidth(&mut self) -> Result<Bandwidth, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_dred_decoder_ctl(self.inner.as_mut(), sys::OPUS_GET_BANDWIDTH_REQUEST, &mut value)
        };

        map_sys_error!(result => value.into())
    }

    #[inline]
    ///Gets configured sample rate of this instance
    pub fn get_sample_rate(&mut self) -> Result<SampleRate, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_dred_decoder_ctl(self.inner.as_mut(), sys::OPUS_GET_SAMPLE_RATE_REQUEST, &mut value)
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
            sys::opus_dred_decoder_ctl(self.inner.as_mut(), sys::OPUS_GET_PHASE_INVERSION_DISABLED_REQUEST, &mut value)
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
            sys::opus_dred_decoder_ctl(self.inner.as_mut(), sys::OPUS_SET_PHASE_INVERSION_DISABLED_REQUEST, value)
        };

        map_sys_error!(result => ())
    }
}

unsafe impl<const CH: u8> Send for Dred<CH> {}
