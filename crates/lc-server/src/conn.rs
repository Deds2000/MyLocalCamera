//! Laço de aceitação e laço por conexão: handshake (`AwaitHello` →
//! `WELCOME`) e laço de sessão (`Active`).

mod accept;
mod connection;
mod session;

pub(crate) use accept::accept_loop;
pub(crate) use accept::{ConnShared, KeepaliveTiming};
