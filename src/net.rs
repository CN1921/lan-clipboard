//! Network discovery and peer connection for lan-clipboard
//!
//! - UDP broadcast discovery (local network)
//! - TCP listener for incoming encrypted clipboard messages
//! - Simple length-prefixed framing with serde-json for control messages

use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use serde::{Deserialize, Serialize};
use crate::crypto::EncryptedMessage;

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

/// Start discovery and TCP listener. This function runs until the program exits.
#[cfg(feature = "net")]
pub async fn start_discovery() -> Result<(), Box<dyn Error>> {
    // Start TCP listener
    let listener_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), DEFAULT_TCP_PORT);
    let listener = tokio::net::TcpListener::bind(listener_addr).await?;
    log::info!("TCP listener bound to {}", listener.local_addr()?);

    // Determine group id from exe name. Fallback to "default" if not available.
    let group = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
        .unwrap_or_else(|| "lan-clipboard".to_string());

    // Spawn task to accept incoming TCP connections
    let accept_group = group.clone();
    tokio::spawn(async move {
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
        //#[allow(unreachable_code)]
        //Ok::<(), Box<dyn Error>>(())
    });

    // UDP socket for discovery (broadcast)
    let udp_socket = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
    udp_socket.set_broadcast(true)?;

    let discovery_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), DISCOVERY_PORT);

    // spawn task to listen for discovery packets
    let udp_socket_clone = udp_socket.clone();
    tokio::spawn(async move {
        let mut buf = [0u8; 1500];
        loop {
            match udp_socket_clone.recv_from(&mut buf).await {
                Ok((n, src)) => {
                    if let Ok(pkt) = serde_json::from_slice::<DiscoveryPacket>(&buf[..n]) {
                        // Ignore our own announcements (same group and same origin)
                        if pkt.group == accept_group {
                            log::debug!("Discovered peer in same group from {}:{}