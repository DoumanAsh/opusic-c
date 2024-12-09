use crate::{sys, mem, ErrorCode, Application, SampleRate, Bandwidth, Bitrate, Signal, InbandFec, FrameDuration};
use super::Config;

#[repr(transparent)]
///OPUS multistream encoder
///
pub struct Encoder {
    inner: mem::Unique<sys::OpusMSEncoder>
}

impl Encoder {
    ///Creates new encoder instance
    ///
    ///## Notes
    ///
    ///### Mapping
    ///
    ///`config.mapping` table defines which decoded channel i should be used for each input/output (I/O) channel j.
    ///
    ///Let `i` = `mapping[j]` be the index for I/O channel `j`.
    ///
    ///If `i` < `2*coupled_streams`, then I/O channel `j` is encoded as the left channel of stream `(i/2)` if `i` is even,
    ///or as the right channel of stream `(i/2)` if `i` is odd.
    ///
    ///Otherwise, I/O channel `j` is encoded as mono in stream `(i - coupled_streams)`,
    ///unless it has the special value **255**,
    ///in which case it is omitted from the encoding entirely (the decoder will reproduce it as silence).
    ///
    ///Each value `i` must either be the special value **255** or be less than `streams + coupled_streams`.
    pub fn new<const CH: usize>(config: Config<CH>, rate: SampleRate, app: Application) -> Result<Self, ErrorCode> {
        let size = unsafe {
            sys::opus_multistream_encoder_get_size(config.streams as _, config.coupled_streams as _)
        };

        if size == 0 {
            return Err(ErrorCode::Internal);
        }

        let mut encoder = match mem::Unique::new(size as _) {
            Some(inner) => Encoder {
                inner,
            },
            None => return Err(ErrorCode::AllocFail)
        };

        let result = unsafe {
            sys::opus_multistream_encoder_init(encoder.inner.as_mut(), rate as _, CH as _, config.streams as _, config.coupled_streams as _, config.mapping.as_ptr() as _, app as _)
        };

        map_sys_error!(result => encoder)
    }

