// bootloader/src/uefi_raw.rs
//
// Raw UEFI type definitions — zero external crate dependencies.
//
// Every struct layout here is derived directly from the UEFI Specification
// Version 2.10. Field offsets are cited per spec section.
//
// WHY INLINE INSTEAD OF A CRATE?
//   The uefi / uefi-macros crates pull in proc-macro2, quote, syn — all of
//   which compile build-script binaries on the host. Windows Application
//   Control (WDAC/AppLocker) blocks unsigned executables created in temp
//   directories. By defining only the types we actually use, we produce
//   no host-native binaries at all.
//
// CALLING CONVENTION
//   UEFI Spec §2.3.1 mandates the Microsoft x64 calling convention for all
//   UEFI interfaces on x86_64. Rust spells this `extern "efiapi"`.
//   On x86_64-unknown-uefi, `efiapi` == MS x64: RCX, RDX, R8, R9 for
//   the first four integer/pointer arguments, XMM0-3 for floats, 32-byte
//   shadow store allocated by the caller.

#![allow(dead_code)]

// ── Primitive types (UEFI Spec Appendix A / §2.3.1) ─────────────────────────

/// UINTN — natural machine-word unsigned integer. 8 bytes on x86_64.
pub type EfiStatus = usize;
/// Opaque firmware handle (void *).
pub type EfiHandle = *mut core::ffi::c_void;
/// UCS-2 character (16-bit, little-endian on x86_64).
pub type Char16  = u16;

pub const EFI_SUCCESS: EfiStatus = 0;

// ── EFI_TABLE_HEADER (UEFI Spec §4.2) ───────────────────────────────────────
//
// Every UEFI table starts with this header.
// Field layout (all offsets from start of header):
//   offset  0 : Signature   u64   8 bytes
//   offset  8 : Revision    u32   4 bytes
//   offset 12 : HeaderSize  u32   4 bytes
//   offset 16 : CRC32       u32   4 bytes
//   offset 20 : Reserved    u32   4 bytes
//   total     : 24 bytes, alignment 8
#[repr(C)]
pub struct EfiTableHeader {
    pub signature:   u64,
    pub revision:    u32,
    pub header_size: u32,
    pub crc32:       u32,
    pub reserved:    u32,
}

// ── EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL (UEFI Spec §12.4) ───────────────────────
//
// Function pointer slots in declaration order (each 8 bytes on x86_64):
//   offset  0 : Reset
//   offset  8 : OutputString   ← we call this
//   offset 16 : TestString
//   offset 24 : QueryMode
//   offset 32 : SetMode
//   offset 40 : SetAttribute   ← we call this
//   offset 48 : ClearScreen    ← we call this
//   offset 56 : SetCursorPosition
//   offset 64 : EnableCursor
//   offset 72 : Mode (data pointer)
//
// Prototypes from UEFI Spec §12.4.1–§12.4.9:
//   OutputString(This, String) -> EFI_STATUS
//   SetAttribute(This, Attribute) -> EFI_STATUS
//   ClearScreen(This) -> EFI_STATUS
#[repr(C)]
pub struct SimpleTextOutput {
    pub reset:
        unsafe extern "efiapi" fn(*mut Self, extended_verification: bool) -> EfiStatus,
    pub output_string:
        unsafe extern "efiapi" fn(*mut Self, string: *const Char16) -> EfiStatus,
    pub test_string:
        unsafe extern "efiapi" fn(*mut Self, string: *const Char16) -> EfiStatus,
    pub query_mode:
        unsafe extern "efiapi" fn(*mut Self, mode: usize, columns: *mut usize, rows: *mut usize) -> EfiStatus,
    pub set_mode:
        unsafe extern "efiapi" fn(*mut Self, mode: usize) -> EfiStatus,
    pub set_attribute:
        unsafe extern "efiapi" fn(*mut Self, attribute: usize) -> EfiStatus,
    pub clear_screen:
        unsafe extern "efiapi" fn(*mut Self) -> EfiStatus,
    pub set_cursor_position:
        unsafe extern "efiapi" fn(*mut Self, column: usize, row: usize) -> EfiStatus,
    pub enable_cursor:
        unsafe extern "efiapi" fn(*mut Self, visible: bool) -> EfiStatus,
    pub mode: *mut core::ffi::c_void,
}

