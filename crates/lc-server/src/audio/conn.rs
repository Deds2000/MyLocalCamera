//! Laço do socket de áudio (F1-T09b).
//!
//! O 1º quadro é `AUDIO_HELLO{sessionId, ticket}`: o ticket é resgatado
//! na `TicketStore` (uso único, 10 s, preso à sessão). Sessão inexistente,
//! ticket inválido/expirado/já usado ou de outra sessão →
//! `WELCOME{accepted:false, reason:bad_audio_ticket}` e fechar.
//!
//! Aceito: o canal é registrado na sessão (substituindo um canal anterior,
//! que fecha com `Replaced`), responde `WELCOME{accepted:true}` (campos
//! mínimos) e entra no laço:
//! - `AUDIO` → conta `audio_blocks`/`audio_bytes`/`last_audio_pts`;
//! - `PING` → `PONG` (mesmo `u64`);
//! - keepalive (F1-T10): `PING` a cada `ping_interval` e sem `PONG` por
//!   `liveness_timeout` → fecha com `ConnectionLost` (mesma regra da
//!   sessão principal; **só** o `PONG` mantém vivo);
//! - `BYE` → fecha;
//! - qualquer outro tipo → ignorado com log.
//!
//! O leitor do laço é criado com `audio_block_max` (16 KiB): um bloco
//! acima disso vira `TooLarge`/`ProtocolError` **só neste socket** e o
//! payload é checado **antes** de ser alocado. A sessão principal
//! **nunca** é afetada pela queda do áudio; o contrário vale — ao
//! encerrar, a sessão fecha o áudio.
//!
//! Motivos do fechamento deste socket (evento `AudioClosed`):
//! - `PeerClosed` — o celular fechou/mandou `BYE`, **ou** a sessão que o
//!   donava encerrou/substituiu (o kick acorda o laço e a sessão já não
//!   é a ativa);
//! - `Replaced` — um novo socket de áudio da **mesma** sessão substituiu
//!   este (a sessão segue ativa);
//! - `Shutdown` — shutdown do servidor;
//! - `ProtocolError` — bloco `AUDIO` acima do máximo, frame inválido;
//! - `ConnectionLost` / `AuthDenied` — erro de transporte / recusa de
//!   ticket (nestes o canal nunca "abriu": não há `AudioOpened`).

use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::time::Duration;

use lc_protocol::json::{AudioHello, Reason, Welcome};
use lc_protocol::{type_name, FrameReader, Message};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, watch, OwnedSemaphorePermit};

use crate::events::{CloseReason, ServerEvent};
use crate::handshake::{now_secs, write_msg, ServerCtx};
use crate::keepalive::{mono_ms, Keepalive};

/// Buffer de leitura do socket de áudio (o leitor limita o payload a
/// `audio_block_max`; o buffer só precisa caber o máximo).
const READ_BUF_SIZE: usize = 16 * 1024;

/// Dados compartilhados do laço de áudio (F1-T09b) — agrupa o que o
/// `conn.rs` já compartilha (padrão `ConnShared`) e evita
/// `too_many_arguments` em [`run_audio`].
pub(crate) struct AudioConn {
    /// Contexto do servidor (tickets e sessão).
    pub(crate) ctx: std::sync::Arc<ServerCtx>,
    /// Fila de eventos (clone — o `conn.rs` guarda o original).
    pub(crate) events_tx: mpsc::Sender<ServerEvent>,
    /// Shutdown do servidor (clone).
    pub(crate) shutdown_rx: watch::Receiver<bool>,
    /// Máximo de um bloco `AUDIO` neste socket.
    pub(crate) audio_block_max: usize,
    /// Intervalo dos `PING` de keepalive (F1-T10).
    pub(crate) ping_interval: Duration,
    /// Sem `PONG` por este intervalo → `ConnectionLost` (F1-T10).
    pub(crate) liveness_timeout: Duration,
}

/// `WELCOME` mínimo do socket de áudio.
fn audio_welcome(accepted: bool, session_id: Option<String>, reason: Option<Reason>) -> Welcome {
    Welcome {
        accepted,
        protocol: None,
        session_id,
        audio_ticket: None,
        device_key: None,
        pc: None,
        reason,
        min_supported: None,
    }
}

