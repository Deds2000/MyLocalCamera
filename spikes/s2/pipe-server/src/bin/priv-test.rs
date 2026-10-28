// Diagnóstico rápido: usuário comum (sem elevar) consegue criar objeto em
// `Global\`? Isso depende de SeCreateGlobalPrivilege (padrão: Administrators,
// SYSTEM e serviços do SCM). Rodar: cargo run --release --bin priv-test

use windows::core::w;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Memory::{CreateFileMappingW, PAGE_READWRITE};
use windows::Win32::System::Threading::GetCurrentProcess;

fn main() {
    let cases = [
        ("Global\\S2PrivTest", w!("Global\\S2PrivTest")),
        ("Local\\S2PrivTest", w!("Local\\S2PrivTest")),
        ("S2PrivTestNoPrefix", w!("S2PrivTestNoPrefix")),
    ];
    for (label, name) in cases {
        match unsafe {
            CreateFileMappingW(GetCurrentProcess(), None, PAGE_READWRITE, 0, 4096, name)
        } {
            Ok(h) => {
                let _ = unsafe { CloseHandle(h) };
                println!("{label} => OK");
            }
            Err(e) => println!("{label} => ERR {e}"),
        }
    }
}
