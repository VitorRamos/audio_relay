extern crate libpulse_binding as pulse;
extern crate libpulse_simple_binding as psimple;
use clap::Parser;
use psimple::Simple;
use pulse::{def::BufferAttr, sample, stream::Direction};
use std::{
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

fn get_source_by_name(pattern: &'static str) -> String {
    use pulse::context::{Context, FlagSet};
    use pulse::mainloop::standard::Mainloop;

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

fn spwan_udp_server(addr: &str, port: u16, func: impl Fn(String)){

}

fn main() {
    let args = Args::parse();

    let client_addr = Arc::new(Mutex::new(SocketAddr::new(
        Ipv4Addr::new(192, 168, 0, 13).into(),
        4051,
    )));
    std::thread::scope(|s| {
        s.spawn(|| {
            let server_addr = format!("{}:{}", args.addr, args.port_addr);
            let socket = UdpSocket::bind(&server_addr)
                .unwrap_or_else(|_| panic!("Failed to bind server on {}", server_addr));
            let mut buffer = [0; 12];
            loop {
                let (nbytes, mut src) = socket
                    .recv_from(&mut buffer)
                    .expect("Failed to receive data");
                let message = String::from_utf8_lossy(&buffer[..nbytes]);
                println!("Received: {} from {}", message, src);
                src.set_port(4051);
                *client_addr.lock().unwrap() = src;
            }
        });
        s.spawn(|| {
            let server_addr = format!("{}:{}", args.addr, args.port_cmds);
            let socket = UdpSocket::bind(&server_addr)
                .unwrap_or_else(|_| panic!("Failed to bind server on {}", server_addr));
            let mut buffer = [0; 12];
            loop {
                let (nbytes, src) = socket
                    .recv_from(&mut buffer)
                    .expect("Failed to receive data");
                let message = String::from_utf8_lossy(&buffer[..nbytes]).to_string();
                println!("Received: {} from {}", message, src);
                if message.trim() == "NEXT" {
                } else if message.trim() == "PREV" {
                }
            }
        });
        s.spawn(|| {
            let socket = UdpSocket::bind("0.0.0.0:0").expect("Error creating client");
            let ss = sample::Spec {
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
            let s = Simple::new(
                None,
                "pc_relay",
                Direction::Record,
                Some(&get_source_by_name("Monitor of Built-in")),
                "System sound",
                &ss,
                None,
                Some(&attr),
            )
            .unwrap();
            loop {
                let mut buffer = [0u8; 2048];
                s.read(&mut buffer).unwrap();
                let n = socket
                    .send_to(&buffer, *client_addr.lock().unwrap())
                    .unwrap();
                println!("Sending to {:?} {}", buffer, n);
                std::thread::sleep(Duration::from_secs(1));
            }
        });
    });
}
