// bootloader/src/serial.rs
//
// 16550A UART driver — COM1 (I/O base port 0x3F8).
//
// SPEC: National Semiconductor PC16550D UART datasheet.
// Every hardware register name and bit definition below is cited from that sheet.
//
// WHY SERIAL?
//   UEFI ConOut disappears after ExitBootServices. The linear framebuffer
//   survives but cannot be safely written to during early crash paths.
//   The 16550 UART is memory-mapped via x86 I/O ports — it works from the
//   very first instruction, requires no allocation, and is readable from the
//   host (VirtualBox "serial port -> file" or QEMU -serial stdio).
//
// REGISTER MAP (base = 0x3F8):
//   base+0  : RBR (read) / THR (write) / DLL (DLAB=1)   Receiver/Transmit/Divisor LSB
//   base+1  : IER        / DLM (DLAB=1)                  Interrupt Enable / Divisor MSB
//   base+2  : IIR (read) / FCR (write)                   FIFO Control
//   base+3  : LCR                                        Line Control
//   base+4  : MCR                                        Modem Control
//   base+5  : LSR (read-only)                            Line Status
//   base+6  : MSR (read-only)                            Modem Status
//   base+7  : SCR                                        Scratch
//
// BAUD RATE:
//   16550 uses a 1.8432 MHz crystal.  Divisor = 1_843_200 / (16 × baud).
//   For 115200 baud: divisor = 1 (fastest, universally supported by hosts).

const COM1_BASE: u16 = 0x3F8;

// Register offsets
const OFF_DATA: u16 = 0; // THR (write) / RBR (read)
const OFF_IER:  u16 = 1; // Interrupt Enable Register
const OFF_FCR:  u16 = 2; // FIFO Control Register
const OFF_LCR:  u16 = 3; // Line Control Register — DLAB is bit 7
const OFF_MCR:  u16 = 4; // Modem Control Register
const OFF_LSR:  u16 = 5; // Line Status Register
// DLL (Divisor Latch LSB) = base+0 when DLAB=1
// DLM (Divisor Latch MSB) = base+1 when DLAB=1

// LSR bits (PC16550D §8.6.5)
const LSR_THRE: u8 = 0x20; // Transmitter Holding Register Empty — safe to write

/// Write a byte to an x86 I/O port.
///
/// # Safety
/// The caller must ensure `port` is a valid, writable x86 I/O port.
/// Sending arbitrary bytes to wrong ports can crash or hang the system.
#[inline]
unsafe fn outb(port: u16, val: u8) {
    // SAFETY: `out dx, al` — x86 I/O write. Valid in UEFI pre-ExitBootServices
    // (CPL 0, IOPL 0 is the firmware-established privilege level on x86_64).
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") val,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Read a byte from an x86 I/O port.
///
/// # Safety
/// Same constraints as `outb`.
#[inline]
unsafe fn inb(port: u16) -> u8 {
    let val: u8;
    // SAFETY: `in al, dx` — x86 I/O read.
    unsafe {
        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            out("al") val,
            options(nomem, nostack, preserves_flags)
        );
    }
    val
}

/// Initialize COM1 to 115200 8N1, FIFO enabled.
///
/// Must be called once before any `serial_write_*` call.
///
/// # Safety
/// Writes to I/O ports 0x3F8–0x3FF.  Valid only on x86 hardware with a
/// 16550-compatible UART at COM1. VirtualBox always emulates one.
pub unsafe fn serial_init() {
    unsafe {
        // 1. Disable interrupts (IER = 0x00). We poll LSR for TX readiness.
        outb(COM1_BASE + OFF_IER, 0x00);

        // 2. Enable DLAB to access divisor registers (LCR bit 7 = 1).
        outb(COM1_BASE + OFF_LCR, 0x80);

        // 3. Set divisor = 1 → 115200 baud.
        //    DLL (base+0 when DLAB=1) = 0x01, DLM (base+1) = 0x00.
        outb(COM1_BASE + OFF_DATA, 0x01); // DLL = 1
        outb(COM1_BASE + OFF_IER,  0x00); // DLM = 0

        // 4. Set 8N1: 8 data bits (bits 1:0 = 11), No parity (bit 3 = 0),
        //    1 stop bit (bit 2 = 0). Clear DLAB (bit 7 = 0). → LCR = 0x03.
        outb(COM1_BASE + OFF_LCR, 0x03);

        // 5. Enable and reset FIFOs, trigger at 14-byte threshold.
        //    FCR = 0xC7: FIFO enable(0), RX FIFO reset(1), TX FIFO reset(2),
        //                14-byte threshold (bits 7:6 = 11).
        outb(COM1_BASE + OFF_FCR, 0xC7);

        // 6. Assert RTS + DTR in MCR so the host sees the port as "ready".
        //    MCR = 0x03: DTR(0) | RTS(1).
        outb(COM1_BASE + OFF_MCR, 0x03);
    }
}

/// Write one byte to COM1, polling until the UART is ready.
///
/// # Safety
/// `serial_init()` must have been called first.
#[inline]
pub unsafe fn serial_write_byte(byte: u8) {
    unsafe {
        // Poll LSR.THRE (bit 5): wait until the transmit FIFO is empty.
        while inb(COM1_BASE + OFF_LSR) & LSR_THRE == 0 {}
        outb(COM1_BASE + OFF_DATA, byte);
    }
}

/// Write a UTF-8 string to COM1 with CRLF line ending.
///
/// We emit `\r\n` for `\n` so serial terminals and VirtualBox logs display
/// lines correctly on Windows hosts.
///
/// # Safety
/// `serial_init()` must have been called first.
pub unsafe fn serial_write(s: &str) {
    for &b in s.as_bytes() {
        unsafe {
            if b == b'\n' {
                serial_write_byte(b'\r');
            }
            serial_write_byte(b);
        }
    }
}

/// Write a u64 as decimal digits to COM1.
///
/// # Safety
/// `serial_init()` must have been called first.
pub unsafe fn serial_write_u64(mut n: u64) {
    if n == 0 {
        unsafe { serial_write_byte(b'0'); }
        return;
    }
    let mut buf = [0u8; 20]; // max 20 decimal digits for u64
    let mut len = 0usize;
    while n > 0 {
        buf[len] = b'0' + (n % 10) as u8;
        n /= 10;
        len += 1;
    }
    // digits are stored in reverse
    for i in (0..len).rev() {
        unsafe { serial_write_byte(buf[i]); }
    }
}
