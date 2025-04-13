// SPDX-License-Identifier: EUPL-1.2

use std::{
    ffi::CString,
    io::{BufReader, Read},
    os::unix::net::UnixStream,
};

use anyhow::{bail, Context};
use lockfree::channel::spsc;
use pulseaudio::protocol;
use simple_moving_average::{SumTreeSMA, SMA};
use soundtouch::BPMDetect;
// TODO Enable FFT analysis when running in "real-time" (double check performance usage)
// use spectrum_analyzer::scaling::scale_to_zero_to_one;
// use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit};

#[derive(Debug)]
pub enum Event {
    Tempo { average: f32, accuracy: f32 },
    Volume { average: f32 },
    Reset,
}

const SILENCE_RMS: f32 = 0.01;
const SILENCE_TIME: f32 = 0.618; // Number of seconds of silence to consider reset

pub fn run(event_tx: &mut spsc::Sender<Event>) -> anyhow::Result<()> {
    let (mut sock, protocol_version) = connect_and_init().context("failed to initialize client")?;

    protocol::write_command_message(
        sock.get_mut(),
        10,
        protocol::Command::GetSourceInfo(protocol::GetSourceInfo {
            // TODO Make this configurable
            // name: Some(CString::new("@DEFAULT_SOURCE@")?),
            name: Some(CString::new("alsa_output.usb-Focusrite_Scarlett_4i4_USB_D8JXNH21A2049A-00.analog-surround-40.monitor")?),
            ..Default::default()
        }),
        protocol_version,
    )?;

    let (_, source_info) =
        protocol::read_reply_message::<protocol::SourceInfo>(&mut sock, protocol_version)?;
    eprintln!(
        "recording from source: {:?}...",
        source_info.description.unwrap_or(source_info.name)
    );

    // TODO Support stereo input? or optionally merge stereo signals
    let channels = 1; // source_info.channel_map.num_channels()

    // Create the recording stream on the server.
    protocol::write_command_message(
        sock.get_mut(),
        99,
        protocol::Command::CreateRecordStream(protocol::RecordStreamParams {
            source_index: Some(source_info.index),
            sample_spec: protocol::SampleSpec {
                format: source_info.sample_spec.format,
                channels: channels,
                sample_rate: source_info.sample_spec.sample_rate,
            },
            channel_map: source_info.channel_map,
            cvolume: Some(protocol::ChannelVolume::norm(2)),
            ..Default::default()
        }),
        protocol_version,
    )?;

    let (_, record_stream) = protocol::read_reply_message::<protocol::CreateRecordStreamReply>(
        &mut sock,
        protocol_version,
    )?;

    // Create the output file.
    // TODO Support different sample formats
    let (_bits_per_sample, _sample_format) = match record_stream.sample_spec.format {
        protocol::SampleFormat::S16Le => (16, hound::SampleFormat::Int),
        protocol::SampleFormat::Float32Le => (32, hound::SampleFormat::Float),
        protocol::SampleFormat::S32Le => (32, hound::SampleFormat::Int),
        _ => bail!(
            "unsupported sample format: {:?}",
            record_stream.sample_spec.format
        ),
    };

    let frame_size = 16384;
    let frame_time = frame_size as f32 / record_stream.sample_spec.sample_rate as f32;
    eprintln!("frame_time: {:#?}", frame_time);

    eprintln!("stream: {:#?}", record_stream);

    // A reusable buffer.
    let mut buf = vec![0; record_stream.buffer_attr.fragment_size as usize];

    let mut bpm_detect = BPMDetect::new(1, record_stream.sample_spec.sample_rate);

    let mut bpm_sma = SumTreeSMA::<_, f32, 128>::new();
    let mut rms_sma = SumTreeSMA::<_, f32, 512>::new();
    let mut silence: usize = 0;
    let silence_reset: usize =
        (SILENCE_TIME * record_stream.sample_spec.sample_rate as f32).floor() as usize;

    // Read messages from the server in a loop. In real code it would be more
    // efficient to poll the socket using `mio` or similar.
    loop {
        let desc = protocol::read_descriptor(&mut sock)?;

        // A channel of -1 is a command message. Everything else is data.
        if desc.channel == u32::MAX {
            let (_, msg) = protocol::Command::read_tag_prefixed(&mut sock, protocol_version)?;
            eprintln!("received command from server: {:#?}", msg);
        } else {
            buf.resize(desc.length as usize, 0);
            sock.read_exact(&mut buf)?;

            let frame: Vec<f32> = buf
                .chunks_exact(4)
                .map(TryInto::try_into)
                .map(Result::unwrap)
                .map(i32::from_le_bytes)
                .map(|x| x as f32 / i32::MAX as f32)
                .collect();

            debug_assert!(frame_size == frame.len());

            if silence > silence_reset {
                rms_sma = SumTreeSMA::<_, f32, 512>::new();
            }

            for block in frame.chunks_exact(frame_size / 256) {
                bpm_detect.input_samples(&block);
                let rms =
                    (block.iter().map(|x| x.powf(2.0)).sum::<f32>() / block.len() as f32).sqrt();
                rms_sma.add_sample(rms);
            }

            let rms = rms_sma.get_average();

            if rms > SILENCE_RMS {
                silence = 0;

                event_tx
                    .send(Event::Volume {
                        average: if rms < 0.2 { rms / 0.2 } else { 1.0 },
                    })
                    .expect("Can not send audio event");

                let bpm_frame = bpm_detect.get_bpm();
                if bpm_frame != 0.0 {
                    let bpm_select = if bpm_sma.get_num_samples() > 0 {
                        let average: f32 = bpm_sma.get_average();
                        let mut select = bpm_frame;
                        let mut select_delta = 300.0;
                        for b in vec![bpm_frame, bpm_frame * 2.0, bpm_frame / 2.] {
                            let delta = (b - average).abs();
                            if delta < select_delta {
                                select_delta = delta;
                                select = b;
                            }
                        }
                        select
                    } else {
                        bpm_frame
                    };

                    bpm_sma.add_sample(bpm_select);

                    let average = bpm_sma.get_average();
                    let mut accuracy = bpm_sma.get_num_samples() as f32 / 4.0;
                    if accuracy > 1.0 {
                        accuracy = 1.0;
                    }
                    event_tx
                        .send(Event::Tempo { average, accuracy })
                        .expect("Can not send audio event");
                }
            } else {
                if silence < usize::MAX - frame_size && silence < silence_reset {
                    silence += frame_size;
                    if silence >= silence_reset {
                        bpm_detect = BPMDetect::new(1, record_stream.sample_spec.sample_rate);
                        bpm_sma = SumTreeSMA::<_, f32, 128>::new();
                        event_tx
                            .send(Event::Reset)
                            .expect("Can not send audio event");
                    }
                }
            }
        }
    }
}

