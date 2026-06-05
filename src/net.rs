//! Network discovery and peer connection for lan-clipboard
//!
//! - UDP broadcast discovery (local network)
//! - TCP listener for incoming connections
//! - Simple length-prefixed framing (u32 big-endian) for messages

use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use serde::{Deserialize, Serialize};

const DISCOVERY_PORT: u16 = 43576;
const DEFAULT_TCP_PORT: u16 = 43577;
const DISCOVERY_INTERVAL_SECS: u64 = 3;

#[derive(Debug, Serialize, Deserialize)]
struct DiscoveryPacket {
    /// group id derived from executable name
    group: String,
    /// tcp port the sender is listening on
    port: u16,
}

/// Start discovery and TCP listener. This function spawns background tasks and returns.
#[cfg(feature = "net")]
pub async fn start_discovery() -> Result<(), Box<dyn Error>> {
    // Start TCP listener
    let listener_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), DEFAULT_TCP_PORT);
    let listener = tokio::net::TcpListener::bind(listener_addr).await?;
    log::info!("TCP listener bound to {}", listener.local_addr()?);

    // Determine group id from exe name. Fallback to "lan-clipboard" if not available.
    let group = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
        .unwrap_or_else(|| "lan-clipboard".to_string());

    // Spawn task to accept incoming TCP connections
    let accept_group = group.clone();
    tokio::spawn(async move {
        let listener = listener; // move into spawn
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    log::info!("Accepted TCP connection from {}", addr);
                    // handle connection in background
                    tokio::spawn(async move {
                        if let Err(e) = handle_tcp_connection(stream).await {
                            log::warn!("connection handler error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    log::warn!("accept error: {}", e);
                }
            }
        }
    });

    // UDP socket for discovery (broadcast)
    let udp_socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
    udp_socket.set_broadcast(true)?;

    let discovery_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), DISCOVERY_PORT);

    // spawn task to listen for discovery packets
    let udp_socket_clone = udp_socket.clone();
    let listen_group = accept_group.clone();
    tokio::spawn(async move {
        let mut buf = [0u8; 1500];
        loop {
            match udp_socket_clone.recv_from(&mut buf).await {
                Ok((n, src)) => {
                    if let Ok(pkt) = serde_json::from_slice::<DiscoveryPacket>(&buf[..n]) {
                        // Log discovered peers in same group
                        if pkt.group == listen_group {
                            log::info!("Discovered peer in same group from {}:{}", src.ip(), pkt.port);
                            // Attempt a TCP connection to the peer to warm the connection (best-effort)
                            let peer = SocketAddr::new(src.ip(), pkt.port);
                            let _ = tokio::spawn(async move {
                                match tokio::net::TcpStream::connect(peer).await {
                                    Ok(mut s) => {
                                        log::info!("Connected to discovered peer: {}", peer);
                                        // Optionally send a small probe frame (length=0) to indicate presence
                                        use tokio::io::AsyncWriteExt;
                                        let _ = s.write_all(&0u32.to_be_bytes()).await;
                                    }
                                    Err(e) => {
                                        log::debug!("failed to connect to discovered peer {}: {}", peer, e);
                                    }
                                }
                            });
                        } else {
                            log::debug!("Ignoring discovery packet from different group {}", pkt.group);
                        }
                    } else {
                        log::debug!("Received non-conforming discovery packet from {}", src);
                    }
                }
                Err(e) => {
                    log::warn!("udp recv_from error: {}", e);
                }
            }
        }
    });

    // spawn task to periodically broadcast our presence
    let udp_socket_send = udp_socket.clone();
    let send_group = group.clone();
    tokio::spawn(async move {
        loop {
            let pkt = DiscoveryPacket {
                group: send_group.clone(),
                port: DEFAULT_TCP_PORT,
            };
            match serde_json::to_vec(&pkt) {
                Ok(buf) => {
                    if let Err(e) = udp_socket_send.send_to(&buf, discovery_addr).await {
                        log::warn!("failed to send discovery packet: {}", e);
                    }
                }
                Err(e) => {
                    log::warn!("failed to serialize discovery packet: {}", e);
                }
            }
            tokio::time::sleep(Duration::from_secs(DISCOVERY_INTERVAL_SECS)).await;
        }
    });

    Ok(())
}

/// Handle a single TCP connection using a simple length-prefixed framing (u32 BE).
async fn handle_tcp_connection(stream: tokio::net::TcpStream) -> Result<(), Box<dyn Error>> {
    use tokio::io::{AsyncReadExt, BufReader};

    let peer = stream.peer_addr().ok();
    let mut reader = BufReader::new(stream);

    loop {
        // read 4-byte length prefix
        let mut len_buf = [0u8; 4];
        if let Err(e) = reader.read_exact(&mut len_buf).await {
            log::debug!("connection closed or read error from {:?}: {}", peer, e);
            return Ok(());
        }
        let len = u32::from_be_bytes(len_buf) as usize;
        if len == 0 {
            // empty probe frame
            log::debug!("received empty probe frame from {:?}", peer);
            continue;
        }
        let mut data = vec![0u8; len];
        if let Err(e) = reader.read_exact(&mut data).await {
            log::warn!("failed to read frame payload from {:?}: {}", peer, e);
            return Ok(());
        }

        // For now we don't attempt to decrypt or interpret payload; just log its size.
        log::info!("received tcp frame ({} bytes) from {:?}", data.len(), peer);

        // Optionally echo back the frame as an acknowledgement (not implemented)
    }
}
