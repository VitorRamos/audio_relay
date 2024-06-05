use cpal::traits::{DeviceTrait, HostTrait};
use std::{
    borrow::Cow,
    fmt::Display,
    net::{SocketAddr, UdpSocket},
};

pub unsafe fn into_slice<Tin, Tout>(input: &[Tin]) -> &[Tout] {
    let len = input.len() * std::mem::size_of::<Tin>() / std::mem::size_of::<Tout>();
    let ptr = input.as_ptr() as *const Tout;
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

pub fn get_source_by_name(pattern: &'static str) -> Option<cpal::Device> {
    let host = cpal::default_host();
    for d in host.output_devices().unwrap() {
        let name = d.name().unwrap();
        if name.contains(pattern) {
            return Some(d);
        }
    }
    None
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

#[cfg(target_os = "linux")]
pub fn media_control(method: PlayerControl) {
    use dbus::{
        blocking::{BlockingSender, Connection},
        Message,
    };
    let cnn = Connection::new_session().expect("Failed to open connection to session message bus");
    let path = "/org/mpris/MediaPlayer2";
    let dest = "org.mpris.MediaPlayer2.playerctld";
    let name = "org.mpris.MediaPlayer2.Player";
    let message = Message::new_method_call(dest, path, name, method.to_string())
        .expect("Failed to create message");
    let reply = cnn
        .send_with_reply_and_block(message, std::time::Duration::from_millis(5000))
        .expect("Failed to send message and receive reply");
    if reply.get_items().is_empty() {
        log::info!("Dbus {name} received empty reply");
    }
}
#[cfg(target_os = "windows")]
pub fn media_control(method: PlayerControl) {}

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
        log::debug!("Received: {} from {}", message, client);
        func(message, client);
    }
}
