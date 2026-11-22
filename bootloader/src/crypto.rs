// bootloader/src/crypto.rs
//
// Milestone B4: Cryptographic verification module for MyOS bootloader.
// Enforces detached signature validation over KERNEL.ELF.

#![allow(dead_code)]

pub const SIG_MAGIC: [u8; 4] = [0x4D, 0x59, 0x4F, 0x53]; // "MYOS"
pub const SIG_VERSION: u32 = 1;
pub const EXPECTED_SIG_LEN: usize = 104;

pub const TRUSTED_PUBLIC_KEY: [u8; 32] = [
    0x2c, 0xa3, 0x7b, 0x6e, 0x98, 0x1f, 0x4d, 0x3a,
    0x8c, 0x5e, 0x71, 0x90, 0x24, 0x6b, 0x8a, 0xdf,
    0x13, 0x49, 0x6a, 0x7c, 0x52, 0x38, 0x9e, 0x01,
    0x4f, 0x83, 0xbc, 0x7d, 0x61, 0x92, 0x05, 0xaa,
];

pub struct ParsedSignature<'a> {
    pub digest: &'a [u8; 32],
    pub signature: &'a [u8; 64],
}

/// Validates the format and headers of the canonical 104-byte KERNEL.SIG file.
pub fn parse_signature(sig_bytes: &[u8]) -> Result<ParsedSignature<'_>, &'static str> {
    if sig_bytes.len() != EXPECTED_SIG_LEN {
        return Err("Invalid signature file length (expected exactly 104 bytes)");
    }

    if sig_bytes[0..4] != SIG_MAGIC {
        return Err("Invalid signature magic header (expected 'MYOS')");
    }

    let version_bytes: [u8; 4] = sig_bytes[4..8].try_into().unwrap();
    let version = u32::from_le_bytes(version_bytes);
    if version != SIG_VERSION {
        return Err("Unsupported signature format version");
    }

    let digest: &[u8; 32] = sig_bytes[8..40].try_into().unwrap();
    let signature: &[u8; 64] = sig_bytes[40..104].try_into().unwrap();

    Ok(ParsedSignature { digest, signature })
}

/// Constant-time memory comparison to prevent side-channel timing attacks.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
