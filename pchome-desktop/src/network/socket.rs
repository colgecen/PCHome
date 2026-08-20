use anyhow::{Context, Result};
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tracing::{debug, info, warn};

const STUN_SERVERS: [&str; 3] = [
    "stun.l.google.com:19302",
    "stun1.l.google.com:19302",
    "stun2.l.google.com:19302",
];

pub async fn bind_udp(addr: SocketAddr) -> Result<UdpSocket> {
    let socket = UdpSocket::bind(addr).await?;
    info!("UDP socket bound to {}", addr);
    Ok(socket)
}

pub async fn nat_probe(socket: &UdpSocket, target: SocketAddr) -> Result<Option<SocketAddr>> {
    let mut buf = [0u8; 64];
    socket.send_to(b"PCHOME-NAT-PROBE", target).await?;

    match tokio::time::timeout(std::time::Duration::from_secs(2), socket.recv_from(&mut buf)).await {
        Ok(Ok((len, addr))) => {
            debug!("NAT probe response from {}: {} bytes", addr, len);
            Ok(Some(addr))
        }
        Ok(Err(e)) => {
            warn!("NAT probe recv error: {}", e);
            Ok(None)
        }
        Err(_) => {
            warn!("NAT probe timeout");
            Ok(None)
        }
    }
}

pub async fn stun_bind_request(socket: &UdpSocket, stun_addr: SocketAddr) -> Result<Option<SocketAddr>> {
    let mut buf = [0u8; 512];
    let request = build_stun_bind_request();

    socket.send_to(&request, stun_addr).await?;

    match tokio::time::timeout(std::time::Duration::from_secs(2), socket.recv_from(&mut buf)).await {
        Ok(Ok((len, _from))) => {
            if let Some(mapped) = parse_stun_bind_response(&buf[..len]) {
                debug!("STUN mapped address: {}", mapped);
                Ok(Some(mapped))
            } else {
                warn!("Invalid STUN response");
                Ok(None)
            }
        }
        Ok(Err(e)) => {
            warn!("STUN recv error: {}", e);
            Ok(None)
        }
        Err(_) => {
            warn!("STUN request timeout");
            Ok(None)
        }
    }
}

pub async fn discover_external_addr(socket: &UdpSocket) -> Result<Option<SocketAddr>> {
    for server in &STUN_SERVERS {
        if let Ok(Some(addr)) = stun_bind_request(socket, server.parse()?).await {
            return Ok(Some(addr));
        }
    }
    warn!("All STUN servers failed");
    Ok(None)
}

fn build_stun_bind_request() -> Vec<u8> {
    let mut buf = Vec::with_capacity(20);

    buf.extend_from_slice(&(0u16).to_be_bytes());
    buf.extend_from_slice(&(0x0001u16).to_be_bytes());

    let mut transaction_id = [0u8; 12];
    transaction_id[8..].copy_from_slice(&rand::random::<[u8; 4]>());
    buf.extend_from_slice(&transaction_id);

    buf
}

fn parse_stun_bind_response(data: &[u8]) -> Option<SocketAddr> {
    if data.len() < 20 {
        return None;
    }

    let msg_type = u16::from_be_bytes([data[0], data[1]]);
    if msg_type != 0x0101 {
        return None;
    }

    let mut offset = 20;
    while offset + 4 <= data.len() {
        let attr_type = u16::from_be_bytes([data[offset], data[offset + 1]]);
        let attr_len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;

        if attr_type == 0x0020 && offset + 20 + attr_len <= data.len() {
            let ip_offset = offset + 12;
            let port = u16::from_be_bytes([
                data[ip_offset],
                data[ip_offset + 1],
            ]);
            let ip = std::net::Ipv4Addr::new(
                data[ip_offset + 4],
                data[ip_offset + 5],
                data[ip_offset + 6],
                data[ip_offset + 7],
            );
            return Some(SocketAddr::new(std::net::IpAddr::V4(ip), port));
        }

        offset += 4 + attr_len;
    }

    None
}
