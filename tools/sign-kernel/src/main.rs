// tools/sign-kernel/src/main.rs
//
// MyOS Host Kernel Signing & Key Management CLI
// Formats canonical KERNEL.SIG containing SHA-256 digest + Ed25519 signature.

use std::fs;
use std::path::Path;
use sha2::{Digest, Sha256};
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey, Signature};

const SIG_MAGIC: [u8; 4] = [0x4D, 0x59, 0x4F, 0x53]; // "MYOS"
const SIG_VERSION: u32 = 1;
const CANONICAL_SIG_SIZE: usize = 4 + 4 + 32 + 64; // 104 bytes

fn print_usage() {
    eprintln!("MyOS Kernel Signing Tool");
    eprintln!("Usage:");
    eprintln!("  sign-kernel keygen <private_key_out> <public_key_out>");
    eprintln!("  sign-kernel sign   <kernel_elf> <private_key_path> <sig_out>");
    eprintln!("  sign-kernel verify <kernel_elf> <public_key_path> <sig_path>");
}

fn keygen(priv_path: &str, pub_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut rng = rand::rngs::OsRng;
    let signing_key = SigningKey::generate(&mut rng);
    let verifying_key = signing_key.verifying_key();

    fs::write(priv_path, signing_key.to_bytes())?;
    fs::write(pub_path, verifying_key.to_bytes())?;

    println!("[OK] Generated Ed25519 Keypair:");
    println!("     Private key: {}", priv_path);
    println!("     Public key : {}", pub_path);
    println!("     Public key bytes (hex): {:02x?}", verifying_key.to_bytes());
    Ok(())
}

fn sign(kernel_path: &str, priv_path: &str, sig_out: &str) -> Result<(), Box<dyn std::error::Error>> {
    let kernel_bytes = fs::read(kernel_path)?;
    let priv_bytes = fs::read(priv_path)?;

    if priv_bytes.len() != 32 {
        return Err("Private key must be exactly 32 bytes".into());
    }

    let priv_arr: [u8; 32] = priv_bytes.as_slice().try_into()?;
    let signing_key = SigningKey::from_bytes(&priv_arr);

    // 1. Compute SHA-256 of the kernel binary
    let mut hasher = Sha256::new();
    hasher.update(&kernel_bytes);
    let digest: [u8; 32] = hasher.finalize().into();

    // 2. Sign the 32-byte digest
    let signature: Signature = signing_key.sign(&digest);

    // 3. Assemble canonical 104-byte KERNEL.SIG
    let mut sig_file = Vec::with_capacity(CANONICAL_SIG_SIZE);
    sig_file.extend_from_slice(&SIG_MAGIC);
    sig_file.extend_from_slice(&SIG_VERSION.to_le_bytes());
    sig_file.extend_from_slice(&digest);
    sig_file.extend_from_slice(&signature.to_bytes());

    assert_eq!(sig_file.len(), CANONICAL_SIG_SIZE);
    fs::write(sig_out, sig_file)?;

    println!("[OK] Successfully signed kernel:");
    println!("     Kernel ELF  : {} ({} bytes)", kernel_path, kernel_bytes.len());
    println!("     SHA-256     : {:02x?}", digest);
    println!("     Signature   : {} (104 bytes)", sig_out);
    Ok(())
}

fn verify(kernel_path: &str, pub_path: &str, sig_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let kernel_bytes = fs::read(kernel_path)?;
    let pub_bytes = fs::read(pub_path)?;
    let sig_bytes = fs::read(sig_path)?;

    if pub_bytes.len() != 32 {
        return Err("Public key must be exactly 32 bytes".into());
    }
    if sig_bytes.len() != CANONICAL_SIG_SIZE {
        return Err(format!("Invalid signature size: expected {}, got {}", CANONICAL_SIG_SIZE, sig_bytes.len()).into());
    }

    if sig_bytes[0..4] != SIG_MAGIC {
        return Err("Invalid signature magic".into());
    }
    let version = u32::from_le_bytes(sig_bytes[4..8].try_into()?);
    if version != SIG_VERSION {
        return Err(format!("Unsupported signature version: {}", version).into());
    }

    let embedded_digest = &sig_bytes[8..40];
    let sig_arr: [u8; 64] = sig_bytes[40..104].try_into()?;

    // 1. Recompute SHA-256
    let mut hasher = Sha256::new();
    hasher.update(&kernel_bytes);
    let computed_digest: [u8; 32] = hasher.finalize().into();

    if embedded_digest != computed_digest.as_slice() {
        return Err("SHA-256 digest mismatch! Kernel binary has been modified.".into());
    }

    // 2. Verify Ed25519 signature
    let pub_arr: [u8; 32] = pub_bytes.as_slice().try_into()?;
    let verifying_key = VerifyingKey::from_bytes(&pub_arr)?;
    let signature = Signature::from_bytes(&sig_arr);

    verifying_key.verify(&computed_digest, &signature)?;

    println!("[OK] Signature Verification PASSED!");
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let result = match args[1].as_str() {
        "keygen" => {
            if args.len() != 4 {
                print_usage();
                std::process::exit(1);
            }
            keygen(&args[2], &args[3])
        }
        "sign" => {
            if args.len() != 5 {
                print_usage();
                std::process::exit(1);
            }
            sign(&args[2], &args[3], &args[4])
        }
        "verify" => {
            if args.len() != 5 {
                print_usage();
                std::process::exit(1);
            }
            verify(&args[2], &args[3], &args[4])
        }
        _ => {
            print_usage();
            std::process::exit(1);
        }
    };

    if let Err(e) = result {
        eprintln!("[ERROR] {}", e);
        std::process::exit(1);
    }
}
