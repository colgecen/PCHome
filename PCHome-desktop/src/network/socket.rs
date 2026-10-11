use anyhow::Result;
use rand::Rng;
use std::net::SocketAddr;
use tokio::net::UdpSocket;

const STUN_SERVERS: [&str; 3] = [
    "stun.l.google.com:19302",
    "stun1.l.google.com:19302",
    "stun2.l.google.com:19302",
];

pub async fn bind_udp(addr: SocketAddr) -> Result<UdpSocket> {
    let socket = UdpSocket::bind(addr).await?;
    log::info!("UDP socket bound to {}", addr);
    Ok(socket)
}

pub async fn nat_probe(socket: &UdpSocket, target: SocketAddr) -> Result<Option<SocketAddr>> {
    let mut buf = [0u8; 64];
    socket.send_to(b"PCHOME-NAT-PROBE", target).await?;

    match tokio::time::timeout(std::time::Duration::from_secs(2), socket.recv_from(&mut buf)).await {
        Ok(Ok((len, addr))) => {
            log::debug!("NAT probe response from {}: {} bytes", addr, len);
            Ok(Some(addr))
        }
        Ok(Err(e)) => {
            log::warn!("NAT probe recv error: {}", e);
            Ok(None)
        }
        Err(_) => {
            log::warn!("NAT probe timeout");
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
                log::debug!("STUN mapped address: {}", mapped);
                Ok(Some(mapped))
            } else {
                log::warn!("Invalid STUN response");
                Ok(None)
            }
        }
        Ok(Err(e)) => {
            log::warn!("STUN recv error: {}", e);
            Ok(None)
        }
        Err(_) => {
            log::warn!("STUN request timeout");
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
    log::warn!("All STUN servers failed");
    Ok(None)
}

fn build_stun_bind_request() -> Vec<u8> {
    const MAGIC_COOKIE: u32 = 0x2112_A442;
    let mut buf = Vec::with_capacity(20);

    // STUN Binding Request: message type 0x0001, message length 0.
    buf.extend_from_slice(&0x0001u16.to_be_bytes());
    buf.extend_from_slice(&0x0000u16.to_be_bytes());
    // Magic cookie.
    buf.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
    // 12-byte transaction ID (first 4 bytes overlap the magic cookie space per RFC).
    let mut transaction_id = [0u8; 12];
    rand::thread_rng().fill(&mut transaction_id[..]);
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

        // XOR-MAPPED-ADDRESS (0x0020).
        if attr_type == 0x0020 && offset + 12 <= data.len() {
            const MAGIC_COOKIE: u32 = 0x2112_A442;
            let xport = u16::from_be_bytes([data[offset + 6], data[offset + 7]])
                ^ (MAGIC_COOKIE >> 16) as u16;
            let cookie = MAGIC_COOKIE.to_be_bytes();
            let ip = std::net::Ipv4Addr::new(
                data[offset + 8] ^ cookie[0],
                data[offset + 9] ^ cookie[1],
                data[offset + 10] ^ cookie[2],
                data[offset + 11] ^ cookie[3],
            );
            return Some(SocketAddr::new(std::net::IpAddr::V4(ip), xport));
        }

        offset += 4 + attr_len;
    }

    None
}
