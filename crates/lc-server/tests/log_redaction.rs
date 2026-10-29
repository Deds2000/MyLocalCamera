//! ST-1.01 — nenhum segredo aparece inteiro nos logs (F1-T16).
//!
//! Captura **todos** os logs em TRACE durante um fluxo completo
//! (pareamento com `auto_approve`, áudio, reconexão via `device`, token
//! errado e ticket reutilizado) e procura os segredos usados.
//!
//! O subscriber é instalado com `set_default` (escopo de thread); o
//! runtime é `current_thread` para que todas as tasks do servidor rodem
//! na mesma thread do teste.

#[allow(dead_code)]
mod common;

use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::TestClient;
use lc_protocol::json::{AudioHello, Reason, Welcome};
use lc_protocol::Message;
use lc_server::config::ServerConfig;
use lc_server::events::ServerEvent;
use lc_server::redact::short;
use lc_server::{start, ServerHandle};
use tracing_subscriber::fmt::MakeWriter;

mod log_redaction {
    use super::*;

    /// Prazo dos `recv` dos clientes de teste.
    const TIMEOUT: Duration = Duration::from_secs(3);

    /// Coleta todos os bytes escritos pelo subscriber (clone compartilhado).
    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<u8>>>);

    impl Capture {
        /// Conteúdo capturado até agora (UTF-8 tolerante a bytes parciais).
        fn text(&self) -> String {
            String::from_utf8_lossy(&self.0.lock().unwrap_or_else(|p| p.into_inner())).into_owned()
        }
    }

    /// `io::Write` que anexa ao buffer compartilhado.
    struct CaptureWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for CaptureWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for Capture {
        type Writer = CaptureWriter;

        fn make_writer(&'a self) -> CaptureWriter {
            CaptureWriter(self.0.clone())
        }
    }

    /// Token de pareamento atual da loja de confiança.
    fn pair_token(handle: &ServerHandle) -> String {
        handle
            .trust()
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .pair_token()
            .to_owned()
    }

    /// Espera o `WELCOME` do cliente.
    async fn take_welcome(c: &mut TestClient) -> Welcome {
        match c.recv(TIMEOUT).await.expect("esperava WELCOME") {
            Message::Welcome(w) => w,
            other => panic!("esperava WELCOME, veio {other:?}"),
        }
    }

    /// ST-1.01: nenhum token/chave/ticket completo nos logs (TRACE).
    #[tokio::test(flavor = "current_thread")]
    async fn logs_never_contain_full_secrets() {
        let capture = Capture::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(capture.clone())
            .with_max_level(tracing::Level::TRACE)
            .with_ansi(false)
            .finish();
        let _guard = tracing::subscriber::set_default(subscriber);

        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = ServerConfig::for_tests(dir.path().to_path_buf(), true);
        let mut handle = start(cfg).await.expect("start");
        let port = match handle.events().recv().await.expect("evento") {
            ServerEvent::Listening { port } => port,
            other => panic!("esperava Listening, veio {other:?}"),
        };
        let token = pair_token(&handle);

        // 1) Pareamento (auto_approve) e sessão.
        let mut c1 = TestClient::connect(port).await.expect("connect");
        c1.send(&TestClient::hello_pair(&token, "dev-redact"))
            .await
            .expect("send");
        let w1 = take_welcome(&mut c1).await;
        assert!(w1.accepted, "auto_approve deve aceitar o pareamento");
        let session1 = w1.session_id.clone().expect("sessionId");
        let audio_ticket = w1.audio_ticket.clone().expect("audioTicket");
        let device_key = w1.device_key.clone().expect("deviceKey");

        // 2) Abertura do áudio com o ticket.
        let mut a1 = TestClient::connect(port).await.expect("connect áudio");
        a1.send(&Message::AudioHello(AudioHello {
            session_id: session1.clone(),
            ticket: audio_ticket.clone(),
        }))
        .await
        .expect("send áudio");
        let aw = take_welcome(&mut a1).await;
        assert!(aw.accepted, "ticket válido deve abrir o áudio");
        drop(a1);

        // 3) Reconexão via `device` (derruba a sessão 1).
        let mut c2 = TestClient::connect(port).await.expect("connect device");
        c2.send(&TestClient::hello_device("dev-redact", &device_key))
            .await
            .expect("send device");
        let w2 = take_welcome(&mut c2).await;
        assert!(w2.accepted, "deviceKey deve autenticar");
        let session2 = w2.session_id.clone().expect("sessionId 2");

        // 4) Token errado.
        let mut c3 = TestClient::connect(port)
            .await
            .expect("connect token errado");
        c3.send(&TestClient::hello_pair(&"f".repeat(32), "dev-bad"))
            .await
            .expect("send token errado");
        let w3 = take_welcome(&mut c3).await;
        assert!(!w3.accepted, "token errado deve ser recusado");

        // 5) Ticket já usado (sessão 1 encerrada): WELCOME de recusa.
        let mut a2 = TestClient::connect(port).await.expect("connect áudio 2");
        a2.send(&Message::AudioHello(AudioHello {
            session_id: session2.clone(),
            ticket: audio_ticket.clone(),
        }))
        .await
        .expect("send ticket reutilizado");
        let reuse = take_welcome(&mut a2).await;
        assert!(!reuse.accepted, "ticket reutilizado deve ser recusado");
        assert_eq!(reuse.reason, Some(Reason::BadAudioTicket));

        drop(a2);
        drop(c2);
        drop(c3);
        handle.shutdown().await;
        tokio::task::yield_now().await;

        let logs = capture.text();
        assert!(!logs.is_empty(), "os logs deveriam ter sido capturados");

        for (label, secret) in [
            ("token de pareamento", token.as_str()),
            ("deviceKey", device_key.as_str()),
            ("sessionId 1", session1.as_str()),
            ("sessionId 2", session2.as_str()),
            ("audioTicket", audio_ticket.as_str()),
        ] {
            assert!(
                !logs.contains(secret),
                "{label} apareceu inteiro nos logs:\n{logs}"
            );
        }

        // O span da sessão aparece com o id abreviado (4 chars + `…`).
        assert!(
            logs.contains(&format!("session={}", short(&session1))),
            "span da sessão 1 deve aparecer abreviado"
        );
        assert!(
            logs.contains(&format!("session={}", short(&session2))),
            "span da sessão 2 deve aparecer abreviado"
        );
        assert!(logs.contains("conn="), "span da conexão deve aparecer");
    }
}