// EFI text attribute constants (UEFI Spec §12.4.6, Table 12-9)
// Attribute = (background << 4) | foreground
pub const EFI_LIGHTGRAY:  usize = 0x07; // white foreground, black bg
pub const EFI_WHITE:      usize = 0x0F; // bright white fg, black bg
pub const EFI_BLACK_BG:   usize = 0x00; // black background (shifted: 0x00 << 4)
pub const EFI_ATTR_WHITE_ON_BLACK: usize = EFI_WHITE | (EFI_BLACK_BG << 4); // = 0x0F

// ── EFI_BOOT_SERVICES (UEFI Spec §7.1) — partial ────────────────────────────
//
// We only need Stall. Its position in the Boot Services table:
//   The Boot Services table has EFI_TABLE_HEADER first (24 bytes),
//   then many function pointers. Stall is the 27th function pointer
//   (UEFI Spec §7.1 Table 7-1, in order):
//
//   Slot  0  (offset 24): RaiseTPL
//   Slot  1  (offset 32): RestoreTPL
//   Slot  2  (offset 40): AllocatePages
//   Slot  3  (offset 48): FreePages
//   Slot  4  (offset 56): GetMemoryMap
//   Slot  5  (offset 64): AllocatePool
//   Slot  6  (offset 72): FreePool
//   Slot  7  (offset 80): CreateEvent
//   Slot  8  (offset 88): SetTimer
//   Slot  9  (offset 96): WaitForEvent
//   Slot 10  (offset 104): SignalEvent
//   Slot 11  (offset 112): CloseEvent
//   Slot 12  (offset 120): CheckEvent
//   Slot 13  (offset 128): InstallProtocolInterface
//   Slot 14  (offset 136): ReinstallProtocolInterface
//   Slot 15  (offset 144): UninstallProtocolInterface
//   Slot 16  (offset 152): HandleProtocol
//   Slot 17  (offset 160): Reserved
//   Slot 18  (offset 168): RegisterProtocolNotify
//   Slot 19  (offset 176): LocateHandle
//   Slot 20  (offset 184): LocateDevicePath
//   Slot 21  (offset 192): InstallConfigurationTable
//   Slot 22  (offset 200): LoadImage
//   Slot 23  (offset 208): StartImage
//   Slot 24  (offset 216): Exit
//   Slot 25  (offset 224): UnloadImage
//   Slot 26  (offset 232): ExitBootServices
//   Slot 27  (offset 240): GetNextMonotonicCount
//   Slot 28  (offset 248): Stall   ← HERE
//
// Stall prototype (UEFI Spec §7.4.9):
//   EFI_STATUS Stall(IN UINTN Microseconds)
//
// Rather than define all 44+ slots, we use a raw pointer and manual offset.
// This is explicitly marked unsafe; the invariant is stated in every caller.
pub type StallFn = unsafe extern "efiapi" fn(microseconds: usize) -> EfiStatus;

/// Returns a pointer to the Stall function from a raw boot services table pointer.
///
/// # Safety
/// `boot_services` must point to a valid EFI_BOOT_SERVICES table as described
/// in UEFI Spec §7.1. The table must remain valid for the duration of the call.
/// This is guaranteed while we are in the UEFI Boot Services execution phase
/// (before ExitBootServices).
pub unsafe fn stall_fn(boot_services: *const core::ffi::c_void) -> StallFn {
    // SAFETY: boot_services points to EFI_BOOT_SERVICES. Stall is at byte
    // offset 248 (header 24 bytes + 28 × 8-byte function pointers).
    // We cast to *const u8, advance by 248, then reinterpret as a function
    // pointer. The firmware guarantees this memory is valid and executable.
    // SAFETY: boot_services points to EFI_BOOT_SERVICES. Stall is at byte
    // offset 248 (header 24 bytes + 28 × 8-byte function pointers).
    // ptr::add(248) stays within the firmware-owned table; the firmware
    // guarantees the table is mapped and valid through Boot Services phase.
    // ptr::read dereferences an 8-byte-aligned location containing a fn ptr.
    unsafe {
        let ptr = boot_services as *const u8;
        let stall_ptr = ptr.add(248) as *const StallFn;
        core::ptr::read(stall_ptr)
    }
}

