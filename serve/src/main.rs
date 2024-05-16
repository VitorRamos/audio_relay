extern crate libpulse_binding as pulse;
extern crate libpulse_simple_binding as psimple;
use clap::Parser;
use dbus::{arg::messageitem::MessageItem, blocking::{BlockingSender, Connection}, Message};
use log::{debug, error, info};
use psimple::Simple;
use pulse::{context::{Context, FlagSet}, def::BufferAttr, mainloop::standard::Mainloop, sample, stream::Direction};
use std::{
    borrow::Cow,
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Debug, Parser)]
struct Args {
    #[arg(short, long)]
    with_aptx: bool,

    #[arg(short, long, default_value_t = String::from("0.0.0.0"))]
    addr: String,

    #[arg(long, default_value_t = 4052)]
    port_addr: u16,

    #[arg(long, default_value_t = 4053)]
    port_cmds: u16,
}

fn pulse_get_source_by_name(pattern: &'static str) -> String {
    let mut main_loop = Mainloop::new().unwrap();
    let mut ctx = Context::new(&main_loop, "test").unwrap();
    ctx.set_state_callback(Some(Box::new(|| {})));
    ctx.connect(None, FlagSet::NOFLAGS, None).unwrap();
    loop {
        match ctx.get_state() {
            pulse::context::State::Ready => {
                break;
            }
            _ => main_loop.iterate(false),
        };
    }
    let name = Arc::new(Mutex::new(String::new()));
    let name_clone = name.clone();
    let op = ctx.introspect().get_source_info_list(move |info| {
        if let pulse::callbacks::ListResult::Item(item) = info {
            let desc = item.description.clone().unwrap();
            if desc.find(pattern).is_some() {
                *name_clone.lock().unwrap() = item.name.clone().unwrap().to_string();
            }
        }
    });
    while let pulse::operation::State::Running = op.get_state() {
        main_loop.iterate(false);
    }
    let a = name.lock().unwrap().clone();
    a
}

fn dbus_media_control(name: &str) {
    let cnn = Connection::new_session().expect("Failed to open connection to session message bus");
    let path = "/org/mpris/MediaPlayer2";
    let dest = "playerctld";
    let last_dot = name.rsplit('.').next().expect("Failed to split string");
    let name = &name[..name.rfind('.').expect("Failed to find last dot")];
    let message = Message::new_method_call(dest, path, name, last_dot)
        .expect("Failed to create message");
    let reply = cnn.send_with_reply_and_block(message, Duration::from_millis(5000))
        .expect("Failed to send message and receive reply");
    if reply.get_items().is_empty() {
        eprintln!("Received empty reply");
    }
}

fn _dbus_get_players() -> Vec<String> {
    let mut res = Vec::new();
    let cnn = Connection::new_session().expect("Fail to open dbus connection");
    let path = "/org/freedesktop/DBus";
    let dest = "org.freedesktop.DBus";
    let msg = Message::new_method_call(dest, path, "org.freedesktop.DBus", "ListNames")
        .expect("Failed to create message");
    let reply = cnn.send_with_reply_and_block(msg, Duration::from_millis(5000))
        .expect("Failed to send message and receive reply");
    let items = reply.get_items();
    if let Some(MessageItem::Array(array)) = items.first() {
        array.iter().for_each(|item| {
            if let MessageItem::Str(val) = item {
                if val.contains("org.mp") {
                    res.push(val.to_string());
                }
            }
        });
    }
    res
}

fn udp_server_loop_data<const T: usize>(
    addr: &str,
    port: u16,
    func: impl Fn(Cow<str>, SocketAddr),
) {
    let server_addr = format!("{}:{}", addr, port);
    let socket =
        UdpSocket::bind(&server_addr).expect(&format!("Failed to bind server on {}", server_addr));
    let mut buffer = [0; T];
    loop {
        let (nbytes, client) = socket
            .recv_from(&mut buffer)
            .expect("Failed to receive data");
        let message = String::from_utf8_lossy(&buffer[..nbytes]);
        debug!("Received: {} from {}", message, client);
        func(message, client);
    }
}

fn main() {
    env_logger::init();
    let args = Args::parse();

    let client_addr = Arc::new(Mutex::new(SocketAddr::new(
        Ipv4Addr::new(192, 168, 0, 13).into(),
        4051,
    )));
    std::thread::scope(|s| {
        s.spawn(|| {
            udp_server_loop_data::<12>(&args.addr, args.port_addr, |_, mut client| {
                client.set_port(4051);
                *client_addr.lock().unwrap() = client;
            })
        });
        s.spawn(|| {
            udp_server_loop_data::<12>(&args.addr, args.port_cmds, |data, _| {
                if data.trim() == "NEXT" {
                    dbus_media_control("org.mpris.MediaPlayer2.Player.Previous");
                } else if data.trim() == "PREV" {
                    dbus_media_control("org.mpris.MediaPlayer2.Player.Next");
                }
            });
        });
        s.spawn(|| {
            let socket = UdpSocket::bind("0.0.0.0:0").expect("Error creating client");
            let audio_spec = sample::Spec {
                format: sample::Format::S16le,
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
                Direction::Record,
                Some(&pulse_get_source_by_name("Monitor of Built-in")),
                "System sound",
                &audio_spec,
                None,
                Some(&attr),
            )
            .expect("Fail to connect to the audio server");
            loop {
                let mut buffer = [0u8; 2048];
                pulse_cnn.read(&mut buffer).unwrap_or_else(|err| {
                    error!("{}", err);
                });
                if buffer.iter().map(|&x| x as u64).sum::<u64>() == 0 {
                    continue;
                }
                let client = *client_addr.lock().unwrap();
                match socket.send_to(&buffer, client) {
                    Ok(nbytes) => info!("Sending to {:?} {}", client, nbytes),
                    Err(err) => error!("{}", err),
                }
            }
        });
    });
}
