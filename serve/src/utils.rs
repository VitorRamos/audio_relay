extern crate libpulse_binding as pulse;
extern crate libpulse_simple_binding as psimple;

use log::{debug, error, info};

use std::{
    borrow::Cow,
    fmt::Display,
    net::{SocketAddr, UdpSocket},
    sync::{Arc, Mutex},
    time::Duration,
};

use dbus::{
    arg::messageitem::MessageItem,
    blocking::{BlockingSender, Connection},
    Message,
};

use pulse::{
    context::{Context, FlagSet},
    mainloop::standard::Mainloop,
};

pub fn pulse_get_source_by_name(pattern: &'static str) -> String {
    let mut main_loop = Mainloop::new().expect("Fail to create pulse mainloop");
    let mut ctx = Context::new(&main_loop, "test").expect("Fail to get pulse context");
    ctx.set_state_callback(Some(Box::new(|| {})));
    ctx.connect(None, FlagSet::NOFLAGS, None)
        .map_err(|err| error!("{err}"))
        .unwrap();
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
    let owned_name = name.lock().unwrap().clone();
    owned_name
}

pub enum PlayerControl {
    Previous,
    Next,
}

impl Display for PlayerControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayerControl::Previous => write!(f, "Previous"),
            PlayerControl::Next => write!(f, "Next"),
        }
    }
}

pub fn dbus_media_control(method: PlayerControl) {
    let cnn = Connection::new_session().expect("Failed to open connection to session message bus");
    let path = "/org/mpris/MediaPlayer2";
    let dest = "org.mpris.MediaPlayer2.playerctld";
    let name = "org.mpris.MediaPlayer2.Player";
    let message = Message::new_method_call(dest, path, name, method.to_string())
        .expect("Failed to create message");
    let reply = cnn
        .send_with_reply_and_block(message, Duration::from_millis(5000))
        .expect("Failed to send message and receive reply");
    if reply.get_items().is_empty() {
        info!("Dbus {name} received empty reply");
    }
}

fn _dbus_get_players() -> Vec<String> {
    let cnn = Connection::new_session().expect("Fail to open dbus connection");
    let path = "/org/freedesktop/DBus";
    let dest = "org.freedesktop.DBus";
    let method = "ListNames";
    let msg = Message::new_method_call(dest, path, "org.freedesktop.DBus", method)
        .expect("Failed to create message");
    let reply = cnn
        .send_with_reply_and_block(msg, Duration::from_millis(5000))
        .expect("Failed to send message and receive reply");
    let items = reply.get_items();
    let mut res = Vec::new();
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

pub fn udp_server_loop_data<const T: usize>(
    addr: &str,
    port: u16,
    func: impl Fn(Cow<str>, SocketAddr),
) {
    let server_addr = format!("{}:{}", addr, port);
    let socket = UdpSocket::bind(&server_addr)
        .unwrap_or_else(|_| panic!("Failed to bind server on {}", server_addr));
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
