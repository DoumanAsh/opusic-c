use opusic_c::{multistream, repacketizer, Encoder, Decoder};
use opusic_c::{ErrorCode, frame_bytes_size, version};
use opusic_c::{SampleRate, Channels, Application, Bandwidth, Bitrate, Signal, InbandFec, FrameDuration};

#[cfg(feature = "dred")]
#[test]
fn should_verify_dred_packet_size() {
    use opusic_c::dred::{DRED_PACKET_SIZE, dred_packet_size};
    assert_eq!(DRED_PACKET_SIZE, dred_packet_size());
}

#[test]
fn should_verify_frame_size_utils() {
    assert_eq!(frame_bytes_size(SampleRate::Hz48000, Channels::Mono, 120), 5760);
    assert_eq!(frame_bytes_size(SampleRate::Hz48000, Channels::Stereo, 120), 11520);
}
#[test]
fn should_assert_crate_version() {
    assert_eq!(version(), "libopus 1.5.2");
}

#[test]
fn should_verify_encoder_encoding_stereo() {
    let mut encoder = Encoder::<{Channels::Stereo as _}>::new(SampleRate::Hz48000, Application::Audio).expect("Create");

    const SIZE_20MS: usize = frame_bytes_size(SampleRate::Hz48000, Channels::Stereo, 20);
    let input = [0; SIZE_20MS];
    let mut output = [0; 256];

    let len = encoder.encode_to_slice(&input, &mut output).expect("to encode");
    assert_eq!(&output[..len], &[252, 255, 254]);

    let mut decoder = Decoder::<{Channels::Stereo as _}>::new(SampleRate::Hz48000).expect("Create");
    let mut decoded = [0; SIZE_20MS];

    let decoded_len = decoder.decode_to_slice(&output[..len], &mut decoded, false).expect("to decode");
    assert_eq!(decoded_len, SIZE_20MS / 2);

    let mut vec_output = Vec::with_capacity(256);
    encoder.reset().expect("reset");
    encoder.encode_to_vec(&input, &mut vec_output).expect("to encode");
    assert_eq!(vec_output, &[252, 255, 254]);

    let mut vec_decoded = Vec::with_capacity(SIZE_20MS);
    decoder.reset().expect("to reset");
    let decoded_len = decoder.decode_to_vec(&vec_output, &mut vec_decoded, SIZE_20MS, false).expect("to decode");
    assert_eq!(decoded_len, vec_decoded.len());
    assert_eq!(vec_decoded, decoded[..SIZE_20MS / 2]);

    encoder.reset().expect("reset");
    encoder.encode_to_vec(&input, &mut vec_output).expect("to encode");
    assert_eq!(vec_output, &[252, 255, 254, 252, 255, 254]);

    #[cfg(feature = "dred")]
    {
        encoder.reset().expect("reset");
        encoder.set_dred_duration(10).expect("enable DRED");
        let len = encoder.encode_to_slice(&input, &mut output).expect("to encode");
        assert_eq!(&output[..len], &[252, 255, 254]);

        let mut decoded_dred = [1; SIZE_20MS];
        let mut dred = opusic_c::dred::Dred::new(decoder).expect("create DRED decoder");
        let len = dred.decode_to_slice(&output[..len], &mut decoded_dred).expect("to decode");
        assert_eq!(len, SIZE_20MS / 2);
        assert_eq!(decoded, decoded_dred);
    }
}

#[test]
fn should_verify_encoder_encoding_mono() {
    let mut encoder = Encoder::<{Channels::Mono as _}>::new(SampleRate::Hz48000, Application::Audio).expect("Create");

    const SIZE_20MS: usize = frame_bytes_size(SampleRate::Hz48000, Channels::Mono, 20);
    let input = [0; SIZE_20MS];
    let mut output = [0; 256];

    let len = encoder.encode_to_slice(&input, &mut output).expect("to encode");
    assert_eq!(&output[..len], &[248, 255, 254]);

    let mut decoder = Decoder::<{Channels::Mono as _}>::new(SampleRate::Hz48000).expect("Create");
    let mut decoded = [0; SIZE_20MS];

    let len = decoder.decode_to_slice(&output[..len], &mut decoded, false).expect("to decode");
    assert_eq!(len, SIZE_20MS);
    assert_eq!(decoded, input);

    decoder.reset().expect("reset");
}