fn connect_and_init() -> anyhow::Result<(BufReader<UnixStream>, u16)> {
    let socket_path = pulseaudio::socket_path_from_env().context("PulseAudio not available")?;
    let mut sock = std::io::BufReader::new(UnixStream::connect(socket_path)?);

    let cookie = pulseaudio::cookie_path_from_env()
        .and_then(|path| std::fs::read(path).ok())
        .unwrap_or_default();
    let auth = protocol::AuthParams {
        version: protocol::MAX_VERSION,
        supports_shm: false,
        supports_memfd: false,
        cookie,
    };

    protocol::write_command_message(
        sock.get_mut(),
        0,
        protocol::Command::Auth(auth),
        protocol::MAX_VERSION,
    )?;

    let (_, auth_reply) =
        protocol::read_reply_message::<protocol::AuthReply>(&mut sock, protocol::MAX_VERSION)?;
    let protocol_version = std::cmp::min(protocol::MAX_VERSION, auth_reply.version);

    let mut props = protocol::Props::new();
    props.set(
        protocol::Prop::ApplicationName,
        CString::new("pulseaudio-rs-playback").unwrap(),
    );
    protocol::write_command_message(
        sock.get_mut(),
        1,
        protocol::Command::SetClientName(props),
        protocol_version,
    )?;

    let _ =
        protocol::read_reply_message::<protocol::SetClientNameReply>(&mut sock, protocol_version)?;
    Ok((sock, protocol_version))
}
