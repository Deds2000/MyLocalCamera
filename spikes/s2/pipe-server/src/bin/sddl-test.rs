// Diagnóstico: testa variações de SDDL contra
// ConvertStringSecurityDescriptorToSecurityDescriptorW (advapi32) para isolar
// qual parte do SDDL do pipe é rejeitada com ERROR_INVALID_PARAMETER (87).
//
// Parte A: FFI "cru" (sem a crate windows) para isolar binding/codegen.
// Parte B: controle com ConvertStringSidToSidW (mesma DLL, mesma chamada de
// marshalling de PCWSTR) para ver se o problema é geral ou específico da SDDL.
// Rodar: cargo run --release --bin sddl-test

use std::ffi::c_void;

#[link(name = "advapi32", kind = "raw-dylib")]
extern "system" {
    fn ConvertStringSecurityDescriptorToSecurityDescriptorW(
        sddl: *const u16,
        revision: u32,
        sd: *mut *mut c_void,
        size: *mut u32,
    ) -> i32;
    fn ConvertStringSidToSidW(sid: *const u16, out_sid: *mut *mut c_void) -> i32;
}

#[link(name = "kernel32", kind = "raw-dylib")]
extern "system" {
    fn GetLastError() -> u32;
}

unsafe fn test_sddl(t: &str) {
    let wide: Vec<u16> = t.encode_utf16().chain(std::iter::once(0)).collect();
    let mut sd: *mut c_void = std::ptr::null_mut();
    let mut size: u32 = 0;
    let r = ConvertStringSecurityDescriptorToSecurityDescriptorW(
        wide.as_ptr(),
        1,
        &mut sd,
        &mut size,
    );
    let err = if r == 0 { GetLastError() } else { 0 };
    println!("[A raw-ffi] {t} => ok={} err={} size={}", r != 0, err, size);
}

fn main() {
    unsafe {
        println!("=== A) FFI cru ===");
        test_sddl("(D;A;;GA;;;SY)");
        test_sddl("(D;A;;GA;;;BA)");
        test_sddl("O:SYG:SYD:(A;;GA;;;BA)");

        println!("=== B0) quais seções são obrigatórias? ===");
        for t in [
            "O:SYG:SYD:(A;;GA;;;BA)",
            "O:SYD:(A;;GA;;;BA)",
            "G:SYD:(A;;GA;;;BA)",
            "O:G:D:(A;;GA;;;BA)",
            "O:BAG:SYD:(A;;0x1F01FF;;;S-1-5-21-4125282749-3283497435-2269617215-1001)(A;;0x1F01FF;;;S-1-5-19)",
        ] {
            test_sddl(t);
        }

        println!("=== B) controle: ConvertStringSidToSidW ===");
        for s in ["S-1-5-19", "S-1-5-21-4125282749-3283497435-2269617215-1001"] {
            let wide: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
            let mut sid: *mut c_void = std::ptr::null_mut();
            let r = ConvertStringSidToSidW(wide.as_ptr(), &mut sid);
            let err = if r == 0 { GetLastError() } else { 0 };
            println!("[B raw-ffi] {s} => ok={} err={}", r != 0, err);
        }

        println!("=== C) crate windows (mesmo caminho do servidor) ===");
        use windows::core::PCWSTR;
        use windows::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW as crate_conv;
        use windows::Win32::Security::PSECURITY_DESCRIPTOR;
        let tests = [
            "(D;A;;GA;;;SY)",
            "O:SYG:SYD:(A;;GA;;;BA)",
        ];
        for t in tests {
            let wide: Vec<u16> = t.encode_utf16().chain(std::iter::once(0)).collect();
            let pcw = PCWSTR(wide.as_ptr());
            let mut sd = PSECURITY_DESCRIPTOR(std::ptr::null_mut());
            let mut size: u32 = 0;
            let r = crate_conv(pcw, 1, &mut sd, Some(&mut size));
            match r {
                Ok(()) => println!("[C crate] {t} => OK size={size}"),
                Err(e) => println!("[C crate] {t} => ERR {e}"),
            }
        }
    }
}
