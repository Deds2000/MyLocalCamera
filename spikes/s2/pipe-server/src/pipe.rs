// Criação do named pipe do spike S2 com DACL.
//
// Nome:  \\.\pipe\LocalCameraSpike
// Modo:  PIPE_ACCESS_DUPLEX (servidor envia quadros; cliente envia acks)
//        + PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE (cada WriteFile = 1 mensagem)
//        + PIPE_WAIT + PIPE_REJECT_REMOTE_CLIENTS
//
// Valores de constantes confirmados no header do SDK instalado
// (Windows Kits 10.0.22621.0, um\winbase.h) e no docs MSDN CreateNamedPipe:
//   PIPE_REJECT_REMOTE_CLIENTS = 0x00000008.
//
// Buffers de 3 MiB (1 mensagem = 3 110 428 bytes; o buffer precisa caber a
// mensagem inteira em pipe de modo mensagem).

use windows::Win32::Foundation::HANDLE;
use windows::Win32::Security::SECURITY_ATTRIBUTES;
use windows::Win32::Storage::FileSystem::{PIPE_ACCESS_DUPLEX, FILE_FLAGS_AND_ATTRIBUTES};
use windows::Win32::System::Pipes::{
    CreateNamedPipeW, PIPE_READMODE_MESSAGE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_MESSAGE,
    PIPE_WAIT,
};

pub const PIPE_NAME: &str = "\\\\.\\pipe\\LocalCameraSpike";
// (O macro w! só aceita strings literais; usamos o literal inline abaixo.)
const BUFFER_BYTES: u32 = 3 * 1024 * 1024;

pub fn create_pipe(sa: Option<&SECURITY_ATTRIBUTES>) -> Result<HANDLE, String> {
    unsafe {
        let h = CreateNamedPipeW(
            windows::core::w!(r"\\.\pipe\LocalCameraSpike"),
            FILE_FLAGS_AND_ATTRIBUTES(PIPE_ACCESS_DUPLEX.0),
            PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,          // nMaxInstances
            BUFFER_BYTES,
            BUFFER_BYTES,
            0,          // nDefaultTimeOut
            sa.map(|p| p as *const _),
        );
        if h.is_invalid() {
            let e = windows::Win32::Foundation::GetLastError();
            Err(format!("CreateNamedPipeW falhou: win32={}", e.0))
        } else {
            Ok(h)
        }
    }
}
