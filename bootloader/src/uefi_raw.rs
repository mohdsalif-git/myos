// bootloader/src/uefi_raw.rs
//
// UEFI Specification 2.10 Protocol and Service definitions.
// Zero external dependencies. Everything strictly repr(C).

#![allow(dead_code)]

pub type EfiStatus = usize;
pub type EfiHandle = *mut core::ffi::c_void;
pub type Char16 = u16;

pub const EFI_SUCCESS: EfiStatus = 0;
pub const EFI_UNSUPPORTED: EfiStatus = 0x8000_0000_0000_0003;

// ── EFI_GUID (UEFI Spec §Appendix A) ────────────────────────────────────────
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct EfiGuid {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

// EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID: {9042a9de-23dc-4a38-96fb-7aded080516a}
pub const EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0x9042a9de,
    data2: 0x23dc,
    data3: 0x4a38,
    data4: [0x96, 0xfb, 0x7a, 0xde, 0xd0, 0x80, 0x51, 0x6a],
};

// ── EFI_TABLE_HEADER (UEFI Spec §4.2) ───────────────────────────────────────
#[repr(C)]
pub struct EfiTableHeader {
    pub signature: u64,
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub reserved: u32,
}

// ── EFI_GRAPHICS_OUTPUT_PROTOCOL (UEFI Spec §12.9) ──────────────────────────
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EfiPixelBitmask {
    pub red_mask: u32,
    pub green_mask: u32,
    pub blue_mask: u32,
    pub reserved_mask: u32,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EfiGraphicsPixelFormat {
    PixelRedGreenBlueReserved8BitPerColor = 0,
    PixelBlueGreenRedReserved8BitPerColor = 1,
    PixelBitMask = 2,
    PixelBltOnly = 3,
}

#[repr(C)]
pub struct EfiGraphicsOutputModeInformation {
    pub version: u32,
    pub horizontal_resolution: u32,
    pub vertical_resolution: u32,
    pub pixel_format: EfiGraphicsPixelFormat,
    pub pixel_information: EfiPixelBitmask,
    pub pixels_per_scan_line: u32,
}

#[repr(C)]
pub struct EfiGraphicsOutputProtocolMode {
    pub max_mode: u32,
    pub mode: u32,
    pub info: *mut EfiGraphicsOutputModeInformation,
    pub size_of_info: usize,
    pub frame_buffer_base: u64, // Physical address
    pub frame_buffer_size: usize,
}

#[repr(C)]
pub struct EfiGraphicsOutputProtocol {
    pub query_mode: unsafe extern "efiapi" fn(
        *mut Self,
        u32,
        *mut usize,
        *mut *mut EfiGraphicsOutputModeInformation,
    ) -> EfiStatus,
    pub set_mode: unsafe extern "efiapi" fn(*mut Self, u32) -> EfiStatus,
    pub blt: unsafe extern "efiapi" fn(
        *mut Self,
        *mut core::ffi::c_void,
        u32,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
    ) -> EfiStatus,
    pub mode: *mut EfiGraphicsOutputProtocolMode,
}

// ── EFI_BOOT_SERVICES (UEFI Spec §7.1, Table 7-1 in exact declaration order) ─
#[repr(C)]
pub struct EfiBootServices {
    pub hdr: EfiTableHeader,

    // Task Priority Services (§7.2)
    pub raise_tpl: unsafe extern "efiapi" fn(usize) -> usize,
    pub restore_tpl: unsafe extern "efiapi" fn(usize),

    // Memory Services (§7.2)
    pub allocate_pages: unsafe extern "efiapi" fn(u32, u32, usize, *mut u64) -> EfiStatus,
    pub free_pages: unsafe extern "efiapi" fn(u64, usize) -> EfiStatus,
    pub get_memory_map: unsafe extern "efiapi" fn(
        *mut usize,
        *mut core::ffi::c_void,
        *mut usize,
        *mut usize,
        *mut u32,
    ) -> EfiStatus,
    pub allocate_pool: unsafe extern "efiapi" fn(u32, usize, *mut *mut core::ffi::c_void) -> EfiStatus,
    pub free_pool: unsafe extern "efiapi" fn(*mut core::ffi::c_void) -> EfiStatus,

    // Event & Timer Services (§7.1)
    pub create_event: unsafe extern "efiapi" fn(u32, usize, *mut core::ffi::c_void, *mut core::ffi::c_void, *mut EfiHandle) -> EfiStatus,
    pub set_timer: unsafe extern "efiapi" fn(EfiHandle, u32, u64) -> EfiStatus,
    pub wait_for_event: unsafe extern "efiapi" fn(usize, *mut EfiHandle, *mut usize) -> EfiStatus,
    pub signal_event: unsafe extern "efiapi" fn(EfiHandle) -> EfiStatus,
    pub close_event: unsafe extern "efiapi" fn(EfiHandle) -> EfiStatus,
    pub check_event: unsafe extern "efiapi" fn(EfiHandle) -> EfiStatus,

    // Protocol Handler Services (§7.3)
    pub install_protocol_interface: unsafe extern "efiapi" fn(*mut EfiHandle, *const EfiGuid, u32, *mut core::ffi::c_void) -> EfiStatus,
    pub reinstall_protocol_interface: unsafe extern "efiapi" fn(EfiHandle, *const EfiGuid, *mut core::ffi::c_void, *mut core::ffi::c_void) -> EfiStatus,
    pub uninstall_protocol_interface: unsafe extern "efiapi" fn(EfiHandle, *const EfiGuid, *mut core::ffi::c_void) -> EfiStatus,
    pub handle_protocol: unsafe extern "efiapi" fn(EfiHandle, *const EfiGuid, *mut *mut core::ffi::c_void) -> EfiStatus,
    pub reserved: *mut core::ffi::c_void,
    pub register_protocol_notify: unsafe extern "efiapi" fn(*const EfiGuid, EfiHandle, *mut *mut core::ffi::c_void) -> EfiStatus,
    pub locate_handle: unsafe extern "efiapi" fn(u32, *const EfiGuid, *mut core::ffi::c_void, *mut usize, *mut EfiHandle) -> EfiStatus,
    pub locate_device_path: unsafe extern "efiapi" fn(*const EfiGuid, *mut *mut core::ffi::c_void, *mut EfiHandle) -> EfiStatus,
    pub install_configuration_table: unsafe extern "efiapi" fn(*const EfiGuid, *mut core::ffi::c_void) -> EfiStatus,

    // Image Services (§7.4)
    pub load_image: unsafe extern "efiapi" fn(bool, EfiHandle, *mut core::ffi::c_void, *mut core::ffi::c_void, usize, *mut EfiHandle) -> EfiStatus,
    pub start_image: unsafe extern "efiapi" fn(EfiHandle, *mut usize, *mut *mut Char16) -> EfiStatus,
    pub exit: unsafe extern "efiapi" fn(EfiHandle, EfiStatus, usize, *mut Char16) -> EfiStatus,
    pub unload_image: unsafe extern "efiapi" fn(EfiHandle) -> EfiStatus,
    pub exit_boot_services: unsafe extern "efiapi" fn(EfiHandle, usize) -> EfiStatus,

    // Miscellaneous Services (§7.4)
    pub get_next_monotonic_count: unsafe extern "efiapi" fn(*mut u64) -> EfiStatus,
    pub stall: unsafe extern "efiapi" fn(usize) -> EfiStatus,
    pub set_watchdog_timer: unsafe extern "efiapi" fn(usize, u64, usize, *mut Char16) -> EfiStatus,

    // Driver Support Services (§7.3)
    pub connect_controller: unsafe extern "efiapi" fn(EfiHandle, *mut EfiHandle, *mut core::ffi::c_void, bool) -> EfiStatus,
    pub disconnect_controller: unsafe extern "efiapi" fn(EfiHandle, EfiHandle, EfiHandle) -> EfiStatus,

    // Open and Close Protocol Services (§7.3)
    pub open_protocol: unsafe extern "efiapi" fn(EfiHandle, *const EfiGuid, *mut *mut core::ffi::c_void, EfiHandle, EfiHandle, u32) -> EfiStatus,
    pub close_protocol: unsafe extern "efiapi" fn(EfiHandle, *const EfiGuid, EfiHandle, EfiHandle) -> EfiStatus,
    pub open_protocol_information: unsafe extern "efiapi" fn(EfiHandle, *const EfiGuid, *mut *mut core::ffi::c_void, *mut usize) -> EfiStatus,

    // Library Services (§7.3)
    pub protocols_per_handle: unsafe extern "efiapi" fn(EfiHandle, *mut *mut *mut EfiGuid, *mut usize) -> EfiStatus,
    pub locate_handle_buffer: unsafe extern "efiapi" fn(u32, *const EfiGuid, *mut core::ffi::c_void, *mut usize, *mut *mut EfiHandle) -> EfiStatus,
    pub locate_protocol: unsafe extern "efiapi" fn(*const EfiGuid, *mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> EfiStatus,
}

// ── EFI_SYSTEM_TABLE (UEFI Spec §4.3) ───────────────────────────────────────
#[repr(C)]
pub struct SystemTable {
    pub hdr: EfiTableHeader,
    pub firmware_vendor: *const Char16,
    pub firmware_revision: u32,
    _pad: u32,
    pub console_in_handle: EfiHandle,
    pub con_in: *mut core::ffi::c_void,
    pub console_out_handle: EfiHandle,
    pub con_out: *mut core::ffi::c_void,
    pub std_error_handle: EfiHandle,
    pub std_err: *mut core::ffi::c_void,
    pub runtime_services: *mut core::ffi::c_void,
    pub boot_services: *mut EfiBootServices,
}
