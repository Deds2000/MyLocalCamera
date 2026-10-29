//! Confiança: `pcId`, token de pareamento e dispositivos confiáveis.
//!
//! A `deviceKey` em claro existe apenas na memória, no momento do
//! pareamento (`add_device` devolve a chave uma única vez). O PC grava
//! **somente** o SHA-256 em hex, em `<data_dir>/trust.json`, com
//! escrita atômica e recuperação de arquivo corrompido.

pub mod approval;
mod atomic;
pub mod ratelimit;
pub mod secrets;
pub mod store;
pub mod usb_window;

pub use approval::{effective_auto_approve, ApprovalInfo, ApprovalManager, Busy};
pub use ratelimit::{AuthLimiter, ConnRateLimiter, Limiters};
pub use store::{TrustError, TrustStore, TrustedDevice, VerifyDevice};
pub use usb_window::{UsbWindow, USB_WINDOW_SECS};
