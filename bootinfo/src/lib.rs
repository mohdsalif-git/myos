// bootinfo/src/lib.rs
//
// MyOS BootInfo Interface Contract
// Shared between Stage 1 Bootloader and Stage 2 Kernel.
// Everything is strictly #[repr(C)] to guarantee System V AMD64 ABI stability.

#![no_std]

/// Magic value identifying the BootInfo structure: "MYOSBOOT" in ASCII little-endian.
pub const BOOTINFO_MAGIC: u64 = 0x544F_4F42_534F_594D;

/// Pixel format descriptor matching UEFI GOP definitions.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgb = 0,
    Bgr = 1,
    Bitmask = 2,
}

/// Linear Framebuffer metadata passed from GOP to the kernel.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FramebufferInfo {
    pub base_address: u64,
    pub size: usize,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub pixel_format: PixelFormat,
}

/// The master BootInfo structure handed to the kernel entry point in register RDI.
#[repr(C)]
#[derive(Debug)]
pub struct BootInfo {
    /// Magic identifier: must equal BOOTINFO_MAGIC.
    pub magic: u64,
    /// Protocol version (currently 1).
    pub version: u32,
    pub _reserved: u32,

    /// GOP Framebuffer descriptor.
    pub framebuffer: FramebufferInfo,

    /// Physical base address where the kernel ELF was loaded.
    pub kernel_physical_base: u64,
    /// Total memory size occupied by the kernel segments.
    pub kernel_size: u64,
    /// Kernel entry point address.
    pub kernel_entry: u64,
}
