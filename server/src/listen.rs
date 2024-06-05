#[allow(dead_code)]
mod aptx;
#[allow(dead_code)]
mod utils;

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
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use log::{debug, error, info};
    use std::{
        net::UdpSocket,
        sync::{Arc, RwLock},
    };

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
    let config = cpal::StreamConfig {
        channels: 2,
        sample_rate: cpal::SampleRate(48000),
        buffer_size: cpal::BufferSize::Fixed(512),
    };
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("No input device available");
    let sock_buffer_size = args.with_aptx.then(|| 512).unwrap_or(2048);
    let sock_buffer = Arc::new(RwLock::new(Vec::new()));
    let sock_buffer_clone = sock_buffer.clone();
    std::thread::spawn(move || {
        let mut local_buffer = Vec::new();
        local_buffer.resize(sock_buffer_size, 0);
        loop {
            let _ = sock_audio.recv_from(&mut local_buffer);
            if args.with_aptx {
                let mut dec_buffer = [0; 2048];
                let mut written = 0;
                let mut dropped = 0;
                let mut synced = false;
                let processed = aptx_ctx.decode_sync(
                    &local_buffer,
                    &mut dec_buffer,
                    &mut written,
                    &mut synced,
                    &mut dropped,
                );
                if !synced || dropped > 0 {
                    error!("aptX decoding failed, synchronizing {written} {synced} {dropped}");
                }
                if processed != local_buffer.len() {
                    panic!("aptX decoding failed {written} != {}", dec_buffer.len());
                }
                sock_buffer_clone.write().unwrap().extend(&dec_buffer);
            } else {
                sock_buffer_clone.write().unwrap().extend(&local_buffer);
            }
        }
    });
    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                if sock_buffer.read().unwrap().len() < data.len() * 2 {
                    return;
                }
                let buffer: Vec<u8> = {
                    sock_buffer
                        .write()
                        .unwrap()
                        .drain(..(data.len() * std::mem::size_of::<i16>()))
                        .collect()
                };
                let bslice = unsafe { utils::into_slice(&buffer) };
                debug!("{} {}", data.len(), bslice.len());
                data.copy_from_slice(&bslice);
            },
            move |err| {
                error!("{}", err);
            },
            None,
        )
        .unwrap();
    stream.play().unwrap();
    loop {
        std::hint::spin_loop();
    }
}

#[cfg(target_os = "android")]
fn main() {}
