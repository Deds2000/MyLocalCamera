//! One-shot, LAN-only transfer of the pairing URI to the instrumented app.
//! The URI is served from memory and is never written or logged.

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio::time::timeout;

pub struct PairingBroker {
    address: SocketAddr,
    task: JoinHandle<()>,
}

impl PairingBroker {
    pub async fn start(bind_ip: IpAddr, uri: String) -> io::Result<Self> {
        let listener = TcpListener::bind(SocketAddr::new(bind_ip, 0)).await?;
        let address = listener.local_addr()?;
        let task = tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    return;
                };
                match serve_once(&mut stream, &uri).await {
                    Ok(true) => return,
                    Ok(false) | Err(_) => continue,
                }
            }
        });
        Ok(Self { address, task })
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub async fn shutdown(self) {
        self.task.abort();
        let _ = self.task.await;
    }
}

async fn serve_once(stream: &mut TcpStream, uri: &str) -> io::Result<bool> {
    let mut request = Vec::with_capacity(256);
    loop {
        let mut chunk = [0_u8; 128];
        let read = match timeout(Duration::from_secs(3), stream.read(&mut chunk)).await {
            Ok(result) => result?,
            Err(_) => return Ok(false),
        };
        if read == 0 {
            return Ok(false);
        }
        request.extend_from_slice(&chunk[..read]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if request.len() >= MAX_REQUEST_BYTES {
            stream
                .write_all(
                    b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await?;
            return Ok(false);
        }
    }
    if !valid_request(&request) {
        stream
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await?;
        return Ok(false);
    }

    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
        uri.len()
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(uri.as_bytes()).await?;
    Ok(true)
}

fn valid_request(bytes: &[u8]) -> bool {
    bytes
        .split(|byte| *byte == b'\n')
        .next()
        .is_some_and(|line| {
            line == b"GET /pairing HTTP/1.1\r" || line == b"GET /pairing HTTP/1.0\r"
        })
}

const MAX_REQUEST_BYTES: usize = 1_024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_pairing_get_path() {
        assert!(valid_request(b"GET /pairing HTTP/1.1\r\nHost: pc\r\n\r\n"));
        assert!(!valid_request(b"GET / HTTP/1.1\r\nHost: pc\r\n\r\n"));
        assert!(!valid_request(
            b"POST /pairing HTTP/1.1\r\nHost: pc\r\n\r\n"
        ));
    }

    #[tokio::test]
    async fn serves_uri_once_from_memory() {
        let broker = PairingBroker::start(
            IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            "localcamera://connect?probe=ephemeral".to_owned(),
        )
        .await
        .unwrap();
        let mut client = TcpStream::connect(broker.address()).await.unwrap();
        client
            .write_all(b"GET /pairing HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();
        assert!(response.ends_with(b"localcamera://connect?probe=ephemeral"));
        broker.shutdown().await;
    }
}