#[test]
fn should_verify_encoder_building() {
    let mut encoder = Encoder::<{Channels::Stereo as _}>::new(SampleRate::Hz48000, Application::Audio).expect("Create");
    let value = encoder.get_sample_rate().expect("get sample rate");
    assert_eq!(value, SampleRate::Hz48000);

    assert!(!encoder.get_phase_inversion_disabled().expect("get phase inversion status"), "Phase inversion is ON by default");
    encoder.set_phase_inversion_disabled(true).expect("update phase inversion");
    assert!(encoder.get_phase_inversion_disabled().expect("get phase inversion status"), "Phase inversion is set to OFF");

    assert!(!encoder.get_dtx().expect("get dtx"));

    encoder.set_dtx(true).expect("update dtx");
    assert!(encoder.get_dtx().expect("get dtx"));

    encoder.set_dtx(false).expect("update dtx");
    assert!(!encoder.get_dtx().expect("get dtx"));

    encoder.set_lsb_depth(16).expect("update LSB depth");
    let value = encoder.get_lsb_depth().expect("get LSB depth");
    assert_eq!(value, 16);

    let value = encoder.get_bandwidth().expect("get default bandwidth");
    let max_value = encoder.get_max_bandwidth().expect("get default bandwidth");
    assert_eq!(value, max_value);

    encoder.set_bandwidth(Bandwidth::Narrow).expect("set bandwidth");
    encoder.set_max_bandwidth(Bandwidth::Superwide).expect("set bandwidth");

    let value = encoder.get_bandwidth().expect("get default bandwidth");
    let max_value = encoder.get_max_bandwidth().expect("get default bandwidth");
    //User set bandwidth takes effect after encoding pass
    assert_eq!(value, Bandwidth::Full);
    assert_eq!(max_value, Bandwidth::Superwide);

    encoder.set_complexity(5).expect("set complexity");
    let value = encoder.get_complexity().expect("set complexity");
    assert_eq!(value, 5);

    let value = encoder.get_bitrate().expect("get_bitrate");
    //Default derived from channels and sample rate
    assert_eq!(value, Bitrate::Value(120000));

    encoder.set_bitrate(Bitrate::Auto).expect("set bitrate");
    let value = encoder.get_bitrate().expect("get_bitrate");
    assert_eq!(value, Bitrate::Value(120000));

    encoder.set_bitrate(Bitrate::Max).expect("set bitrate");
    let value = encoder.get_bitrate().expect("get_bitrate");
    assert_eq!(value, Bitrate::Value(4083200));

    let value = encoder.get_vbr().expect("get default VBR");
    assert!(value, "Default VBR is ON");
    let value = encoder.get_vbr_constraint().expect("get default VBR constraint");
    assert!(value, "Default VBR constraint is ON");

    encoder.set_vbr(false).expect("set non-default VBR");
    let value = encoder.get_vbr().expect("get non-default VBR");
    assert!(!value, "VBR is OFF");

    encoder.set_vbr_constraint(false).expect("set non-default VBR constraint");
    let value = encoder.get_vbr_constraint().expect("get non-default VBR constraint");
    assert!(!value, "VBR constraint is OFF");

    let value = encoder.get_force_channels().expect("get default force channels");
    assert!(value.is_none(), "Force channels should be OFF by default");

    encoder.set_force_channels(Some(Channels::Stereo)).expect("set force channels to stereo");
    let value = encoder.get_force_channels().expect("get default force channels");
    assert_eq!(value, Some(Channels::Stereo), "Force channels is set to Stereo");

    encoder.set_force_channels(None).expect("reset force channels");
    let value = encoder.get_force_channels().expect("get default force channels");
    assert_eq!(value, None, "Force channels is reset");

    let value = encoder.get_signal().expect("get default signal value");
    assert_eq!(value, Signal::Auto, "Signal is default Auto");

    encoder.set_signal(Signal::Voice).expect("set signal value");
    let value = encoder.get_signal().expect("get signal value");
    assert_eq!(value, Signal::Voice, "Should be Voice");

    encoder.set_signal(Signal::Music).expect("set signal value");
    let value = encoder.get_signal().expect("get signal value");
    assert_eq!(value, Signal::Music, "Should be Music");

    let value = encoder.get_application().expect("get default application value");
    assert_eq!(value, Application::Audio, "Application was set to Audio during creation");

    encoder.set_application(Application::Voip).expect("set voip application");
    let value = encoder.get_application().expect("get default application value");
    assert_eq!(value, Application::Voip, "Application was set to Voip");

    let value = encoder.get_look_ahead().expect("get look ahead value");
    assert_eq!(value, 312, "Look ahead depends on application");

    let value = encoder.get_inband_fec().expect("get default Inband FEC");
    assert_eq!(value, InbandFec::Off, "Default Inband FEC is OFF");
    encoder.set_inband_fec(InbandFec::Mode1).expect("update InbandFec depth");
    let value = encoder.get_inband_fec().expect("get default Inband FEC");
    assert_eq!(value, InbandFec::Mode1);

    let value = encoder.get_packet_loss().expect("get default Packet loss");
    assert_eq!(value, 0, "Default packet loss value should be 0%");
    for value in 1..=100 {
        encoder.set_packet_loss(value).expect("set packet loss");
        let result = encoder.get_packet_loss().expect("get packet loss");
        assert_eq!(result, value);
    }

    let error = encoder.set_packet_loss(101).expect_err("should not be able to set 101%");
    assert_eq!(error, ErrorCode::BadArg);

    let value = encoder.get_prediction_disabled().expect("get default Prediction status");
    assert!(!value, "Prediction should be enabled by default");

    encoder.set_prediction_disabled(true).expect("to set");
    let value = encoder.get_prediction_disabled().expect("get Prediction status");
    assert!(value, "Prediction should be disabled by default");

    let value = encoder.get_frame_duration().expect("get default frame duration");
    assert_eq!(value, FrameDuration::SizeArg);

    encoder.set_frame_duration(FrameDuration::Size10).expect("set frame duration");
    let value = encoder.get_frame_duration().expect("get frame duration");
    assert_eq!(value, FrameDuration::Size10);

    encoder.reset().expect("To reset");

    #[cfg(feature = "dred")]
    {
        let value = encoder.get_dred_duration().expect("get default DRED");
        assert_eq!(value, 0);
        encoder.set_dred_duration(4).expect("set DRED");
        let value = encoder.get_dred_duration().expect("set DRED duration");
        assert_eq!(value, 4);
    }
}

