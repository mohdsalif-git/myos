// bootloader/src/main.rs
//
// MyOS Stage 1 UEFI Bootloader — Milestone B3
// ============================================
// Features:
//   1. COM1 16550 UART serial logging (115200 8N1)
//   2. TSC timing calibrated against UEFI timer
//   3. EFI_GRAPHICS_OUTPUT_PROTOCOL (GOP) linear framebuffer
//   4. Embedded 8x16 font: draws "hello salif"
//   5. ESP file loading via EFI_SIMPLE_FILE_SYSTEM_PROTOCOL: reads \EFI\MYOS\KERNEL.ELF
//   6. Strict Static-PIE ELF64 validation and segment allocation
//   7. Prepares BootInfo and jumps to kernel stub entry point
//
// SPEC REFERENCES:
//   UEFI 2.10 §9.1    — EFI_LOADED_IMAGE_PROTOCOL
//   UEFI 2.10 §13.4   — EFI_SIMPLE_FILE_SYSTEM_PROTOCOL
//   System V AMD64 ABI — ELF64 & Function Calling Conventions

#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

mod elf;
mod font;
mod fs;
mod gfx;
mod serial;
mod time;
mod uefi_raw;

use bootinfo::{BootInfo, FramebufferInfo, PixelFormat, BOOTINFO_MAGIC};
use elf::LoadedElf;
use fs::EspFileSystem;
use gfx::Framebuffer;
use serial::{serial_init, serial_write, serial_write_u64};
use time::{calibrate_tsc, rdtsc, ticks_to_us};
use uefi_raw::{
    EfiGraphicsOutputProtocol, EfiGraphicsPixelFormat, EfiHandle, EfiStatus,
    SystemTable, EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID, EFI_SUCCESS,
};

// UCS-2 encoded path: \EFI\MYOS\KERNEL.ELF with null terminator
const KERNEL_PATH: &[u16] = &[
    b'\\' as u16, b'E' as u16, b'F' as u16, b'I' as u16,
    b'\\' as u16, b'M' as u16, b'Y' as u16, b'O' as u16, b'S' as u16,
    b'\\' as u16, b'K' as u16, b'E' as u16, b'R' as u16, b'N' as u16,
    b'E' as u16, b'L' as u16, b'.' as u16, b'E' as u16, b'L' as u16,
    b'F' as u16, 0,
];

