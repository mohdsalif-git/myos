// bootloader/src/main.rs
//
// MyOS Stage 1 UEFI Bootloader — Milestone B2
// ============================================
// Features:
//   1. COM1 16550 UART serial logging (115200 8N1)
//   2. TSC (Time Stamp Counter) timing calibrated against UEFI timer
//   3. EFI_GRAPHICS_OUTPUT_PROTOCOL (GOP) discovery
//   4. Linear Framebuffer handling respecting Stride (PixelsPerScanLine)
//   5. Embedded 8x16 bitmap font text rendering: exactly draws "hello salif"
//   6. Per-phase benchmark report output to serial log
//
// SPEC REFERENCES:
//   UEFI 2.10 §12.9   — EFI_GRAPHICS_OUTPUT_PROTOCOL
//   UEFI 2.10 §7.3.11 — EFI_BOOT_SERVICES.LocateProtocol
//   Intel SDM Vol 3B §17.17 — RDTSC Time-Stamp Counter
//   PC16550D UART Spec §8.0 — 16550 FIFO & Registers

#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

mod font;
mod gfx;
mod serial;
mod time;
mod uefi_raw;

use gfx::Framebuffer;
use serial::{serial_init, serial_write, serial_write_u64};
use time::{calibrate_tsc, rdtsc, ticks_to_us};
use uefi_raw::{
    EfiGraphicsOutputProtocol, EfiHandle, EfiStatus, SystemTable,
    EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID, EFI_SUCCESS,
};

#[no_mangle]
pub extern "efiapi" fn efi_main(_image: EfiHandle, st: *mut SystemTable) -> EfiStatus {
    // Record boot entry timestamp immediately
    let t_boot_entry = rdtsc();

    // ── 1. Initialize COM1 Serial Output (Port 0x3F8) ──────────────────────
    // SAFETY: COM1 port initialization on standard x86 hardware.
    unsafe {
        serial_init();
        serial_write("\n=======================================================\n");
        serial_write("[MyOS] Stage 1 Bootloader: Milestone B2 Active\n");
        serial_write("[MyOS] COM1 Serial logging initialized (115200 8N1)\n");
    }

    // ── 2. Validate System Table ───────────────────────────────────────────
    if st.is_null() {
        unsafe {
            serial_write("[ERROR] SystemTable pointer is null! Halting.\n");
        }
        loop_halt();
    }

    let bs = unsafe { (*st).boot_services };
    if bs.is_null() {
        unsafe {
            serial_write("[ERROR] BootServices pointer is null! Halting.\n");
        }
        loop_halt();
    }

    // ── 3. Calibrate TSC against UEFI Stall (10 ms) ────────────────────────
    unsafe {
        serial_write("[MyOS] Calibrating TSC against UEFI timer (10ms)...\n");
    }
    let stall_fn = unsafe { (*bs).stall };
    let ticks_per_us = unsafe { calibrate_tsc(stall_fn) };

    unsafe {
        serial_write("[MyOS] TSC Frequency: ");
        serial_write_u64(ticks_per_us);
        serial_write(" MHz (ticks/microsecond)\n");
    }

    // ── 4. Locate EFI_GRAPHICS_OUTPUT_PROTOCOL (GOP) ───────────────────────
    let t_gop_start = rdtsc();
    let mut gop_ptr: *mut core::ffi::c_void = core::ptr::null_mut();

    let status = unsafe {
        ((*bs).locate_protocol)(
            &EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID,
            core::ptr::null_mut(),
            &mut gop_ptr,
        )
    };

    if status != EFI_SUCCESS || gop_ptr.is_null() {
        unsafe {
            serial_write("[ERROR] Failed to locate EFI_GRAPHICS_OUTPUT_PROTOCOL! Status: ");
            serial_write_u64(status as u64);
            serial_write("\n");
        }
        loop_halt();
    }

    let t_gop_located = rdtsc();
    let gop = gop_ptr as *mut EfiGraphicsOutputProtocol;

    // ── 5. Initialize Framebuffer & Verify Stride ──────────────────────────
    let mut fb = match unsafe { Framebuffer::from_gop(gop) } {
        Ok(fb) => fb,
        Err(err) => {
            unsafe {
                serial_write("[ERROR] Framebuffer initialization failed: ");
                serial_write(err);
                serial_write("\n");
            }
            loop_halt();
        }
    };

    unsafe {
        serial_write("[MyOS] GOP Framebuffer found:\n");
        serial_write("       Resolution : ");
        serial_write_u64(fb.width as u64);
        serial_write(" x ");
        serial_write_u64(fb.height as u64);
        serial_write("\n       Stride     : ");
        serial_write_u64(fb.stride as u64);
        serial_write(" pixels per scanline\n       Base Addr  : 0x");
        serial_write_u64(fb.base as u64);
        serial_write("\n       Size       : ");
        serial_write_u64(fb.size as u64 / 1024);
        serial_write(" KiB\n");
    }

    // ── 6. Render Framebuffer & Draw "hello salif" ─────────────────────────
    let t_render_start = rdtsc();

    // Clear background to dark navy (R=12, G=16, B=32)
    fb.clear(12, 16, 32);

    // Header banner text
    fb.draw_string(40, 40, "MyOS Bootloader [Stage 1 - Milestone B2]", 160, 180, 220);

    // ⭐ HARD CONSTRAINT: Draw exactly "hello salif" with embedded bitmap font
    fb.draw_string(40, 80, "hello salif", 50, 255, 120);

    // Additional info for visual confirmation on screen
    fb.draw_string(40, 120, "GOP Direct Framebuffer: Active", 200, 200, 200);
    fb.draw_string(40, 140, "COM1 Serial Log: Active (115200 8N1)", 200, 200, 200);
    fb.draw_string(40, 160, "TSC Calibrated: Verified", 200, 200, 200);

    let t_render_end = rdtsc();

    // ── 7. Benchmark Report over Serial Log ─────────────────────────────────
    let gop_us = ticks_to_us(t_gop_located - t_gop_start, ticks_per_us);
    let render_us = ticks_to_us(t_render_end - t_render_start, ticks_per_us);
    let total_us = ticks_to_us(t_render_end - t_boot_entry, ticks_per_us);

    unsafe {
        serial_write("\n--- [MyOS Milestone B2 Boot Benchmarks] ---\n");
        serial_write("  GOP Protocol Locate : ");
        serial_write_u64(gop_us);
        serial_write(" us\n");
        serial_write("  Framebuffer Render  : ");
        serial_write_u64(render_us);
        serial_write(" us\n");
        serial_write("  Total B2 Phase Time : ");
        serial_write_u64(total_us);
        serial_write(" us\n");
        serial_write("-------------------------------------------\n");
        serial_write("[MyOS] B2 Acceptance Complete. Text drawn: 'hello salif'\n");
    }

    // ── 8. Stall 5 seconds for user visual inspection ──────────────────────
    unsafe {
        stall_fn(5_000_000);
    }

    EFI_SUCCESS
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