#[test]
fn should_verify_decoder_building() {
    let mut decoder = Decoder::<{Channels::Stereo as _}>::new(SampleRate::Hz48000).expect("Create");
    let value = decoder.get_sample_rate().expect("get sample rate");
    assert_eq!(value, SampleRate::Hz48000);

    let value = decoder.get_pitch().expect("get default pitch");
    assert_eq!(value, None);

    let value = decoder.get_last_packet_duration().expect("get default last packet duration");
    assert_eq!(value, 0);

    let value = decoder.get_bandwidth().expect("get default bandwidth");
    assert_eq!(value, Bandwidth::Auto);

    assert!(!decoder.get_phase_inversion_disabled().expect("get phase inversion status"), "Phase inversion is ON by default");
    decoder.set_phase_inversion_disabled(true).expect("update phase inversion");
    assert!(decoder.get_phase_inversion_disabled().expect("get phase inversion status"), "Phase inversion is set to OFF");

    let value = decoder.get_gain().expect("get default gain");
    assert_eq!(value, 0);

    for value in -32768..=32767 {
        decoder.set_gain(value).expect("set gain");
        let result = decoder.get_gain().expect("get gain");
        assert_eq!(result, value);
    }
}

#[test]
fn should_fail_to_repacketizer() {
    let mut packet = [0u8; 1277];

    let mut repacketizer = repacketizer::Repacketizer::new().expect("create repacketizer");

    assert_eq!(repacketizer.get_nb_frames(), 0);

    //no buffer = fail
    let mut error = repacketizer.add_packet(&[]).expect_err("should fail empty packet");
    assert_eq!(error, ErrorCode::InvalidPacket);

    //packet with zero len but invalid actual size
    error = repacketizer.add_packet(&packet).expect_err("should fail empty packet");
    assert_eq!(error, ErrorCode::InvalidPacket);

    //odd op code
    packet[0] = 1;
    error = repacketizer.add_packet(&packet[..2]).expect_err("should fail empty packet");
    assert_eq!(error, ErrorCode::InvalidPacket);

    //overflow
    packet[0] = 2;
    error = repacketizer.add_packet(&packet[..1]).expect_err("should fail empty packet");
    assert_eq!(error, ErrorCode::InvalidPacket);

    //no count
    packet[0] = 3;
    error = repacketizer.add_packet(&packet[..1]).expect_err("should fail empty packet");
    assert_eq!(error, ErrorCode::InvalidPacket);

    //ok empty packet
    packet[0] = 0;
    let packet_holder1 = repacketizer.add_packet(&packet[..3]).expect("should successfully add packet with zero len");
    assert_eq!(packet_holder1.0.len(), 3);

    //TOC change error detected
    packet[0] = 1 << 2;
    error = repacketizer.add_packet(&packet[..3]).expect_err("should fail empty packet");
    assert_eq!(error, ErrorCode::InvalidPacket);

    repacketizer.reset();
    //Reset allows new TOC
    let packet_holder1 = repacketizer.add_packet(&packet[..3]).expect("should successfully add packet with zero len");
    assert_eq!(packet_holder1.0.len(), 3);
}

