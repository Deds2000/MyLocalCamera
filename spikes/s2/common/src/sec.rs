//! DACL para objetos criados pelo servidor (pipe/seção/evento):
//! concede `mask` ao **usuário atual** e à conta do Frame Server
//! (**`NT AUTHORITY\LocalService`**, confirmada em docs/spikes/S2.md).
//!
//! Padrão documentado: "Creating a Discretionary Access Control List"
//! (Microsoft Learn, Win32 security).

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{
    AddAccessAllowedAce, CreateWellKnownSid, GetTokenInformation, InitializeAcl,
    InitializeSecurityDescriptor, SetSecurityDescriptorDacl, WinLocalServiceSid, ACL,
    ACL_REVISION, PSID, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, SECURITY_DESCRIPTOR, SID,
    TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

const SECURITY_DESCRIPTOR_REVISION: u32 = 1;

/// Buffer proprietário do SECURITY_DESCRIPTOR + DACL + 2 ACEs.
pub struct SecBuf {
    pub data: Vec<u8>,
    pub sa: SECURITY_ATTRIBUTES,
}

fn sid_len(sid: &SID) -> usize {
    8 + sid.SubAuthorityCount as usize * 4
}

/// SID do usuário atual (via token do processo).
fn current_user_sid() -> Result<Vec<u8>, String> {
    let mut token = HANDLE::default();
    unsafe {
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)
            .map_err(|e| format!("OpenProcessToken: {e}"))?;
    }
    let mut len: u32 = 0;
    if unsafe { GetTokenInformation(token, TokenUser, None, 0, &mut len) }.is_ok() || len == 0 {
        return Err("GetTokenInformation: tamanho 0".into());
    }
    let mut buf = vec![0u8; len as usize];
    let ret;
    unsafe {
        ret = GetTokenInformation(
            token,
            TokenUser,
            Some(buf.as_mut_ptr() as *mut _),
            len,
            &mut len,
        );
        let _ = CloseHandle(token);
    }
    ret.map_err(|e| format!("GetTokenInformation: {e}"))?;
    let tu: TOKEN_USER = unsafe { core::ptr::read_unaligned(buf.as_ptr() as *const TOKEN_USER) };
    let sid = unsafe { &*(tu.User.Sid.0 as *const SID) };
    let n = sid_len(sid);
    let mut out = vec![0u8; n];
    unsafe { core::ptr::copy_nonoverlapping(sid as *const SID as *const u8, out.as_mut_ptr(), n) };
    Ok(out)
}

/// SID bem conhecida do `NT AUTHORITY\LocalService` (conta do serviço FrameServer).
fn local_service_sid() -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; 64]; // SID bem conhecida cabe em poucos bytes
    let mut size: u32 = buf.len() as u32;
    unsafe {
        CreateWellKnownSid(
            WinLocalServiceSid,
            None,
            Some(PSID(buf.as_mut_ptr() as *mut _)),
            &mut size,
        )
        .map_err(|e| format!("CreateWellKnownSid: {e}"))?;
    }
    buf.truncate(size as usize);
    Ok(buf)
}

/// Constrói o SECURITY_DESCRIPTOR com DACL de 2 ACEs (usuário atual + LocalService).
///
/// `mask` = direitos específicos do tipo de objeto:
/// - pipe: `GENERIC_READ | GENERIC_WRITE`
/// - seção: `SECTION_MAP_READ | SECTION_MAP_WRITE`
/// - evento: `EVENT_MODIFY_STATE | SYNCHRONIZE`
pub fn build_sd(mask: u32) -> Result<SecBuf, String> {
    let user_sid = current_user_sid()?;
    let ls_sid = local_service_sid()?;

    let sd_size = core::mem::size_of::<SECURITY_DESCRIPTOR>();
    let dacl_hdr = core::mem::size_of::<ACL>(); // cabeçalho (DACL estende ACL)
    let ace1 = 4 + 4 + user_sid.len(); // ACE header + SidStart
    let ace2 = 4 + 4 + ls_sid.len();
    let dacl_size = dacl_hdr + ace1 + ace2;
    let total = sd_size + dacl_size;

    let mut data = vec![0u8; total];
    unsafe {
        let sd = data.as_mut_ptr() as *mut SECURITY_DESCRIPTOR;
        InitializeSecurityDescriptor(PSECURITY_DESCRIPTOR(sd as *mut _), SECURITY_DESCRIPTOR_REVISION)
            .map_err(|e| format!("InitializeSecurityDescriptor: {e}"))?;

        let dacl = data.as_mut_ptr().add(sd_size) as *mut ACL;
        InitializeAcl(dacl, dacl_size as u32, ACL_REVISION)
            .map_err(|e| format!("InitializeAcl: {e}"))?;

        AddAccessAllowedAce(dacl, ACL_REVISION, mask, PSID(user_sid.as_ptr() as *mut _))
            .map_err(|e| format!("AddAccessAllowedAce (usuario): {e}"))?;
        AddAccessAllowedAce(dacl, ACL_REVISION, mask, PSID(ls_sid.as_ptr() as *mut _))
            .map_err(|e| format!("AddAccessAllowedAce (LocalService): {e}"))?;

        SetSecurityDescriptorDacl(
            PSECURITY_DESCRIPTOR(sd as *mut _),
            true,
            Some(dacl as *const ACL),
            false,
        )
        .map_err(|e| format!("SetSecurityDescriptorDacl: {e}"))?;
    }

    let sa = SECURITY_ATTRIBUTES {
        nLength: core::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: data.as_ptr() as *mut core::ffi::c_void,
        bInheritHandle: false.into(),
    };
    Ok(SecBuf { data, sa })
}

fn sid_to_sstr(b: &[u8]) -> String {
    let sid = unsafe { &*(b.as_ptr() as *const SID) };
    let subs = unsafe {
        core::slice::from_raw_parts(sid.SubAuthority.as_ptr(), sid.SubAuthorityCount as usize)
    };
    let mut s = format!("S-1-{}", sid.IdentifierAuthority.Value[5]);
    for v in subs {
        s.push('-');
        s.push_str(&v.to_string());
    }
    s
}

/// Descreve os SIDs usados na DACL (para registrar no S2.md).
pub fn describe_sids() -> Result<String, String> {
    let user = current_user_sid()?;
    let ls = local_service_sid()?;
    Ok(format!(
        "usuario={} | localService={} (NT AUTHORITY\\LocalService, conta do FrameServer)",
        sid_to_sstr(&user),
        sid_to_sstr(&ls)
    ))
}
