//! Janela de aprovação USB (F1-T07): **15 s**, só loopback
//! (`127.0.0.1`/`::1`) e de **uso único**.
//!
//! Módulo **puro**: o relógio é injetado como `now: u64` (segundos) —
//! nenhum `Instant`/relógio global. Aberta por
//! `ServerHandle::open_usb_window()`; o primeiro `HELLO` **válido** de
//! peer loopback a consome (fecha) e o pareamento é aprovado com
//! `ApproveAlways` sem diálogo. O token de pareamento continua
//! obrigatório (token inválido não chega nem a consumir a janela).

use std::net::IpAddr;

/// Duração da janela de aprovação USB, em segundos.
pub const USB_WINDOW_SECS: u64 = 15;

/// Janela de aprovação USB: aberta por [`USB_WINDOW_SECS`] segundos a
/// partir de `open(now)` (limite **inclusivo**: aberta até
/// `now + USB_WINDOW_SECS`) e consumida pelo primeiro `HELLO` válido de
/// peer loopback.
#[derive(Debug, Default, Clone, Copy)]
pub struct UsbWindow {
    /// Janela aberta enquanto `now <= open_until`; `None` = fechada.
    open_until: Option<u64>,
}

impl UsbWindow {
    /// Cria a janela fechada.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Abre (ou reabre) a janela por [`USB_WINDOW_SECS`] segundos a
    /// partir de `now`.
    pub fn open(&mut self, now: u64) {
        self.open_until = Some(now.saturating_add(USB_WINDOW_SECS));
    }

    /// A janela está aberta e não expirada no instante `now`?
    #[must_use]
    pub fn is_open(&self, now: u64) -> bool {
        self.open_until.is_some_and(|until| now <= until)
    }

    /// Tenta consumir a janela: `true` **somente** se ela está aberta,
    /// não expirada e `peer` é loopback. Consome (fecha).
    ///
    /// Peer **não**-loopback não consome (a janela segue aberta para o
    /// próximo `HELLO` de loopback).
    #[must_use]
    pub fn try_consume(&mut self, peer: IpAddr, now: u64) -> bool {
        if self.is_open(now) && peer.is_loopback() {
            self.open_until = None;
            true
        } else {
            false
        }
    }

    /// Fecha uma janela já expirada (expiração **preguiçosa**: não há
    /// timer em segundo plano). Devolve `true` se ela estava aberta e
    /// expirou agora (o chamador emite o evento de fechamento).
    #[must_use]
    pub fn expire_if_stale(&mut self, now: u64) -> bool {
        match self.open_until {
            Some(until) if now > until => {
                self.open_until = None;
                true
            }
            _ => false,
        }
    }
}