#[test]
fn should_verify_multistream_encoder_building() {
    let config = multistream::Config::<2>::new(2, 0, [0, 1]);
    let mut encoder = multistream::Encoder::new(config, SampleRate::Hz48000, Application::Audio).expect("create new encoder");
    let value = encoder.get_sample_rate().expect("get sample rate");
    assert_eq!(value, SampleRate::Hz48000);

    assert!(!encoder.get_phase_inversion_disabled().expect("get phase inversion status"), "Phase inversion is ON by default");
    encoder.set_phase_inversion_disabled(true).expect("update phase inversion");
    assert!(encoder.get_phase_inversion_disabled().expect("get phase inversion status"), "Phase inversion is set to OFF");

    assert!(!encoder.get_dtx().expect("get dtx"));

    encoder.set_dtx(true).expect("update dtx");
    assert!(encoder.get_dtx().expect("get dtx"));

    encoder.set_dtx(false).expect("update dtx");
    assert!(!encoder.get_dtx().expect("get dtx"));

    encoder.set_lsb_depth(16).expect("update LSB depth");
    let value = encoder.get_lsb_depth().expect("get LSB depth");
    assert_eq!(value, 16);

    let value = encoder.get_bandwidth().expect("get default bandwidth");
    assert_eq!(value, Bandwidth::Full);

    encoder.set_bandwidth(Bandwidth::Narrow).expect("set bandwidth");
    encoder.set_max_bandwidth(Bandwidth::Superwide).expect("set bandwidth");

    let value = encoder.get_bandwidth().expect("get default bandwidth");
    //User set bandwidth takes effect after encoding pass
    assert_eq!(value, Bandwidth::Full);

    encoder.set_complexity(5).expect("set complexity");
    let value = encoder.get_complexity().expect("set complexity");
    assert_eq!(value, 5);

    let value = encoder.get_bitrate().expect("get_bitrate");
    //Default derived from channels and sample rate
    assert_eq!(value, Bitrate::Value(144000));

    encoder.set_bitrate(Bitrate::Auto).expect("set bitrate");
    let value = encoder.get_bitrate().expect("get_bitrate");
    assert_eq!(value, Bitrate::Value(144000));

    encoder.set_bitrate(Bitrate::Max).expect("set bitrate");
    let value = encoder.get_bitrate().expect("get_bitrate");
    //set_bitrate on multistream doesn't set bitrate immediately on underlying encoders
    assert_eq!(value, Bitrate::Value(144000));

    let value = encoder.get_vbr().expect("get default VBR");
    assert!(value, "Default VBR is ON");
    let value = encoder.get_vbr_constraint().expect("get default VBR constraint");
    assert!(value, "Default VBR constraint is ON");

    encoder.set_vbr(false).expect("set non-default VBR");
    let value = encoder.get_vbr().expect("get non-default VBR");
    assert!(!value, "VBR is OFF");

    encoder.set_vbr_constraint(false).expect("set non-default VBR constraint");
    let value = encoder.get_vbr_constraint().expect("get non-default VBR constraint");
    assert!(!value, "VBR constraint is OFF");

    let value = encoder.get_signal().expect("get default signal value");
    assert_eq!(value, Signal::Auto, "Signal is default Auto");

    encoder.set_signal(Signal::Voice).expect("set signal value");
    let value = encoder.get_signal().expect("get signal value");
    assert_eq!(value, Signal::Voice, "Should be Voice");

    encoder.set_signal(Signal::Music).expect("set signal value");
    let value = encoder.get_signal().expect("get signal value");
    assert_eq!(value, Signal::Music, "Should be Music");

    let value = encoder.get_application().expect("get default application value");
    assert_eq!(value, Application::Audio, "Application was set to Audio during creation");

    encoder.set_application(Application::Voip).expect("set voip application");
    let value = encoder.get_application().expect("get default application value");
    assert_eq!(value, Application::Voip, "Application was set to Voip");

    let value = encoder.get_look_ahead().expect("get look ahead value");
    assert_eq!(value, 312, "Look ahead depends on application");

    let value = encoder.get_inband_fec().expect("get default Inband FEC");
    assert_eq!(value, InbandFec::Off, "Default Inband FEC is OFF");
    encoder.set_inband_fec(InbandFec::Mode1).expect("update InbandFec depth");
    let value = encoder.get_inband_fec().expect("get default Inband FEC");
    assert_eq!(value, InbandFec::Mode1);

    let value = encoder.get_packet_loss().expect("get default Packet loss");
    assert_eq!(value, 0, "Default packet loss value should be 0%");
    for value in 1..=100 {
        encoder.set_packet_loss(value).expect("set packet loss");
        let result = encoder.get_packet_loss().expect("get packet loss");
        assert_eq!(result, value);
    }

    let error = encoder.set_packet_loss(101).expect_err("should not be able to set 101%");
    assert_eq!(error, ErrorCode::BadArg);

    let value = encoder.get_prediction_disabled().expect("get default Prediction status");
    assert!(!value, "Prediction should be enabled by default");

    encoder.set_prediction_disabled(true).expect("to set");
    let value = encoder.get_prediction_disabled().expect("get Prediction status");
    assert!(value, "Prediction should be disabled by default");

    let value = encoder.get_frame_duration().expect("get default frame duration");
    assert_eq!(value, FrameDuration::SizeArg);

    encoder.set_frame_duration(FrameDuration::Size10).expect("set frame duration");
    let value = encoder.get_frame_duration().expect("get frame duration");
    assert_eq!(value, FrameDuration::Size10);

    encoder.reset().expect("To reset");
}

#[test]
fn should_verify_multistream_decoder_building() {
    let config = multistream::Config::<2>::new(2, 0, [0, 1]);
    let mut decoder = multistream::Decoder::new(config, SampleRate::Hz48000).expect("create new encoder");
    let value = decoder.get_sample_rate().expect("get sample rate");
    assert_eq!(value, SampleRate::Hz48000);

    let value = decoder.get_last_packet_duration().expect("get default last packet duration");
    assert_eq!(value, 0);

    let value = decoder.get_bandwidth().expect("get default bandwidth");
    assert_eq!(value, Bandwidth::Auto);

    assert!(decoder.get_phase_inversion_disabled().expect("get phase inversion status"), "Phase inversion is OFF by default");
    decoder.set_phase_inversion_disabled(false).expect("update phase inversion");
    assert!(!decoder.get_phase_inversion_disabled().expect("get phase inversion status"), "Phase inversion is set to ON");

    let value = decoder.get_gain().expect("get default gain");
    assert_eq!(value, 0);

    for value in -32768..=32767 {
        decoder.set_gain(value).expect("set gain");
        let result = decoder.get_gain().expect("get gain");
        assert_eq!(result, value);
    }
}