/// Laço do socket de áudio: valida o `AUDIO_HELLO`, responde `WELCOME`,
/// conta blocos até fechar.
///
/// `conn`: dados compartilhados (ver [`AudioConn`]). `permit`: o permit
/// de "não autenticadas" da conexão — solto assim que o áudio é
/// validado (o socket de áudio não ocupa o limite).
///
/// Emite `AudioOpened` (após o `WELCOME`) e `AudioClosed` (ao sair do
/// laço); o chamador emite o `ConnectionClosed` do `conn_id`. Devolve o
/// motivo do fechamento.
pub(crate) async fn run_audio(
    conn: AudioConn,
    stream: &mut TcpStream,
    peer: SocketAddr,
    conn_id: u64,
    permit: OwnedSemaphorePermit,
    hello: &AudioHello,
) -> CloseReason {
    let AudioConn {
        ctx,
        events_tx,
        mut shutdown_rx,
        audio_block_max,
        ping_interval,
        liveness_timeout,
    } = conn;
    // 1) Ticket de uso único preso à sessão (comparação em tempo
    //    constante dentro do store; nunca logamos o ticket).
    let valid = {
        let mut tickets = ctx.tickets.lock().unwrap_or_else(|p| p.into_inner());
        tickets
            .redeem(&hello.session_id, &hello.ticket, now_secs())
            .is_ok()
    };

    // 2) Registra o canal na sessão (substitui um anterior, se houver) e
    //    pega os contadores na mesma seção crítica. Sessão inexistente
    //    ou que já não é a ativa → recusa uniforme.
    let (kick_tx, mut kick_rx) = oneshot::channel();
    let counters = if valid {
        ctx.sessions.set_audio(&hello.session_id, kick_tx)
    } else {
        None
    };
    let Some(counters) = counters else {
        tracing::debug!(conn_id, %peer, "áudio recusado: ticket/sessão inválidos");
        let _ = write_msg(
            stream,
            &Message::Welcome(audio_welcome(false, None, Some(Reason::BadAudioTicket))),
        )
        .await;
        return CloseReason::AuthDenied;
    };
    let session_id = hello.session_id.clone();

    // 3) `WELCOME` mínimo antes do laço.
    if let Err(e) = write_msg(
        stream,
        &Message::Welcome(audio_welcome(true, Some(session_id.clone()), None)),
    )
    .await
    {
        tracing::warn!(conn_id, %peer, error = %e, "falha ao enviar WELCOME de áudio; fechando");
        return CloseReason::ConnectionLost;
    }
    // Validado: o socket de áudio não ocupa mais o limite de
    // "não autenticadas".
    drop(permit);
    let _ = events_tx
        .send(ServerEvent::AudioOpened {
            session_id: session_id.clone(),
        })
        .await;

    // Leitor limitado a `audio_block_max`: bloco acima → `TooLarge`
    // antes de alocar o payload.
    let mut reader = FrameReader::with_max_payload(
        u32::try_from(audio_block_max).expect("audio_block_max cabe em u32"),
    );
    let mut buf = [0u8; READ_BUF_SIZE];

    // Keepalive (F1-T10): `PING` a cada `ping_interval`; sem `PONG` por
    // `liveness_timeout` → `ConnectionLost` (mesma regra da principal).
    let mut keepalive = Keepalive::new(liveness_timeout);
    let mut ticker = tokio::time::interval(ping_interval);
    // O 1º tick é imediato: descarta para o 1º `PING` sair após 1 ciclo.
    ticker.tick().await;

    // 4) Laço: `AUDIO` conta; `PING`→`PONG`; `BYE`→fecha; resto→ignora.
    //    Todo `break` atribui `reason` antes de sair.
    let reason: CloseReason;
    'audio: loop {
        let liveness = tokio::time::sleep_until(keepalive.deadline().into());
        tokio::pin!(liveness);
        tokio::select! {
            res = stream.read(&mut buf) => match res {
                Ok(0) => {
                    reason = CloseReason::PeerClosed;
                    break 'audio;
                }
                Ok(n) => match reader.push(&buf[..n]) {
                    Ok(frames) => for frame in frames {
                        match Message::from_frame(&frame) {
                            Ok(Message::Audio { pts_us, data }) => {
                                let bytes =
                                    u64::try_from(data.len()).expect("len() cabe em u64");
                                counters.audio_blocks.fetch_add(1, Ordering::Relaxed);
                                counters
                                    .audio_bytes
                                    .fetch_add(bytes, Ordering::Relaxed);
                                counters
                                    .last_audio_pts
                                    .store(pts_us, Ordering::Relaxed);
                            }
                            Ok(Message::Ping(v)) => {
                                if let Err(e) = write_msg(stream, &Message::Pong(v)).await {
                                    tracing::warn!(conn_id, error = %e, "falha ao enviar PONG no áudio");
                                    reason = CloseReason::ConnectionLost;
                                    break 'audio;
                                }
                            }
                            // Keepalive (F1-T10): só o `PONG` mantém vivo.
                            Ok(Message::Pong(v)) => {
                                keepalive.on_pong(v, mono_ms());
                            }
                            Ok(Message::Bye(_)) => {
                                reason = CloseReason::PeerClosed;
                                break 'audio;
                            }
                            Ok(_) => tracing::debug!(
                                conn_id,
                                frame_type = type_name(frame.msg_type),
                                "mensagem ignorada no socket de áudio"
                            ),
                            Err(e) => {
                                tracing::warn!(conn_id, error = %e, "frame inválido no socket de áudio; fechando");
                                reason = CloseReason::ProtocolError;
                                break 'audio;
                            }
                        }
                    }
                    Err(e) => {
                        // `TooLarge` (bloco > `audio_block_max`) ou magic
                        // inválido: só este socket cai.
                        tracing::warn!(conn_id, error = %e, "erro de protocolo no socket de áudio; fechando");
                        reason = CloseReason::ProtocolError;
                        break 'audio;
                    }
                },
                Err(e) => {
                    tracing::warn!(conn_id, error = %e, "erro de leitura no áudio");
                    reason = CloseReason::ConnectionLost;
                    break 'audio;
                }
            },
            // Keepalive (F1-T10): a cada `ping_interval` envia `PING`
            // (relógio monotônico em ms); se o liveness já estourou
            // (sem `PONG`), fecha com `ConnectionLost`.
            _ = ticker.tick() => {
                if keepalive.is_dead(std::time::Instant::now()) {
                    tracing::info!(conn_id, "sem PONG no áudio por {} ms; fechando", liveness_timeout.as_millis());
                    reason = CloseReason::ConnectionLost;
                    break 'audio;
                }
                if let Err(e) = write_msg(stream, &Message::Ping(mono_ms())).await {
                    tracing::warn!(conn_id, error = %e, "falha ao enviar PING no áudio");
                    reason = CloseReason::ConnectionLost;
                    break 'audio;
                }
            }
            // Keepalive (F1-T10): prazo exato do liveness (recalculado a
            // cada iteração; atualizado a cada `PONG`).
            _ = &mut liveness => {
                tracing::info!(conn_id, "liveness timeout no áudio ({} ms sem PONG); fechando", liveness_timeout.as_millis());
                reason = CloseReason::ConnectionLost;
                break 'audio;
            }
            // Kick: novo áudio da mesma sessão (sessão segue ativa →
            // `Replaced`) ou fim da sessão dona (já não é a ativa →
            // `PeerClosed`).
            _ = &mut kick_rx => {
                reason = if ctx.sessions.is_active(&session_id) {
                    CloseReason::Replaced
                } else {
                    CloseReason::PeerClosed
                };
                break 'audio;
            }
            _ = shutdown_rx.changed() => {
                reason = CloseReason::Shutdown;
                break 'audio;
            }
        }
    }

    let _ = events_tx
        .send(ServerEvent::AudioClosed {
            session_id,
            reason: reason.clone(),
        })
        .await;
    reason
}
