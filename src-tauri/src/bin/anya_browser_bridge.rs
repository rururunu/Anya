//! Chrome native messaging host. stdout is exclusively length-prefixed JSON.
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};
fn main() {
    let Some(origin) = std::env::args().nth(1) else {
        return;
    };
    let Some(local) = std::env::var_os("LOCALAPPDATA") else {
        return;
    };
    let directory = std::path::PathBuf::from(local).join("Anya");
    let allowed =
        std::fs::read_to_string(directory.join("browser-extension-origin.txt")).unwrap_or_default();
    if origin != allowed.trim() {
        return;
    }
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    loop {
        let mut header = [0u8; 4];
        if input.read_exact(&mut header).is_err() {
            break;
        }
        let size = u32::from_le_bytes(header) as usize;
        if size > 1048576 {
            break;
        }
        let mut bytes = vec![0; size];
        if input.read_exact(&mut bytes).is_err() {
            break;
        }
        let reply = forward(&directory, &bytes).unwrap_or_else(|| b"{\"ok\":false}".to_vec());
        if output
            .write_all(&(reply.len() as u32).to_le_bytes())
            .and_then(|_| output.write_all(&reply))
            .and_then(|_| output.flush())
            .is_err()
        {
            break;
        }
    }
}
fn forward(directory: &std::path::Path, bytes: &[u8]) -> Option<Vec<u8>> {
    let config: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("browser-bridge.json")).ok()?).ok()?;
    let port = u16::try_from(config["port"].as_u64()?).ok()?;
    let mut packet: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    if !packet.is_object() {
        return None;
    }
    packet["token"] = serde_json::Value::String(config["token"].as_str()?.to_owned());
    let bytes = serde_json::to_vec(&packet).ok()?;
    let mut socket = TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().ok()?,
        Duration::from_millis(250),
    )
    .ok()?;
    socket
        .set_read_timeout(Some(Duration::from_millis(500)))
        .ok()?;
    socket
        .set_write_timeout(Some(Duration::from_millis(500)))
        .ok()?;
    socket.write_all(&(bytes.len() as u32).to_le_bytes()).ok()?;
    socket.write_all(&bytes).ok()?;
    let mut length = [0; 4];
    socket.read_exact(&mut length).ok()?;
    let size = u32::from_le_bytes(length) as usize;
    if size > 1048576 {
        return None;
    }
    let mut response = vec![0; size];
    socket.read_exact(&mut response).ok()?;
    Some(response)
}
