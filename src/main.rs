mod crypto;

use std::net::SocketAddr;
use tokio::net::TcpListener;
use log::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .format_timestamp_millis()
        .init();

    info!("🚀 局域网粘贴板服务启动");

    // 绑定到本地地址和随机端口
    let addr: SocketAddr = "127.0.0.1:0".parse()?;
    let listener = TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;

    info!("📡 服务监听在: {}", local_addr);

    // 接受连接
    loop {
        match listener.accept().await {
            Ok((socket, peer_addr)) => {
                info!("✅ 新连接: {}", peer_addr);
                tokio::spawn(async move {
                    if let Err(e) = handle_client(socket).await {
                        error!("❌ 处理客户端错误: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("❌ 接受连接错误: {}", e);
            }
        }
    }
}

async fn handle_client(
    mut socket: tokio::net::TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut buffer = vec![0; 1024];
    
    match socket.read(&mut buffer).await? {
        0 => {
            info!("客户端断开连接");
        }
        n => {
            let data = String::from_utf8_lossy(&buffer[..n]);
            info!("📥 接收数据: {}", data);

            // 回复消息
            socket
                .write_all(b"已收到你的消息！\n")
                .await?;
        }
    }

    Ok(())
}
