// bootloader/src/time.rs
//
// Time Stamp Counter (TSC) measurement & calibration.
// Spec: Intel SDM Vol 3B §17.17 (RDTSC instruction).

use crate::uefi_raw::EfiStatus;

/// Read the 64-bit Time Stamp Counter.
#[inline(always)]
pub fn rdtsc() -> u64 {
    let lo: u32;
    let hi: u32;
    // SAFETY: `rdtsc` instruction is available on all x86_64 CPUs (Pentium+).
    // Loads low 32 bits into EAX, high 32 bits into EDX.
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("eax") lo,
            out("edx") hi,
            options(nomem, nostack, preserves_flags)
        );
    }
    ((hi as u64) << 32) | (lo as u64)
}

/// Calibrate the TSC against the UEFI firmware timer (`stall`).
///
/// We stall for 10,000 microseconds (10 ms) and count TSC ticks elapsed.
/// Returns: TSC ticks per microsecond (MHz equivalent).
///
/// # Safety
/// `stall` must be a valid UEFI Stall function pointer.
pub unsafe fn calibrate_tsc(
    stall: unsafe extern "efiapi" fn(usize) -> EfiStatus,
) -> u64 {
    const STALL_US: usize = 10_000; // 10 ms calibration interval
    let t0 = rdtsc();
    // SAFETY: caller guarantees stall is valid firmware function.
    unsafe {
        stall(STALL_US);
    }
    let t1 = rdtsc();

    let delta = t1.saturating_sub(t0);
    let ticks_per_us = delta / (STALL_US as u64);

    if ticks_per_us == 0 {
        1 // prevent divide-by-zero on emulators with non-advancing TSC
    } else {
        ticks_per_us
    }
}

/// Convert TSC delta ticks to elapsed microseconds.
#[inline(always)]
pub fn ticks_to_us(ticks: u64, ticks_per_us: u64) -> u64 {
    if ticks_per_us == 0 {
        ticks
    } else {
        ticks / ticks_per_us
    }
}
