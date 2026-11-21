// bootloader/src/gfx.rs
//
// UEFI Graphics Output Protocol (GOP) linear framebuffer manager.
// Spec: UEFI Specification 2.10 §12.9.

use crate::font::{get_glyph, FONT_HEIGHT, FONT_WIDTH};
use crate::uefi_raw::{EfiGraphicsOutputProtocol, EfiGraphicsPixelFormat, EfiPixelBitmask};

pub struct Framebuffer {
    pub base: *mut u8,
    pub size: usize,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: EfiGraphicsPixelFormat,
    pub bitmask: EfiPixelBitmask,
}

impl Framebuffer {
    /// Initialize Framebuffer wrapper from GOP protocol instance.
    ///
    /// # Safety
    /// `gop` must be a valid, located EFI_GRAPHICS_OUTPUT_PROTOCOL pointer.
    pub unsafe fn from_gop(gop: *mut EfiGraphicsOutputProtocol) -> Result<Self, &'static str> {
        // SAFETY: caller guarantees gop is valid.
        let mode = unsafe { (*gop).mode };
        if mode.is_null() {
            return Err("GOP mode is null");
        }

        // SAFETY: mode is non-null.
        let info = unsafe { (*mode).info };
        if info.is_null() {
            return Err("GOP mode info is null");
        }

        // SAFETY: dereferencing mode and info.
        let fb_base = unsafe { (*mode).frame_buffer_base as *mut u8 };
        let fb_size = unsafe { (*mode).frame_buffer_size };
        let width = unsafe { (*info).horizontal_resolution };
        let height = unsafe { (*info).vertical_resolution };
        let stride = unsafe { (*info).pixels_per_scan_line };
        let format = unsafe { (*info).pixel_format };
        let bitmask = unsafe { (*info).pixel_information };

        if fb_base.is_null() || fb_size == 0 {
            return Err("GOP reports zero or null framebuffer");
        }

        if format == EfiGraphicsPixelFormat::PixelBltOnly {
            return Err("PixelBltOnly is unsupported (no linear framebuffer)");
        }

        Ok(Self {
            base: fb_base,
            size: fb_size,
            width,
            height,
            stride,
            format,
            bitmask,
        })
    }

    /// Set an individual pixel color (RGB 0..255).
    ///
    /// Strict address formula respecting stride:
    /// offset = (y * stride + x) * 4
    #[inline(always)]
    pub fn put_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        if x >= self.width || y >= self.height {
            return;
        }

        let pixel_offset = (y as usize * self.stride as usize + x as usize) * 4;
        if pixel_offset + 4 > self.size {
            return;
        }

        // SAFETY: pixel_offset is within self.size and self.base is valid physical framebuffer memory.
        unsafe {
            let p = self.base.add(pixel_offset);
            match self.format {
                EfiGraphicsPixelFormat::PixelRedGreenBlueReserved8BitPerColor => {
                    // Memory layout: [R, G, B, Reserved]
                    *p.add(0) = r;
                    *p.add(1) = g;
                    *p.add(2) = b;
                    *p.add(3) = 0;
                }
                EfiGraphicsPixelFormat::PixelBlueGreenRedReserved8BitPerColor => {
                    // Memory layout: [B, G, R, Reserved]
                    *p.add(0) = b;
                    *p.add(1) = g;
                    *p.add(2) = r;
                    *p.add(3) = 0;
                }
                EfiGraphicsPixelFormat::PixelBitMask => {
                    let pixel_val = ((r as u32) & self.bitmask.red_mask)
                        | ((g as u32) & self.bitmask.green_mask)
                        | ((b as u32) & self.bitmask.blue_mask);
                    *(p as *mut u32) = pixel_val;
                }
                EfiGraphicsPixelFormat::PixelBltOnly => {}
            }
        }
    }

    /// Clear the entire screen with a uniform RGB background color.
    pub fn clear(&mut self, r: u8, g: u8, b: u8) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.put_pixel(x, y, r, g, b);
            }
        }
    }

    /// Draw a single 8x16 glyph at pixel coordinate (x, y).
    pub fn draw_char(&mut self, x: u32, y: u32, c: u8, r: u8, g: u8, b: u8) {
        let glyph = get_glyph(c);
        for (row_idx, &row_byte) in glyph.iter().enumerate() {
            for col_idx in 0..FONT_WIDTH {
                // Bit 7 is leftmost pixel, Bit 0 is rightmost pixel
                if (row_byte & (0x80 >> col_idx)) != 0 {
                    self.put_pixel(x + col_idx as u32, y + row_idx as u32, r, g, b);
                }
            }
        }
    }

    /// Draw an ASCII string starting at pixel coordinate (x, y).
    pub fn draw_string(&mut self, mut x: u32, y: u32, s: &str, r: u8, g: u8, b: u8) {
        for &byte in s.as_bytes() {
            self.draw_char(x, y, byte, r, g, b);
            x += FONT_WIDTH as u32;
        }
    }
}