#[no_mangle]
pub extern "efiapi" fn efi_main(image: EfiHandle, st: *mut SystemTable) -> EfiStatus {
    let t_boot_entry = rdtsc();

    // ── 1. Initialize COM1 Serial Output (Port 0x3F8) ──────────────────────
    unsafe {
        serial_init();
        serial_write("\n=======================================================\n");
        serial_write("[MyOS] Stage 1 Bootloader: Milestone B3 Active\n");
        serial_write("[MyOS] COM1 Serial logging initialized (115200 8N1)\n");
    }

    if st.is_null() {
        unsafe { serial_write("[ERROR] SystemTable is null!\n"); }
        loop_halt();
    }

    let bs = unsafe { (*st).boot_services };
    if bs.is_null() {
        unsafe { serial_write("[ERROR] BootServices is null!\n"); }
        loop_halt();
    }

    // ── 2. Calibrate TSC against UEFI Stall (10 ms) ────────────────────────
    let stall_fn = unsafe { (*bs).stall };
    let ticks_per_us = unsafe { calibrate_tsc(stall_fn) };

    unsafe {
        serial_write("[MyOS] TSC Frequency: ");
        serial_write_u64(ticks_per_us);
        serial_write(" MHz\n");
    }

    // ── 3. Locate EFI_GRAPHICS_OUTPUT_PROTOCOL (GOP) ───────────────────────
    let mut gop_ptr: *mut core::ffi::c_void = core::ptr::null_mut();
    let status = unsafe {
        ((*bs).locate_protocol)(
            &EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID,
            core::ptr::null_mut(),
            &mut gop_ptr,
        )
    };
    if status != EFI_SUCCESS || gop_ptr.is_null() {
        unsafe { serial_write("[ERROR] Failed to locate GOP!\n"); }
        loop_halt();
    }

    let gop = gop_ptr as *mut EfiGraphicsOutputProtocol;
    let mut fb = match unsafe { Framebuffer::from_gop(gop) } {
        Ok(fb) => fb,
        Err(err) => {
            unsafe {
                serial_write("[ERROR] Framebuffer init failed: ");
                serial_write(err);
                serial_write("\n");
            }
            loop_halt();
        }
    };

    // ── 4. Render Framebuffer: Draw "hello salif" ──────────────────────────
    fb.clear(12, 16, 32);
    fb.draw_string(40, 40, "MyOS Bootloader [Stage 1 - Milestone B3]", 160, 180, 220);
    fb.draw_string(40, 80, "hello salif", 50, 255, 120);
    fb.draw_string(40, 120, "Loading \\EFI\\MYOS\\KERNEL.ELF...", 200, 200, 200);

    // ── 5. Open ESP Filesystem & Read Kernel ELF ───────────────────────────
    let t_fs_start = rdtsc();
    let fs = match unsafe { EspFileSystem::open(image, bs) } {
        Ok(fs) => fs,
        Err(err) => {
            unsafe {
                serial_write("[ERROR] Failed to mount ESP: ");
                serial_write(err);
                serial_write("\n");
            }
            fb.draw_string(40, 150, "FAIL: ESP Mount Error", 255, 50, 50);
            loop_halt();
        }
    };

    let (kernel_buf, kernel_size) = match unsafe { fs.read_file(KERNEL_PATH, bs) } {
        Ok(res) => res,
        Err(err) => {
            unsafe {
                serial_write("[ERROR] Failed to read kernel file: ");
                serial_write(err);
                serial_write("\n");
            }
            fb.draw_string(40, 150, "FAIL: Kernel File Not Found", 255, 50, 50);
            loop_halt();
        }
    };

    let t_fs_end = rdtsc();
    unsafe {
        serial_write("[MyOS] Read \\EFI\\MYOS\\KERNEL.ELF: ");
        serial_write_u64(kernel_size as u64);
        serial_write(" bytes\n");
    }

    // ── 6. Parse and Load Static-PIE ELF64 ──────────────────────────────────
    let t_elf_start = rdtsc();
    let kernel_bytes = unsafe { core::slice::from_raw_parts(kernel_buf, kernel_size) };
    let loaded_elf = match unsafe { LoadedElf::load(kernel_bytes, bs) } {
        Ok(elf) => elf,
        Err(err) => {
            unsafe {
                serial_write("[ERROR] ELF64 loading failed: ");
                serial_write(err);
                serial_write("\n");
            }
            fb.draw_string(40, 150, "FAIL: Invalid Kernel ELF", 255, 50, 50);
            loop_halt();
        }
    };

    let t_elf_end = rdtsc();
    unsafe {
        serial_write("[MyOS] Kernel ELF Loaded:\n");
        serial_write("       Physical Base: 0x");
        serial_write_u64(loaded_elf.physical_base);
        serial_write("\n       Entry Point  : 0x");
        serial_write_u64(loaded_elf.entry_point);
        serial_write("\n       Total Span   : ");
        serial_write_u64(loaded_elf.total_size as u64 / 1024);
        serial_write(" KiB\n");
    }

    // ── 7. Prepare BootInfo Structure ──────────────────────────────────────
    let boot_info = BootInfo {
        magic: BOOTINFO_MAGIC,
        version: 1,
        _reserved: 0,
        framebuffer: FramebufferInfo {
            base_address: fb.base as u64,
            size: fb.size,
            width: fb.width,
            height: fb.height,
            stride: fb.stride,
            pixel_format: match fb.format {
                EfiGraphicsPixelFormat::PixelRedGreenBlueReserved8BitPerColor => PixelFormat::Rgb,
                EfiGraphicsPixelFormat::PixelBlueGreenRedReserved8BitPerColor => PixelFormat::Bgr,
                _ => PixelFormat::Bitmask,
            },
        },
        kernel_physical_base: loaded_elf.physical_base,
        kernel_size: loaded_elf.total_size as u64,
        kernel_entry: loaded_elf.entry_point,
    };

    // ── 8. Emit Timing Benchmark Report ────────────────────────────────────
    let fs_us = ticks_to_us(t_fs_end - t_fs_start, ticks_per_us);
    let elf_us = ticks_to_us(t_elf_end - t_elf_start, ticks_per_us);
    let total_us = ticks_to_us(t_elf_end - t_boot_entry, ticks_per_us);

    unsafe {
        serial_write("\n--- [MyOS Milestone B3 Benchmarks] ---\n");
        serial_write("  ESP Read Latency   : ");
        serial_write_u64(fs_us);
        serial_write(" us\n");
        serial_write("  ELF Parse & Load   : ");
        serial_write_u64(elf_us);
        serial_write(" us\n");
        serial_write("  Total Bootloader Time: ");
        serial_write_u64(total_us);
        serial_write(" us\n");
        serial_write("---------------------------------------\n");
        serial_write("[MyOS] Jumping to Kernel Entry Point...\n");
    }

    fb.draw_string(40, 150, "Kernel Loaded. Handing off to Stage 2...", 50, 255, 120);

    // ── 9. Jump to Kernel Entry Point (System V AMD64 ABI: RDI = *const BootInfo) ─
    type KernelEntryFn = extern "sysv64" fn(*const BootInfo) -> !;
    let entry_fn: KernelEntryFn = unsafe { core::mem::transmute(loaded_elf.entry_point) };

    // Jump!
    entry_fn(&boot_info);
}

fn loop_halt() -> ! {
    loop {
        unsafe {
            core::arch::asm!("cli", "hlt", options(nomem, nostack, preserves_flags));
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        serial_write("\n[PANIC] Bootloader encountered an unrecoverable panic!\n");
    }
    loop_halt();
}
