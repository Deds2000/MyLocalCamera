// DACL do pipe do spike S2.
//
// Concede acesso total (0x1F01FF = FILE_ALL_ACCESS) a:
//   1. o usuário atual (SID obtido do token do processo);
//   2. a conta NT AUTHORITY\LOCAL SERVICE (S-1-5-19) — conta do serviço
//      FrameServer, confirmada na máquina com `sc qc FrameServer` e em
//      https://learn.microsoft.com (serviço "Windows Camera Frame Server").
//
// A SD é gerada de um SDDL via ConvertStringSecurityDescriptorToSecurityDescriptorW
// (advapi32; revisão SDDL 1, como documentado em
// https://learn.microsoft.com/en-us/windows/win32/secauthz/security-descriptor-string-format).
//
// SDDL final: O:<user>G:<user>D:(A;;0x1F01FF;;;<user>)(A;;0x1F01FF;;;S-1-5-19)
// Nota: nesta máquina o parser rejeita SDDL sem as seções owner/group (erro 87);
// verificado em src/bin/sddl-test.rs.

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
use windows::Win32::Security::{
    GetTokenInformation, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
    TokenUser,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// SID bem conhecido NT AUTHORITY\LOCAL SERVICE
/// (https://learn.microsoft.com/en-us/windows/win32/secapi/well-known-sids).
pub const LOCAL_SERVICE_SID: &str = "S-1-5-19";

/// Lê o SID do usuário atual a partir do token do processo.
pub fn current_user_sid() -> Result<String, String> {
    unsafe {
        let proc = GetCurrentProcess();
        let mut token = HANDLE::default();
        OpenProcessToken(proc, TOKEN_QUERY, &mut token)
            .map_err(|e| format!("OpenProcessToken: {e}"))?;

        // 1ª chamada: descobre o tamanho.
        let mut needed: u32 = 0;
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut needed);
        if needed == 0 {
            CloseHandle(token).ok();
            return Err("GetTokenInformation: tamanho 0".into());
        }
        let mut buf = vec![0u8; needed as usize];
        GetTokenInformation(
            token,
            TokenUser,
            Some(buf.as_mut_ptr().cast()),
            needed,
            &mut needed,
        )
        .map_err(|e| format!("GetTokenInformation: {e}"))?;
        CloseHandle(token).ok();

        let tu = &*(buf.as_ptr() as *const TOKEN_USER);
        sid_to_string(tu.User.Sid.0)
    }
}

/// Converte um PSID para a forma string ("S-1-5-21-...-1001"), lendo os bytes
/// no layout documentado do struct SID (winnt.h).
unsafe fn sid_to_string(sid: *const core::ffi::c_void) -> Result<String, String> {
    if sid.is_null() {
        return Err("SID nula".into());
    }
    let base = sid as *const u8;
    let revision = base.read();
    let count = base.add(1).read() as usize;
    // IdentifierAuthority tem 6 bytes; no formato string vira UM número
    // (big-endian; para autoridade NT os 5 primeiros bytes são 0).
    let mut authority: u64 = 0;
    for i in 0..6 {
        authority = (authority << 8) | u64::from(base.add(2 + i).read());
    }
    let mut s = format!("S-{revision}-{authority}");
    for i in 0..count {
        let bytes = std::slice::from_raw_parts(base.add(8 + 4 * i), 4);
        s.push_str(&format!("-{}", u32::from_le_bytes(bytes.try_into().unwrap())));
    }
    Ok(s)
}

/// Cria a SD (self-relative) para o pipe: DACL com os dois ACEs acima.
/// O `Vec<u8>` retornado mantém a SD viva; o SECURITY_ATTRIBUTES aponta para ela.
pub fn pipe_security_attributes(user_sid: &str) -> Result<(Vec<u8>, SECURITY_ATTRIBUTES), String> {
    // Formato exigido pelo parser de SDDL desta máquina (verificado empiricamente
    // em src/bin/sddl-test.rs): sem as seções de owner/group o parser rejeita a
    // string com ERROR_INVALID_PARAMETER (87). Owner/group = usuário que cria o
    // pipe (o processo criador sempre pode ser dono do objeto).
    let sddl = format!(
        "O:{user_sid}G:{user_sid}D:(A;;0x1F01FF;;;{user_sid})(A;;0x1F01FF;;;{LOCAL_SERVICE_SID})"
    );
    // A crate windows (0.62) exige PCWSTR; convertemos para UTF-16 nulo-terminado.
    let sddl_wide: Vec<u16> = sddl.encode_utf16().chain(std::iter::once(0)).collect();
    let sddl_pcwstr = windows::core::PCWSTR(sddl_wide.as_ptr());
    unsafe {
        let mut sd: PSECURITY_DESCRIPTOR = PSECURITY_DESCRIPTOR(std::ptr::null_mut());
        let mut size: u32 = 0;
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl_pcwstr,
            1, // SDDL_REVISION
            &mut sd,
            Some(&mut size),
        )
        .map_err(|e| format!("ConvertStringSecurityDescriptorToSecurityDescriptorW({sddl}): {e}"))?;
        if sd.is_invalid() {
            return Err("SD nula".into());
        }
        // Self-relative: os dados estão no próprio buffer apontado por `sd`.
        let mut bytes = vec![0u8; size as usize];
        core::ptr::copy_nonoverlapping(sd.0.cast::<u8>(), bytes.as_mut_ptr(), size as usize);
        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: bytes.as_ptr() as *mut _,
            bInheritHandle: false.into(),
        };
        Ok((bytes, sa))
    }
}
