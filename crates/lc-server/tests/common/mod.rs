//! Cliente mínimo de teste: fala o protocolo LocalCamera por TCP
//! (envia quadros, lê com [`FrameReader`]).

use std::io;
use std::time::Duration;

use lc_protocol::json::{AppInfo, Auth, Caps, DeviceInfo, Hello, ProtocolVersion};
use lc_protocol::{FrameReader, Message};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Buffer de leitura do cliente de teste.
const BUF_SIZE: usize = 64 * 1024;

/// Cliente de protocolo TCP para os testes de integração.
pub struct TestClient {
    stream: TcpStream,
    reader: FrameReader,
}

impl TestClient {
    /// Conecta em `127.0.0.1:port` (sem autenticar).
    pub async fn connect(port: u16) -> io::Result<Self> {
        let stream = TcpStream::connect(("127.0.0.1", port)).await?;
        stream.set_nodelay(true)?;
        Ok(Self {
            stream,
            reader: FrameReader::new(),
        })
    }

    /// Envia uma mensagem completa (serializa o quadro e faz flush).
    pub async fn send(&mut self, msg: &Message) -> io::Result<()> {
        let bytes = msg
            .encode(0)
            .map_err(|e| io::Error::other(format!("encode de quadro: {e}")))?;
        self.stream.write_all(&bytes).await?;
        self.stream.flush().await
    }

    /// Envia bytes crus (para quadros fora do protocolo).
    pub async fn send_raw(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.stream.write_all(bytes).await?;
        self.stream.flush().await
    }

    /// Aguarda a próxima mensagem por até `timeout`.
    ///
    /// `None` em timeout, EOF (servidor fechou) ou erro de leitura do
    /// quadro — ou seja, "não chegou resposta".
    pub async fn recv(&mut self, timeout: Duration) -> Option<Message> {
        let deadline = tokio::time::sleep(timeout);
        tokio::pin!(deadline);
        let mut buf = [0u8; BUF_SIZE];
        loop {
            tokio::select! {
                res = self.stream.read(&mut buf) => match res {
                    Ok(0) => return None, // EOF
                    Ok(n) => match self.reader.push(&buf[..n]) {
                        Ok(frames) => {
                            if let Some(frame) = frames.into_iter().next() {
                                return Message::from_frame(&frame).ok();
                            }
                        }
                        // leitor envenenado: a conexão morreu
                        Err(_) => return None,
                    },
                    Err(_) => return None,
                },
                _ = &mut deadline => return None,
            }
        }
    }

    /// `HELLO` com auth `pair` (`{kind:"pair", token}`) e versão 1.0.
    pub fn hello_pair(token: &str, device_id: &str) -> Message {
        Self::hello_with(
            device_id,
            1,
            0,
            Auth::Pair {
                token: token.to_owned(),
            },
        )
    }

    /// `HELLO` com auth `pair` e versão explícita.
    pub fn hello_pair_v(token: &str, device_id: &str, major: u16, minor: u16) -> Message {
        Self::hello_with(
            device_id,
            major,
            minor,
            Auth::Pair {
                token: token.to_owned(),
            },
        )
    }

    /// `HELLO` com auth `device` (`{kind:"device", deviceId, key}`).
    pub fn hello_device(device_id: &str, key: &str) -> Message {
        Self::hello_with(
            device_id,
            1,
            0,
            Auth::Device {
                device_id: device_id.to_owned(),
                key: key.to_owned(),
            },
        )
    }

    fn hello_with(device_id: &str, major: u16, minor: u16, auth: Auth) -> Message {
        Message::Hello(Hello {
            protocol: ProtocolVersion { major, minor },
            app: AppInfo {
                version: "0.0.0-test".to_owned(),
                platform: "android".to_owned(),
            },
            device: DeviceInfo {
                id: device_id.to_owned(),
                name: "Fone de teste".to_owned(),
                model: String::new(),
                android_api: 33,
            },
            auth,
            caps: Caps::default(),
        })
    }
}
