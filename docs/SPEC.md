# MyOS Stage 1 UEFI Bootloader Specification & Threat Model

## 1. System Overview

MyOS Stage 1 Bootloader is a 64-bit UEFI application (`BOOTX64.EFI`) built in Rust targeting `x86_64-unknown-uefi`. It establishes an end-to-end authenticated, measured, and memory-isolated boot pipeline from UEFI firmware to the 64-bit MyOS kernel.

### Boot Sequence (Stage 1)
```
[UEFI Firmware / Secure Boot db]
          │
          ▼ Verifies Authenticode Signature
   [BOOTX64.EFI Entry]
          │
          ├─► Initialize COM1 Serial logging & Calibrate TSC
          ├─► Query GOP & render "hello salif" via embedded 8x16 font
          ├─► Open ESP (Simple File System Protocol)
          ├─► Read `kernel.elf` and `kernel.sig` (single-pass read)
          ├─► Verify Ed25519 signature over SHA-256 digest (Fail Closed)
          ├─► Extend Measurement into TPM 2.0 (TCG2 Protocol -> PCR 9)
          ├─► Parse static-PIE ELF64, allocate pages, apply R_X86_64_RELATIVE relocations
          ├─► Build 4-level Paging (PML4): Higher-half kernel, HHDM, W^X, KASLR slide
          ├─► Allocate kernel stack (with unmapped guard page) & prepare BootInfo
          ├─► ExitBootServices (with memory map retry loop)
          └─► Switch CR3 (page tables), jump to kernel_main(BootInfo in RDI)
```

---

## 2. Threat Model

Security in Stage 1 is structured around cryptographic verification and hardware measurement.

### 2.1 In-Scope Threats (What We Protect Against)
1. **Malicious or Tampered Bootloader**:
   - Prevented by **UEFI Secure Boot**: Firmware validates the Authenticode PE signature of `BOOTX64.EFI` against enrolled keys in `db`.
2. **Malicious or Modified Kernel Binary**:
   - Prevented by **Ed25519 Signature Verification**: The bootloader verifies `kernel.sig` before parsing ELF segments. If a single byte is flipped, the bootloader halts (`fail closed`).
3. **Boot Configuration Tampering & Undetected Compromise**:
   - Prevented by **TPM 2.0 Measured Boot**: The kernel hash and boot parameters are extended into PCR 9 via `EFI_TCG2_PROTOCOL`. Remote attestation or sealed disk keys can detect any deviation in the measured boot log.
4. **Kernel Space Memory Exploits (Return-to-Data / Code Injection)**:
   - Prevented by **Strict W^X (Write XOR Execute)**: Page tables map `.text` as Read-Only/Executable (`NX=0`, `R/W=0`), and `.data`/`.bss`/stack as Read-Write/No-Execute (`NX=1`, `R/W=1`).
5. **Predictable Address Exploitation**:
   - Mitigated by **KASLR (Kernel Address Space Layout Randomization)**: The higher-half virtual base is randomized using high-entropy hardware random seeds (`EFI_RNG_PROTOCOL`, `RDSEED`, `RDRAND`).
6. **Stack Overflow into Neighboring Structures**:
   - Prevented by an **Unmapped Guard Page** directly below the kernel stack.

### 2.2 Out-of-Scope Threats (What We Do NOT Protect Against)
1. **Firmware Compromise / SMM (System Management Mode) Rootkits**:
   - Ring -2 (SMM) has higher privilege than the CPU in Ring 0. If firmware is malicious, all UEFI protections can be bypassed.
2. **Compromised Private Signing Keys**:
   - If the private key used to sign the kernel or the Secure Boot key is stolen, the adversary can sign malicious payloads that validate successfully.
3. **Physical Hardware Tampering**:
   - Direct SPI flash reprogramming, interposer attacks on the LPC/SPI TPM bus, cold boot memory attacks, or malicious PCIe bus-mastering devices (without IOMMU enforcement).
4. **Post-Handoff Vulnerabilities in Kernel Space**:
   - Once execution transitions to `kernel_main`, the bootloader's role ends. Kernel isolation depends on Stage 2 architecture.

---

## 3. TCG 2.0 PCR Mapping Architecture

According to the **TCG PC Client Platform Firmware Profile Specification (Version 1.05, Section 3.3.4)**:
- **PCRs 0–7**: Reserved for firmware, UEFI drivers, Option ROMs, and Secure Boot policy. The OS/bootloader **must not** extend these directly.
- **PCR 8**: Historically defined for command line / boot arguments / loader measurement in legacy PC Client specs.
- **PCR 9**: Defined for the OS loader authority, kernel image, and initrd/payload measurement:
  > *"If the OS loader measures the OS kernel, it SHOULD measure the kernel into PCR[9]."*
- **PCRs 10–15**: Reserved for OS runtime, application integrity, and custom security domains.

**MyOS Assignment**:
- **PCR 9**: `kernel.elf` binary digest (SHA-256) via `EFI_TCG2_PROTOCOL::HashLogExtendEvent`.
- **PCR 8**: Optional `boot.cfg` / command line configuration parameters.

---

## 4. BootInfo Interface Contract

The bootloader hands off control to the kernel with System V AMD64 ABI:
```rust
extern "sysv64" fn kernel_main(info: *const BootInfo) -> !
```
- First argument passed in register `RDI`.
- Paging is active (PML4 loaded into `CR3`).
- Direct physical memory map (HHDM) accessible at fixed virtual offset (`0xFFFF_8800_0000_0000`).
- Kernel segments mapped at randomized higher-half base (`0xFFFF_FFFF_8000_0000 + slide`).
- Stack pointer (`RSP`) points to top of valid allocated kernel stack, preceded by an unmapped guard page.
- Interrupts disabled (`cli`).
- GDT loaded with 64-bit code/data descriptors.
