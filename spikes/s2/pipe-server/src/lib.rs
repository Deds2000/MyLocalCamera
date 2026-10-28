// Spike S2 — crate descartável (fora do workspace principal do repositório).
// Módulos compartilhados entre o servidor de pipe (main) e o
// smoke-client (src/bin/smoke-client.rs).
pub mod frame;
pub mod pipe;
pub mod protocol;
pub mod sec;
pub mod stats;
