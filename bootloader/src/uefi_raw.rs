// bootloader/src/uefi_raw.rs
//
// UEFI Specification 2.10 Protocol and Service definitions.
// Zero external dependencies. Everything strictly repr(C).

#![allow(dead_code)]

pub type EfiStatus = usize;
pub type EfiHandle = *mut core::ffi::c_void;
pub type Char16 = u16;

pub const EFI_SUCCESS: EfiStatus = 0;
pub const EFI_LOAD_ERROR: EfiStatus = 0x8000_0000_0000_0001;
pub const EFI_INVALID_PARAMETER: EfiStatus = 0x8000_0000_0000_0002;
pub const EFI_UNSUPPORTED: EfiStatus = 0x8000_0000_0000_0003;
pub const EFI_NOT_FOUND: EfiStatus = 0x8000_0000_0000_000E;

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

// EFI_LOADED_IMAGE_PROTOCOL_GUID: {5b1b31a1-9562-11d2-8e3f-00a0c969723b}
pub const EFI_LOADED_IMAGE_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0x5b1b31a1,
    data2: 0x9562,
    data3: 0x11d2,
    data4: [0x8e, 0x3f, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};

// EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_GUID: {964e5b22-6459-11d2-8e39-00a0c969723b}
pub const EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data1: 0x964e5b22,
    data2: 0x6459,
    data3: 0x11d2,
    data4: [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
};

// EFI_FILE_INFO_ID: {09576e92-6d3f-11d2-8e39-00a0c969723b}
pub const EFI_FILE_INFO_ID: EfiGuid = EfiGuid {
    data1: 0x09576e92,
    data2: 0x6d3f,
    data3: 0x11d2,
    data4: [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
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

// ── EFI_LOADED_IMAGE_PROTOCOL (UEFI Spec §9.1) ──────────────────────────────
#[repr(C)]
pub struct EfiLoadedImageProtocol {
    pub revision: u32,
    pub parent_handle: EfiHandle,
    pub system_table: *mut core::ffi::c_void,
    pub device_handle: EfiHandle,
    pub file_path: *mut core::ffi::c_void,
    pub reserved: *mut core::ffi::c_void,
    pub load_options_size: u32,
    pub load_options: *mut core::ffi::c_void,
    pub image_base: *mut core::ffi::c_void,
    pub image_size: u64,
    pub image_code_type: u32,
    pub image_data_type: u32,
    pub unload: unsafe extern "efiapi" fn(EfiHandle) -> EfiStatus,
}

// ── EFI_FILE_PROTOCOL (UEFI Spec §13.5) ─────────────────────────────────────
pub const EFI_FILE_MODE_READ: u64 = 0x0000_0000_0000_0001;

#[repr(C)]
pub struct EfiFileProtocol {
    pub revision: u64,
    pub open: unsafe extern "efiapi" fn(
        *mut Self,
        *mut *mut EfiFileProtocol,
        *const Char16,
        u64,
        u64,
    ) -> EfiStatus,
    pub close: unsafe extern "efiapi" fn(*mut Self) -> EfiStatus,
    pub delete: unsafe extern "efiapi" fn(*mut Self) -> EfiStatus,
    pub read: unsafe extern "efiapi" fn(*mut Self, *mut usize, *mut core::ffi::c_void) -> EfiStatus,
    pub write: unsafe extern "efiapi" fn(*mut Self, *mut usize, *const core::ffi::c_void) -> EfiStatus,
    pub get_position: unsafe extern "efiapi" fn(*mut Self, *mut u64) -> EfiStatus,
    pub set_position: unsafe extern "efiapi" fn(*mut Self, u64) -> EfiStatus,
    pub get_info: unsafe extern "efiapi" fn(
        *mut Self,
        *const EfiGuid,
        *mut usize,
        *mut core::ffi::c_void,
    ) -> EfiStatus,
    pub set_info: unsafe extern "efiapi" fn(
        *mut Self,
        *const EfiGuid,
        usize,
        *const core::ffi::c_void,
    ) -> EfiStatus,
    pub flush: unsafe extern "efiapi" fn(*mut Self) -> EfiStatus,
}

// ── EFI_SIMPLE_FILE_SYSTEM_PROTOCOL (UEFI Spec §13.4) ───────────────────────
#[repr(C)]
pub struct EfiSimpleFileSystemProtocol {
    pub revision: u64,
    pub open_volume: unsafe extern "efiapi" fn(*mut Self, *mut *mut EfiFileProtocol) -> EfiStatus,
}

// ── EFI_FILE_INFO (UEFI Spec §13.5.16) ──────────────────────────────────────
#[repr(C)]
pub struct EfiFileInfo {
    pub size: u64,
    pub file_size: u64,
    pub physical_size: u64,
    pub create_time: [u8; 16],
    pub last_access_time: [u8; 16],
    pub modification_time: [u8; 16],
    pub attribute: u64,
    // Variable length file_name (Char16) follows
}

// ── Memory Allocation Types (UEFI Spec §7.2) ────────────────────────────────
pub const EFI_ALLOCATE_ANY_PAGES: u32 = 0;
pub const EFI_ALLOCATE_MAX_ADDRESS: u32 = 1;
pub const EFI_ALLOCATE_ADDRESS: u32 = 2;

pub const EFI_LOADER_CODE: u32 = 1;
pub const EFI_LOADER_DATA: u32 = 2;

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
