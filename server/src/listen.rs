#[allow(dead_code)]
mod aptx;

#[cfg(not(target_os = "android"))]
#[derive(Debug, clap::Parser)]
struct Args {
    #[arg(short, long)]
    with_aptx: bool,

    #[arg(long)]
    hd: bool,
}

#[cfg(not(target_os = "android"))]
fn main() {
    use aptx::AptxContext;
    use clap::Parser;
    use libpulse_binding::{def::BufferAttr, sample, stream::Direction};
    use libpulse_simple_binding::Simple;
    use log::{error, info};
    use std::net::UdpSocket;

    env_logger::init();
    let args = Args::parse();
    args.hd.then(|| info!("HD enabled"));
    args.with_aptx.then(|| info!("APTX enabled"));

    let sock_addr = UdpSocket::bind("127.0.0.1:0").unwrap();
    sock_addr
        .send_to(b"OK\0", "127.0.0.1:4052")
        .expect("Fail to connect to server");
    let sock_audio = UdpSocket::bind("127.0.0.1:4051").unwrap();

    let mut aptx_ctx = AptxContext::new(args.hd);
    let format = if args.hd {
        sample::Format::S24le
    } else {
        sample::Format::S16le
    };
    let audio_spec = sample::Spec {
        format,
        rate: 48000,
        channels: 2,
    };
    let attr = BufferAttr {
        maxlength: 65536,
        tlength: 2048,
        prebuf: 512,
        minreq: 512,
        fragsize: 2048,
    };
    let pulse_cnn = Simple::new(
        None,
        "pc_relay",
        Direction::Playback,
        None,
        "System sound",
        &audio_spec,
        None,
        Some(&attr),
    )
    .expect("Fail to connect to the audio server");
    let mut buffer = [0; 512];
    let mut out_buffer = [0; 2048];
    let mut written = 0;
    let mut dropped = 0;
    let mut synced = false;
    loop {
        let _ = sock_audio.recv_from(&mut buffer);
        if args.with_aptx {
            let processed = aptx_ctx.decode_sync(
                &buffer,
                &mut out_buffer,
                &mut written,
                &mut synced,
                &mut dropped,
            );
            match pulse_cnn.write(&out_buffer) {
                Ok(_) => {}
                Err(_) => error!("Fail to write audio"),
            }
            if !synced || dropped > 0 {
                error!("aptX decoding failed, synchronizing {written} {synced} {dropped}");
            }
            if processed != buffer.len() {
                error!("aptX decoding failed {written} != {}", out_buffer.len());
                break;
            }
        } else {
            let _ = pulse_cnn.write(&buffer);
        }
    }
}

#[cfg(target_os = "android")]
fn main() {}
