// bootloader/src/main.rs
//
// MyOS UEFI Bootloader — Milestone B1
// ====================================
// Minimal UEFI application. Zero external crate dependencies.
// Prints a banner via UEFI ConOut, stalls 5 s, returns to firmware.
//
// BUILD:
//   cargo build -p bootloader --target x86_64-unknown-uefi
//
// SPEC REFERENCES:
//   UEFI 2.10 §4.3   — EFI_SYSTEM_TABLE
//   UEFI 2.10 §12.4  — EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL
//   UEFI 2.10 §7.4.9 — EFI_BOOT_SERVICES.Stall
//   UEFI 2.10 §2.3.1 — x86_64 calling convention (Microsoft x64 / "efiapi")
//   PE/COFF Spec §2.3 — PE32+ image format required by UEFI

#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)] // every unsafe block must have a SAFETY comment

mod uefi_raw;
use uefi_raw::{
    EfiHandle, EfiStatus, SystemTable,
    EFI_SUCCESS, EFI_ATTR_WHITE_ON_BLACK,
    println as uefi_println, stall_fn,
};

// ── Entry point ──────────────────────────────────────────────────────────────
//
// The x86_64-unknown-uefi Rust target configures the LLD linker to look for
// a PE32+ entry point symbol named `efi_main`.  We export it with
// `#[no_mangle]` and `extern "efiapi"` (Microsoft x64 ABI).
//
// UEFI firmware calls us with (UEFI Spec §4.1):
//   RCX = EFI_HANDLE ImageHandle   — handle to this loaded image
//   RDX = EFI_SYSTEM_TABLE *       — pointer to the firmware system table
//
// We MUST return an EFI_STATUS. SUCCESS causes the boot manager to continue;
// any error may cause a reset depending on the firmware.
#[no_mangle]
pub extern "efiapi" fn efi_main(
    _image: EfiHandle,          // RCX: our loaded-image handle (unused in B1)
    st:     *mut SystemTable,   // RDX: pointer to EFI_SYSTEM_TABLE
) -> EfiStatus {

    // SAFETY: `st` is the EFI_SYSTEM_TABLE* passed by the UEFI firmware.
    // The firmware guarantees it is non-null, properly aligned, and valid
    // for the entire pre-ExitBootServices phase.
    unsafe { boot(st) }
}

/// Main boot logic — separated so the entry point function is minimal.
///
/// # Safety
/// `st` must be a valid EFI_SYSTEM_TABLE pointer from the firmware.
unsafe fn boot(st: *mut SystemTable) -> EfiStatus {
    // ── Dereference the system table ─────────────────────────────────────
    // SAFETY: firmware guarantees st is valid and not null.
    let cout = unsafe { (*st).con_out };
    let boot_services = unsafe { (*st).boot_services };

    // ── Set white-on-black text attribute ────────────────────────────────
    // UEFI Spec §12.4.6 — SetAttribute(This, Attribute)
    // EFI_ATTR_WHITE_ON_BLACK = 0x0F (bright white fg, black bg).
    //
    // SAFETY: cout is firmware-owned and valid through Boot Services phase.
    unsafe {
        ((*cout).set_attribute)(cout, EFI_ATTR_WHITE_ON_BLACK);
    }

    // ── Clear screen ─────────────────────────────────────────────────────
    // UEFI Spec §12.4.7 — ClearScreen(This)
    //
    // SAFETY: same as above.
    unsafe {
        ((*cout).clear_screen)(cout);
    }

    // ── Print banner ─────────────────────────────────────────────────────
    // All text is ASCII (codepoints 0x00–0x7F), so the u8→u16 cast in
    // uefi_raw::print is correct. \r\n is used for CRLF (UEFI Spec §12.4.3).
    //
    // SAFETY: cout is valid; uefi_raw::println is safe given a valid cout.
    unsafe {
        uefi_println(cout, "");
        uefi_println(cout, "  +-----------------------------------------+");
        uefi_println(cout, "  |    MyOS Bootloader  --  Milestone B1    |");
        uefi_println(cout, "  |    Zero-dependency UEFI PE32+ binary    |");
        uefi_println(cout, "  |    Target : x86_64-unknown-uefi         |");
        uefi_println(cout, "  |    Rust   : 1.99.0 (stable)             |");
        uefi_println(cout, "  +-----------------------------------------+");
        uefi_println(cout, "");
        uefi_println(cout, "  Firmware ConOut (SimpleTextOutput): OK");
        uefi_println(cout, "  EFI_SYSTEM_TABLE: accessed");
        uefi_println(cout, "  EFI_BOOT_SERVICES.Stall: about to call...");
        uefi_println(cout, "");
        uefi_println(cout, "  Stalling 5 seconds (readable in VirtualBox)...");
    }

    // ── Stall 5 seconds ──────────────────────────────────────────────────
    // EFI_BOOT_SERVICES.Stall(Microseconds) — UEFI Spec §7.4.9.
    // The function busy-waits inside firmware; it does not yield.
    // We read the function pointer from the Boot Services table at offset 248.
    //
    // SAFETY: boot_services is valid (firmware guarantees it).
    //         stall_fn() reads offset 248 which maps to Stall per UEFI Spec §7.1.
    let stall = unsafe { stall_fn(boot_services as *const _) };
    // SAFETY: stall is a valid function pointer obtained from firmware.
    unsafe { stall(5_000_000); } // 5 000 000 µs = 5 seconds

    // ── Done ─────────────────────────────────────────────────────────────
    // SAFETY: cout is still valid (we have not called ExitBootServices).
    unsafe {
        uefi_println(cout, "");
        uefi_println(cout, "  Done. B1 complete. Returning to firmware.");
    }

    EFI_SUCCESS
    // Returning SUCCESS: the boot manager will show its own menu or reset,
    // depending on firmware policy (UEFI Spec §6.1).
}

// ── Panic handler ────────────────────────────────────────────────────────────
//
// In a no_std binary the compiler requires a #[panic_handler].
// At boot time we cannot safely call ConOut (it might not be set up),
// so we just halt the CPU in a loop. Future milestones will add a serial
// log dump here before halting.
//
// `cli` + `hlt` in a loop is correct: on x86_64, NMIs can still wake the
// CPU, so we loop back to `hlt` after each one (UEFI pre-ExitBootServices,
// NMIs are suppressed by firmware on most hardware, but we loop defensively).
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        // SAFETY: `hlt` is valid in any privilege level on x86_64.
        // On UEFI (long mode, CPL 0), `cli` and `hlt` are always permitted.
        unsafe {
            core::arch::asm!(
                "cli",
                "hlt",
                options(nomem, nostack, preserves_flags)
            );
        }
    }
}
