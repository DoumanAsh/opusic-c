use opusic_c::{Encoder, SampleRate, Channels, Application, Bandwidth, Bitrate, Signal, InbandFec, FrameDuration};
use opusic_c::{ErrorCode, frame_bytes_size, version};

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
}

#[test]
fn should_verify_encoder_encoding_mono() {
    let mut encoder = Encoder::<{Channels::Mono as _}>::new(SampleRate::Hz48000, Application::Audio).expect("Create");

    const SIZE_20MS: usize = frame_bytes_size(SampleRate::Hz48000, Channels::Mono, 20);
    let input = [0; SIZE_20MS];
    let mut output = [0; 256];

    let len = encoder.encode_to_slice(&input, &mut output).expect("to encode");
    assert_eq!(&output[..len], &[248, 255, 254]);
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
}
