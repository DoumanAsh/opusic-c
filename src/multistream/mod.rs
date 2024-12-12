//!The multistream API allows individual Opus streams to be combined into a single packet, enabling support for up to 255 channels
//!
//!Multistream Opus streams can contain up to 255 elementary Opus streams.
//!These may be either "uncoupled" or "coupled", indicating that the decoder is configured to decode them to either 1 or 2 channels, respectively.
//!The streams are ordered so that all coupled streams appear at the beginning.
//!
//!The output channels specified by the encoder should use the [Vorbis channel ordering](https://www.xiph.org/vorbis/doc/Vorbis_I_spec.html#x1-810004.3.9).
//!A decoder may wish to apply an additional permutation to the mapping the encoder used to achieve a different output channel order (e.g. for outputting in WAV order).
//!
//!Each multistream packet contains an Opus packet for each stream, and all of the Opus packets in
//!a single multistream packet must have the same duration. Therefore the duration of a multistream
//!packet can be extracted from the TOC sequence of the first stream, which is located at the
//!beginning of the packet.

mod encoder;
pub use encoder::Encoder;
mod decoder;
pub use decoder::Decoder;

///Multistream configuration
///
///## Parameters
///
///`CH` - Number of channels to use. Up to 255.
pub struct Config<const CH: usize> {
    streams: u8,
    coupled_streams: u8,
    mapping: [u8; CH],
}

impl<const CH: usize> Config<CH> {
    const CHANNELS: u8 = match CH {
        0 => panic!("Unsupported number of channels. Allowed range: 0..=255"),
        _ => CH as _,
    };

    ///Creates new config, verifying that specified number of channels is valid and do not exceed `CH`
    ///
    ///Following constraints are imposed:
    ///
    ///- `streams` cannot be 0 or exceed `CH`
    ///- `coupled_streams` cannot exceed `streams`
    ///- Sum of both cannot exceed `CH`
    pub const fn try_new(streams: u8, coupled_streams: u8, mapping: [u8; CH]) -> Option<Self> {
        if streams == 0 || coupled_streams > streams {
            return None;
        }

        let mut idx = 0;
        while idx < CH {
            if mapping[idx] != 255 && (mapping[idx] as usize) >= CH {
                return None;
            }
            idx = idx.saturating_add(1);
        }

        match streams.checked_add(coupled_streams) {
            Some(total) => if total == 0 || total > Self::CHANNELS {
                None
            } else {
                Some(Self {
                    streams,
                    coupled_streams,
                    mapping,
                })
            }
            None => None,
        }
    }

    ///Creates new config, verifying that specified number of channels is valid and do not exceed `CH`
    ///
    ///Following constraints are imposed and will cause panic:
    ///
    ///- `streams` cannot be 0 or exceed `CH`
    ///- `coupled_streams` cannot exceed `streams`
    ///- Sum of both cannot exceed `CH`
    pub const fn new(streams: u8, coupled_streams: u8, mapping: [u8; CH]) -> Self {
        assert!(streams != 0);
        assert!(coupled_streams <= streams);

        let mut idx = 0;
        while idx < CH {
            if mapping[idx] != 255 {
                assert!((mapping[idx] as usize) < CH, "Non 255 mapping values must be in range 0..CH");
            }
            idx = idx.saturating_add(1);
        }

        match streams.checked_add(coupled_streams) {
            Some(total_streams) => {
                assert!(total_streams != 0);
                assert!(total_streams <= Self::CHANNELS);

                Self {
                    streams,
                    coupled_streams,
                    mapping
                }
            },
            None => panic!("sum(streams, coupled_streams) cannot exceed 255"),
        }
    }

    #[inline(always)]
    ///Accesses mapping
    pub fn mapping(&self) -> &[u8; CH] {
        &self.mapping
    }

    #[inline(always)]
    ///Accesses mapping
    pub fn mapping_mut(&mut self) -> &mut [u8; CH] {
        &mut self.mapping
    }
}
