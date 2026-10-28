//! Working set do próprio processo (Windows).
//! Nota: no Win11 24H2+ a export em psapi.dll foi removida; usar kernel32 (K32...).
//! Em 64 bits, PROCESS_MEMORY_COUNTERS tem campos SIZE_T (u64): 2×u32 + 8×u64 = 72 B.

#![allow(non_snake_case)]

use std::mem::size_of;
use std::os::windows::raw::HANDLE;

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct PROCESS_MEMORY_COUNTERS {
    cb: u32,
    page_fault_count: u32,
    peak_working_set_size: u64,
    working_set_size: u64,
    quota_peak_paged_pool_usage: u64,
    quota_paged_pool_usage: u64,
    quota_peak_non_paged_pool_usage: u64,
    quota_non_paged_pool_usage: u64,
    pagefile: u64,
    peak_pagefile: u64,
}

extern "system" {
    fn GetCurrentProcess() -> HANDLE;
}

#[link(name = "kernel32")]
extern "system" {
    fn K32GetProcessMemoryInfo(
        hprocess: HANDLE,
        ppsmemuinfo: *mut PROCESS_MEMORY_COUNTERS,
        cb: u32,
    ) -> i32;
}

pub fn working_set_mb() -> u64 {
    unsafe {
        let h = GetCurrentProcess(); // pseudo-handle, sempre válido p/ este processo
        let mut m = PROCESS_MEMORY_COUNTERS::default();
        m.cb = size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        let ok = K32GetProcessMemoryInfo(h, &mut m, m.cb) != 0;
        if ok {
            m.working_set_size / 1024 / 1024
        } else {
            0
        }
    }
}
