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
    std::thread::scope(|s| {
        s.spawn(|| {
            utils::udp_server_loop_data::<12>(&args.addr, args.port_addr, |_, mut client| {
                client.set_port(args.port_audio);
                *client_addr.lock().unwrap() = client;
                info!("New client {client_addr:?}");
            })
        });
        s.spawn(|| {
            utils::udp_server_loop_data::<12>(&args.addr, args.port_cmds, |data, _| {
                if data.trim().contains("NEXT") {
                    debug!("PlayerControl::Next");
                    utils::dbus_media_control(utils::PlayerControl::Next);
                } else if data.trim().contains("PREV") {
                    debug!("PlayerControl::Previous");
                    utils::dbus_media_control(utils::PlayerControl::Previous);
                }
            });
        });
        s.spawn(|| {
            let socket = UdpSocket::bind("0.0.0.0:0").expect("Error creating client");
            let format = if args.hd {
                libpulse_binding::sample::Format::S24le
            } else {
                libpulse_binding::sample::Format::S16le
            };
            let audio_spec = libpulse_binding::sample::Spec {
                format,
                rate: 48000,
                channels: 2,
            };
            let attr = libpulse_binding::def::BufferAttr {
                maxlength: 65536,
                tlength: 2048,
                prebuf: 512,
                minreq: 512,
                fragsize: 2048,
            };
            let monitor_name = utils::pulse_get_source_by_name("Monitor of Jabra");
            info!("Output: {monitor_name}");
            let pulse_cnn = libpulse_simple_binding::Simple::new(
                None,
                "pc_relay",
                libpulse_binding::stream::Direction::Record,
                Some(&monitor_name),
                "System sound",
                &audio_spec,
                None,
                Some(&attr),
            )
            .expect("Fail to connect to the audio server");
            let mut ctx = AptxContext::new(args.hd);
            let mut buffer = [0u8; 2048];
            let mut enc_buffer = [0u8; 512];
            loop {
                match pulse_cnn.read(&mut buffer) {
                    Ok(_) => {}
                    Err(err) => error!("{}", err),
                }
                let lat = pulse_cnn
                    .get_latency()
                    .unwrap_or(libpulse_binding::time::MicroSeconds::from_secs_f32(0.0));
                if lat > libpulse_binding::time::MicroSeconds(0) {
                    info!("Latancy: {lat}");
                }
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
            }
        });
    });
}

#[cfg(target_os = "android")]
fn main() {}
