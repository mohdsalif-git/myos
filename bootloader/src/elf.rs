// bootloader/src/elf.rs
//
// Zero-dependency ELF64 Parser & Segment Loader.
// Strictly validates static-PIE (ET_DYN) x86_64 binaries.
// Spec: System V AMD64 ABI & Tool Interface Standard (TIS) ELF64.

use crate::uefi_raw::{
    EfiBootServices, EFI_ALLOCATE_ANY_PAGES, EFI_LOADER_DATA, EFI_SUCCESS,
};

// ── ELF64 Header Definitions ────────────────────────────────────────────────
const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
const ELF_CLASS_64: u8 = 2;
const ELF_DATA_2LSB: u8 = 1; // 2's complement, little endian
const ET_DYN: u16 = 3;       // Position-Independent Executable
const EM_X86_64: u16 = 62;   // AMD x86-64 architecture

const PT_LOAD: u32 = 1;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Header {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64ProgramHeader {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

pub struct LoadedElf {
    pub entry_point: u64,
    pub physical_base: u64,
    pub total_size: usize,
}

impl LoadedElf {
    /// Validate and load a static-PIE ELF64 binary into memory.
    ///
    /// # Safety
    /// `file_bytes` must be a valid slice of memory containing the ELF image.
    /// `bs` must point to valid EFI_BOOT_SERVICES.
    pub unsafe fn load(
        file_bytes: &[u8],
        bs: *mut EfiBootServices,
    ) -> Result<Self, &'static str> {
        // 1. Validate Header Size
        if file_bytes.len() < core::mem::size_of::<Elf64Header>() {
            return Err("ELF file smaller than Elf64Header");
        }

        // SAFETY: Size is verified above.
        let header = unsafe { &*(file_bytes.as_ptr() as *const Elf64Header) };

        // 2. Strict Header Validation
        if header.e_ident[0..4] != ELF_MAGIC {
            return Err("Invalid ELF magic");
        }
        if header.e_ident[4] != ELF_CLASS_64 {
            return Err("ELF is not 64-bit (EI_CLASS != 2)");
        }
        if header.e_ident[5] != ELF_DATA_2LSB {
            return Err("ELF is not little-endian (EI_DATA != 1)");
        }
        if header.e_machine != EM_X86_64 {
            return Err("ELF target architecture is not x86_64");
        }
        if header.e_type != ET_DYN {
            return Err("ELF is not a Position-Independent Executable (ET_DYN)");
        }

        // 3. Scan Program Headers to calculate total contiguous memory span
        let ph_offset = header.e_phoff as usize;
        let ph_entry_size = header.e_phentsize as usize;
        let ph_count = header.e_phnum as usize;

        if ph_offset + (ph_count * ph_entry_size) > file_bytes.len() {
            return Err("Program header table extends past end of file");
        }

        let mut max_vaddr: u64 = 0;
        let mut min_vaddr: u64 = u64::MAX;

        for i in 0..ph_count {
            let ph_ptr = unsafe {
                file_bytes.as_ptr().add(ph_offset + i * ph_entry_size) as *const Elf64ProgramHeader
            };
            let ph = unsafe { &*ph_ptr };

            if ph.p_type == PT_LOAD {
                // Segment bounds check
                if ph.p_offset.saturating_add(ph.p_filesz) > file_bytes.len() as u64 {
                    return Err("PT_LOAD segment extends past end of file");
                }
                if ph.p_filesz > ph.p_memsz {
                    return Err("PT_LOAD p_filesz > p_memsz");
                }

                if ph.p_vaddr < min_vaddr {
                    min_vaddr = ph.p_vaddr;
                }
                let end = ph.p_vaddr.saturating_add(ph.p_memsz);
                if end > max_vaddr {
                    max_vaddr = end;
                }
            }
        }

        if min_vaddr == u64::MAX || max_vaddr <= min_vaddr {
            return Err("No valid PT_LOAD segments found in ELF");
        }

        let total_span = (max_vaddr - min_vaddr) as usize;
        let pages = (total_span + 4095) / 4096;

        // 4. Allocate contiguous physical memory pages for the kernel
        let mut base_phys: u64 = 0;
        let status = unsafe {
            ((*bs).allocate_pages)(
                EFI_ALLOCATE_ANY_PAGES,
                EFI_LOADER_DATA,
                pages,
                &mut base_phys,
            )
        };
        if status != EFI_SUCCESS || base_phys == 0 {
            return Err("Failed to allocate LoaderData pages for kernel segments");
        }

        let load_base = base_phys as *mut u8;

        // Zero out the entire allocated span
        unsafe {
            core::ptr::write_bytes(load_base, 0, pages * 4096);
        }

        // 5. Load segments and populate memory
        for i in 0..ph_count {
            let ph_ptr = unsafe {
                file_bytes.as_ptr().add(ph_offset + i * ph_entry_size) as *const Elf64ProgramHeader
            };
            let ph = unsafe { &*ph_ptr };

            if ph.p_type == PT_LOAD {
                let segment_dest = unsafe { load_base.add((ph.p_vaddr - min_vaddr) as usize) };
                let segment_src = unsafe { file_bytes.as_ptr().add(ph.p_offset as usize) };

                // Copy filesz bytes from file into segment memory
                unsafe {
                    core::ptr::copy_nonoverlapping(segment_src, segment_dest, ph.p_filesz as usize);
                }
                // .bss (p_memsz - p_filesz) is already zeroed because we zeroed the full page allocation
            }
        }

        // Calculate absolute entry point: base + (e_entry - min_vaddr)
        let entry_point = base_phys + (header.e_entry - min_vaddr);

        Ok(Self {
            entry_point,
            physical_base: base_phys,
            total_size: pages * 4096,
        })
    }
}