    #[inline]
    ///Resets state to initial state
    pub fn reset(&mut self) -> Result<(), ErrorCode> {
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_RESET_STATE)
        };

        map_sys_error!(result => ())
    }

    #[inline]
    ///Gets the total samples of delay added by the entire codec.
    ///
    ///From the perspective of a decoding application the real data begins this many samples late.
    pub fn get_look_ahead(&mut self) -> Result<u32, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_GET_LOOKAHEAD_REQUEST, &mut value)
        };

        map_sys_error!(result => match value.is_negative() {
            false => value as _,
            true => return Err(ErrorCode::unknown())
        })
    }

    #[inline]
    ///Gets the encoder's bitrate configuration.
    pub fn get_bitrate(&mut self) -> Result<Bitrate, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_GET_BITRATE_REQUEST, &mut value)
        };

        map_sys_error!(result => value.into())
    }

    #[inline]
    ///Configures the encoder's bitrate
    pub fn set_bitrate(&mut self, value: Bitrate) -> Result<(), ErrorCode> {
        let value: i32 = value.into();
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_SET_BITRATE_REQUEST, value)
        };

        map_sys_error!(result => ())
    }

    #[inline]
    ///Determine if variable bitrate (VBR) is enabled in the encoder.
    pub fn get_vbr(&mut self) -> Result<bool, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_GET_VBR_REQUEST, &mut value)
        };

        map_sys_error!(result => value == 1)
    }

    #[inline]
    ///Enables or disables variable bitrate (VBR) in the encoder.
    ///
    ///The configured bitrate may not be met exactly because frames must be an integer number of bytes in length.
    pub fn set_vbr(&mut self, value: bool) -> Result<(), ErrorCode> {
        let value: i32 = match value {
            true => 1,
            false => 0
        };
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_SET_VBR_REQUEST, value)
        };

        map_sys_error!(result => ())
    }

    #[inline]
    ///Determine if constrained VBR is enabled in the encoder.
    pub fn get_vbr_constraint(&mut self) -> Result<bool, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_GET_VBR_CONSTRAINT_REQUEST, &mut value)
        };

        map_sys_error!(result => value == 1)
    }

    #[inline]
    ///Enables or disables constrained VBR in the encoder.
    ///
    ///This setting is ignored when the encoder is in CBR mode.
    ///
    ///## Note
    ///
    ///Only the MDCT mode of Opus currently heeds the constraint. Speech mode ignores it
    ///completely, hybrid mode may fail to obey it if the LPC layer uses more bitrate than the
    ///constraint would have permitted.
    pub fn set_vbr_constraint(&mut self, value: bool) -> Result<(), ErrorCode> {
        let value: i32 = match value {
            true => 1,
            false => 0
        };
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_SET_VBR_CONSTRAINT_REQUEST, value)
        };

        map_sys_error!(result => ())
    }

    #[inline]
    ///Gets the encoder's complexity configuration.
    pub fn get_complexity(&mut self) -> Result<u8, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_GET_COMPLEXITY_REQUEST, &mut value)
        };

        map_sys_error!(result => value as _)
    }

    #[inline]
    ///Configures the encoder's computational complexity.
    ///
    ///The supported range is 0-10 inclusive with 10 representing the highest complexity.
    pub fn set_complexity(&mut self, value: u8) -> Result<(), ErrorCode> {
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_SET_COMPLEXITY_REQUEST, value as i32)
        };

        map_sys_error!(result => ())
    }

    #[inline]
    ///Gets the encoder's configured signal type.
    pub fn get_signal(&mut self) -> Result<Signal, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_GET_SIGNAL_REQUEST, &mut value)
        };

        map_sys_error!(result => value.into())
    }

    #[inline]
    ///Configures the type of signal being encoded.
    ///
    ///This is a hint which helps the encoder's mode selection.
    pub fn set_signal(&mut self, value: Signal) -> Result<(), ErrorCode> {
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_SET_SIGNAL_REQUEST, value as i32)
        };

        map_sys_error!(result => ())
    }

    #[inline]
    ///Gets the encoder's configured application.
    pub fn get_application(&mut self) -> Result<Application, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_GET_APPLICATION_REQUEST, &mut value)
        };

        map_sys_error!(result => match Application::from_sys(value) {
            Some(value) => value,
            None => return Err(ErrorCode::unknown())
        })
    }

    #[inline]
    ///Configures the encoder's intended application.
    ///
    ///The initial value is a mandatory argument to encoder constructor.
    pub fn set_application(&mut self, value: Application) -> Result<(), ErrorCode> {
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_SET_APPLICATION_REQUEST, value as i32)
        };

        map_sys_error!(result => ())
    }

    #[inline]
    ///Gets the encoder's configured bandpass
    pub fn get_bandwidth(&mut self) -> Result<Bandwidth, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_GET_BANDWIDTH_REQUEST, &mut value)
        };

        map_sys_error!(result => value.into())
    }

    #[inline]
    ///Sets the encoder's bandpass to a specific value.
    ///
    ///This prevents the encoder from automatically selecting the bandpass based on the available
    ///bitrate. If an application knows the bandpass of the input audio it is providing, it should
    ///normally use `set_max_bandwidth` instead, which still gives the encoder the freedom to
    ///reduce the bandpass when the bitrate becomes too low, for better overall quality.
    pub fn set_bandwidth(&mut self, value: Bandwidth) -> Result<(), ErrorCode> {
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_SET_BANDWIDTH_REQUEST, value as i32)
        };

        map_sys_error!(result => ())
    }

    #[inline]
    ///Configures the maximum bandpass that the encoder will select automatically.
    ///
    ///Applications should normally use this instead of `set_bandwidth` (leaving that set to the
    ///default, `Bandwidth::Auto`). This allows the application to set an upper bound based on the type of
    ///input it is providing, but still gives the encoder the freedom to reduce the bandpass when
    ///the bitrate becomes too low, for better overall quality.
    pub fn set_max_bandwidth(&mut self, value: Bandwidth) -> Result<(), ErrorCode> {
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_SET_MAX_BANDWIDTH_REQUEST, value as i32)
        };

        map_sys_error!(result => ())
    }

    #[inline]
    ///Gets encoder's configured use of inband forward error correction.
    pub fn get_inband_fec(&mut self) -> Result<InbandFec, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_GET_INBAND_FEC_REQUEST, &mut value)
        };

        map_sys_error!(result => match value {
            0 => InbandFec::Off,
            1 => InbandFec::Mode1,
            2 => InbandFec::Mode2,
            _ => return Err(ErrorCode::unknown()),
        })
    }

    #[inline]
    ///Configures the encoder's use of inband forward error correction (FEC).
    ///
    ///## Note
    ///
    ///This is only applicable to the LPC layer
    pub fn set_inband_fec(&mut self, value: InbandFec) -> Result<(), ErrorCode> {
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_SET_INBAND_FEC_REQUEST, value as i32)
        };

        map_sys_error!(result => ())
    }

    #[inline]
    ///Gets the encoder's configured packet loss percentage.
    pub fn get_packet_loss(&mut self) -> Result<u8, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_GET_PACKET_LOSS_PERC_REQUEST, &mut value)
        };

        map_sys_error!(result => value as _)
    }

    #[inline]
    ///Configures the encoder's expected packet loss percentage (Allowed values are 0..=100).
    ///
    ///Higher values trigger progressively more loss resistant behavior in the encoder at the
    ///expense of quality at a given bitrate in the absence of packet loss, but greater quality
    ///under loss.
    pub fn set_packet_loss(&mut self, value: u8) -> Result<(), ErrorCode> {
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_SET_PACKET_LOSS_PERC_REQUEST, value as i32)
        };

        map_sys_error!(result => ())
    }

    #[inline]
    ///Gets the encoder's configured prediction status.
    pub fn get_prediction_disabled(&mut self) -> Result<bool, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_GET_PREDICTION_DISABLED_REQUEST, &mut value)
        };

        map_sys_error!(result => value == 1)
    }

    #[inline]
    ///If set to `true`, disables almost all use of prediction, making frames almost completely independent.
    ///
    ///This reduces quality.
    pub fn set_prediction_disabled(&mut self, value: bool) -> Result<(), ErrorCode> {
        let value: i32 = match value {
            true => 1,
            false => 0,
        };

        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_SET_PREDICTION_DISABLED_REQUEST, value)
        };

        map_sys_error!(result => ())
    }

    #[inline]
    ///Gets the encoder's configured signal depth.
    pub fn get_lsb_depth(&mut self) -> Result<u8, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_GET_LSB_DEPTH_REQUEST, &mut value)
        };

        map_sys_error!(result => value as _)
    }

    #[inline]
    ///Configures the depth of signal being encoded (Defaults to 24) in range 8 to 24.
    ///
    ///This is a hint which helps the encoder identify silence and near-silence. It represents the
    ///number of significant bits of linear intensity below which the signal contains ignorable
    ///quantization or other noise.
    ///
    ///For example, 14 would be an appropriate setting for G.711 u-law input.
    ///16 would be appropriate for 16-bit linear pcm input with `encode_float`.
    ///
    ///When using `encode` instead of `encode_float`, or when libopus is compiled for
    ///fixed-point, the encoder uses the minimum of the value set here and the value 16.
    pub fn set_lsb_depth(&mut self, value: u8) -> Result<(), ErrorCode> {
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_SET_LSB_DEPTH_REQUEST, value as i32)
        };

        map_sys_error!(result => ())
    }

    #[inline]
    ///Gets the encoder's configured use of variable duration frames.
    pub fn get_frame_duration(&mut self) -> Result<FrameDuration, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_GET_EXPERT_FRAME_DURATION_REQUEST, &mut value)
        };

        map_sys_error!(result => match value {
            sys::OPUS_FRAMESIZE_ARG => FrameDuration::SizeArg,
            sys::OPUS_FRAMESIZE_2_5_MS => FrameDuration::Size2_5,
            sys::OPUS_FRAMESIZE_5_MS => FrameDuration::Size5,
            sys::OPUS_FRAMESIZE_10_MS => FrameDuration::Size10,
            sys::OPUS_FRAMESIZE_20_MS => FrameDuration::Size20,
            sys::OPUS_FRAMESIZE_40_MS => FrameDuration::Size40,
            sys::OPUS_FRAMESIZE_60_MS => FrameDuration::Size60,
            sys::OPUS_FRAMESIZE_80_MS => FrameDuration::Size80,
            sys::OPUS_FRAMESIZE_100_MS => FrameDuration::Size100,
            sys::OPUS_FRAMESIZE_120_MS => FrameDuration::Size120,
            _ => return Err(ErrorCode::unknown()),
        })
    }

    #[inline]
    ///Configures the encoder's use of variable duration frames.
    ///
    ///When variable duration is enabled, the encoder is free to use a shorter frame size than the
    ///one requested in the `encode` call. It is then the user's responsibility to verify how
    ///much audio was encoded by checking the ToC byte of the encoded packet. The part of the audio
    ///that was not encoded needs to be resent to the encoder for the next call. Do not use this
    ///option unless you really know what you are doing.
    pub fn set_frame_duration(&mut self, value: FrameDuration) -> Result<(), ErrorCode> {
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_SET_EXPERT_FRAME_DURATION_REQUEST, value as i32)
        };

        map_sys_error!(result => ())
    }

    #[inline]
    ///Gets configured sample rate of this instance
    pub fn get_sample_rate(&mut self) -> Result<SampleRate, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_GET_SAMPLE_RATE_REQUEST, &mut value)
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
    ///Access encoder's DTX value
    pub fn get_dtx(&mut self) -> Result<bool, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_GET_DTX_REQUEST, &mut value)
        };

        map_sys_error!(result => value == 1)
    }

    #[inline]
    ///Configures the encoder's use of discontinuous transmission (DTX).
    ///
    ///This is only applicable to the LPC layer
    pub fn set_dtx(&mut self, value: bool) -> Result<(), ErrorCode> {
        let value: i32 = match value {
            true => 1,
            false => 0,
        };

        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_SET_DTX_REQUEST, value)
        };

        map_sys_error!(result => ())
    }

    #[inline]
    ///Gets the encoder's configured phase inversion status.
    pub fn get_phase_inversion_disabled(&mut self) -> Result<bool, ErrorCode> {
        let mut value: i32 = 0;
        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_GET_PHASE_INVERSION_DISABLED_REQUEST, &mut value)
        };

        map_sys_error!(result => value == 1)
    }

    #[inline]
    ///Configures phase inversion.
    ///
    ///If set to `true`, disables the use of phase inversion for intensity stereo, improving the quality
    ///of mono downmixes, but slightly reducing normal stereo quality.
    pub fn set_phase_inversion_disabled(&mut self, value: bool) -> Result<(), ErrorCode> {
        let value: i32 = match value {
            true => 1,
            false => 0,
        };

        let result = unsafe {
            sys::opus_multistream_encoder_ctl(self.inner.as_mut(), sys::OPUS_SET_PHASE_INVERSION_DISABLED_REQUEST, value)
        };

        map_sys_error!(result => ())
    }
}

unsafe impl Send for Encoder {}
