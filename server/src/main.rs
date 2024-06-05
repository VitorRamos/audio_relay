#[allow(dead_code)]
mod aptx;
#[cfg(not(target_os = "android"))]
mod utils;

#[cfg(not(target_os = "android"))]
#[derive(Debug, clap::Parser)]
struct Args {
    #[arg(short, long)]
    with_aptx: bool,

    #[arg(long)]
    hd: bool,

    #[arg(short, long, default_value_t = String::from("0.0.0.0"))]
    addr: String,

    #[arg(long, default_value_t = 4051)]
    port_audio: u16,

    #[arg(long, default_value_t = 4052)]
    port_addr: u16,

    #[arg(long, default_value_t = 4053)]
    port_cmds: u16,
}

#[cfg(not(target_os = "android"))]
fn main() {
    use crate::aptx::AptxContext;
    use clap::Parser;
    use cpal::traits::{DeviceTrait, StreamTrait};
    use log::{debug, error, info};
    use std::{
        net::{Ipv4Addr, SocketAddr, UdpSocket},
        sync::{Arc, Mutex},
    };
    unsafe { std::env::set_var("RUST_LOG", "info") };
    env_logger::init();
    let args = Args::parse();
    args.with_aptx.then(|| info!("APTX enabled"));
    args.hd.then(|| info!("HD enabled"));
    let client_addr = Arc::new(Mutex::new(SocketAddr::new(
        Ipv4Addr::new(192, 168, 0, 13).into(),
        args.port_audio,
    )));
    let client_addr_clone = client_addr.clone();
    std::thread::scope(|s| {
        s.spawn(|| {
            utils::udp_server_loop_data::<12>(&args.addr, args.port_addr, |_, mut client| {
                client.set_port(args.port_audio);
                *client_addr_clone.lock().unwrap() = client;
                info!("New client {client_addr_clone:?}");
            })
        });
        s.spawn(|| {
            utils::udp_server_loop_data::<12>(&args.addr, args.port_cmds, |data, _| {
                if data.trim().contains("NEXT") {
                    debug!("PlayerControl::Next");
                    utils::media_control(utils::PlayerControl::Next);
                } else if data.trim().contains("PREV") {
                    debug!("PlayerControl::Previous");
                    utils::media_control(utils::PlayerControl::Previous);
                }
            });
        });
        s.spawn(|| {
            let socket = UdpSocket::bind("0.0.0.0:0").expect("Error creating client");
            let config = cpal::StreamConfig {
                channels: 2,
                sample_rate: cpal::SampleRate(48000),
                buffer_size: cpal::BufferSize::Fixed(128),
            };
            let device = utils::get_source_by_name("default").expect("Fail to get device");
            info!("Output: {}", device.name().unwrap());
            let mut ctx = AptxContext::new(args.hd);
            let mut enc_buffer = [0u8; 512];
            let mut cbuffer = Vec::with_capacity(4096);
            let stream = device
                .build_input_stream(
                    &config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let u8slice = unsafe { utils::into_slice(data) };
                        cbuffer.extend(u8slice);
                        if cbuffer.len() > 2048 {
                            let buffer = &cbuffer[..2048];
                            let client = *client_addr.lock().unwrap();
                            if args.with_aptx {
                                let mut written = 0usize;
                                let processed = ctx.encode(&buffer, &mut enc_buffer, &mut written);
                                if processed != buffer.len() {
                                    error!(
                                        "Fail to encode processed {} out of {}",
                                        processed,
                                        buffer.len()
                                    );
                                }
                                match socket.send_to(&enc_buffer, client) {
                                    Ok(nbytes) => debug!("Sending to {:?} {}", client, nbytes),
                                    Err(err) => error!("{}", err),
                                }
                            } else {
                                match socket.send_to(&buffer, client) {
                                    Ok(nbytes) => debug!("Sending to {:?} {}", client, nbytes),
                                    Err(err) => error!("{}", err),
                                }
                            }
                            cbuffer.drain(..2048);
                        }
                    },
                    move |err| {
                        error!("An error occurred on the input audio stream: {}", err);
                    },
                    None,
                )
                .expect("Fail to build output stream");
            stream.play().unwrap();
            loop {}
        });
    });
}

#[cfg(target_os = "android")]
fn main() {}
