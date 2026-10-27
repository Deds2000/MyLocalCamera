//! Utilitários compartilhados por testes e ferramentas.
use std::net::TcpListener;

/// Devolve uma porta TCP livre em 127.0.0.1 (a porta fica livre ao retornar).
pub fn free_port() -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind");
    l.local_addr().expect("addr").port()
}

/// Inicializa logs para testes (idempotente). Use RUST_LOG=debug para ver.
pub fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn free_port_is_bindable() {
        let p = free_port();
        assert!(p > 0);
        assert!(std::net::TcpListener::bind(("127.0.0.1", p)).is_ok());
    }
}