// ── EFI_SYSTEM_TABLE (UEFI Spec §4.3) ───────────────────────────────────────
//
// Field layout with sizes and offsets (x86_64, 8-byte pointer):
//   offset  0 : Hdr                  EfiTableHeader  24 bytes
//   offset 24 : FirmwareVendor       *const Char16    8 bytes
//   offset 32 : FirmwareRevision     u32              4 bytes
//   offset 36 : [padding]            u32              4 bytes  ← #[repr(C)] inserts automatically
//   offset 40 : ConsoleInHandle      EfiHandle        8 bytes
//   offset 48 : ConIn                *mut void        8 bytes
//   offset 56 : ConsoleOutHandle     EfiHandle        8 bytes
//   offset 64 : ConOut               *mut STO         8 bytes
//   offset 72 : StandardErrorHandle  EfiHandle        8 bytes
//   offset 80 : StdErr               *mut STO         8 bytes
//   offset 88 : RuntimeServices      *mut void        8 bytes
//   offset 96 : BootServices         *mut void        8 bytes  ← we read this
#[repr(C)]
pub struct SystemTable {
    pub hdr:                  EfiTableHeader,
    pub firmware_vendor:      *const Char16,
    pub firmware_revision:    u32,
    // 4-byte padding inserted by #[repr(C)] for pointer alignment
    _firmware_revision_pad:   u32,
    pub console_in_handle:    EfiHandle,
    pub con_in:               *mut core::ffi::c_void,
    pub console_out_handle:   EfiHandle,
    pub con_out:              *mut SimpleTextOutput,
    pub std_error_handle:     EfiHandle,
    pub std_err:              *mut SimpleTextOutput,
    pub runtime_services:     *mut core::ffi::c_void,
    pub boot_services:        *mut core::ffi::c_void,
    // remaining fields (config table count, config table) omitted — not needed
}

// ── Entry-point helpers ──────────────────────────────────────────────────────

/// Write a Rust `&str` (ASCII subset) to a UEFI SimpleTextOutput.
///
/// UEFI ConOut uses UCS-2 (a fixed-width 16-bit encoding for the BMP).
/// For pure ASCII (codepoints 0x0000–0x007F), UCS-2 and UTF-8 agree.
/// Our banner uses only ASCII, so the cast `byte as u16` is correct.
/// Line endings must be `\r\n` (CRLF) per UEFI Spec §12.4.3.
///
/// We use a small stack buffer (256 `u16`s = 512 bytes) to avoid heap
/// allocation — UEFI Boot Services pool allocation is not set up yet.
///
/// # Safety
/// `cout` must point to a valid, firmware-owned SimpleTextOutput.
pub unsafe fn print(cout: *mut SimpleTextOutput, s: &str) {
    let mut buf = [0u16; 256];
    let mut i = 0usize;
    for &byte in s.as_bytes() {
        if i + 1 >= buf.len() {
            // Flush partial buffer
            buf[i] = 0;
            // SAFETY: caller guarantees cout is valid; buffer is null-terminated.
            unsafe { ((*cout).output_string)(cout, buf.as_ptr()); }
            i = 0;
        }
        buf[i] = byte as u16;
        i += 1;
    }
    buf[i] = 0; // UCS-2 null terminator
    if i > 0 {
        // SAFETY: same as above.
        unsafe { ((*cout).output_string)(cout, buf.as_ptr()); }
    }
}

/// Print a line with UEFI-correct CRLF termination.
///
/// # Safety
/// Same as `print`.
pub unsafe fn println(cout: *mut SimpleTextOutput, s: &str) {
    // SAFETY: forwarded to `print`.
    unsafe {
        print(cout, s);
        print(cout, "\r\n");
    }
}
