// bootloader/src/fs.rs
//
// UEFI Simple File System driver for reading files from the ESP.
// Uses EFI_LOADED_IMAGE_PROTOCOL and EFI_SIMPLE_FILE_SYSTEM_PROTOCOL.
// Spec: UEFI Specification 2.10 §9.1, §13.4, §13.5.

use crate::uefi_raw::{
    EfiBootServices, EfiFileProtocol, EfiHandle, EfiLoadedImageProtocol,
    EfiSimpleFileSystemProtocol, EFI_FILE_INFO_ID, EFI_FILE_MODE_READ,
    EFI_LOADED_IMAGE_PROTOCOL_GUID, EFI_LOADER_DATA, EFI_ALLOCATE_ANY_PAGES,
    EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_GUID, EFI_SUCCESS,
};

pub struct EspFileSystem {
    root: *mut EfiFileProtocol,
}

impl EspFileSystem {
    /// Open the ESP partition where this bootloader image was executed from.
    ///
    /// # Safety
    /// `image_handle` must be the valid image handle passed to `efi_main`.
    /// `bs` must point to valid EFI_BOOT_SERVICES.
    pub unsafe fn open(image_handle: EfiHandle, bs: *mut EfiBootServices) -> Result<Self, &'static str> {
        // 1. Get LoadedImageProtocol for our image handle to discover the DeviceHandle
        let mut loaded_image_ptr: *mut core::ffi::c_void = core::ptr::null_mut();
        let status = unsafe {
            ((*bs).handle_protocol)(
                image_handle,
                &EFI_LOADED_IMAGE_PROTOCOL_GUID,
                &mut loaded_image_ptr,
            )
        };
        if status != EFI_SUCCESS || loaded_image_ptr.is_null() {
            return Err("Failed to open LoadedImageProtocol on bootloader image handle");
        }

        let loaded_image = loaded_image_ptr as *mut EfiLoadedImageProtocol;
        let device_handle = unsafe { (*loaded_image).device_handle };
        if device_handle.is_null() {
            return Err("LoadedImageProtocol returned null device_handle");
        }

        // 2. Open SimpleFileSystemProtocol on that DeviceHandle
        let mut sfs_ptr: *mut core::ffi::c_void = core::ptr::null_mut();
        let status = unsafe {
            ((*bs).handle_protocol)(
                device_handle,
                &EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_GUID,
                &mut sfs_ptr,
            )
        };
        if status != EFI_SUCCESS || sfs_ptr.is_null() {
            return Err("Failed to open SimpleFileSystemProtocol on ESP device handle");
        }

        let sfs = sfs_ptr as *mut EfiSimpleFileSystemProtocol;

        // 3. Open root directory volume
        let mut root_ptr: *mut EfiFileProtocol = core::ptr::null_mut();
        let status = unsafe { ((*sfs).open_volume)(sfs, &mut root_ptr) };
        if status != EFI_SUCCESS || root_ptr.is_null() {
            return Err("Failed to open ESP root directory volume");
        }

        Ok(Self { root: root_ptr })
    }

    /// Read an entire file into newly allocated `EfiLoaderData` memory pages.
    /// Returns: (buffer_pointer, file_size_in_bytes)
    ///
    /// # Safety
    /// `path_ucs2` must be a null-terminated slice of 16-bit UCS-2 characters.
    /// `bs` must point to valid EFI_BOOT_SERVICES.
    pub unsafe fn read_file(
        &self,
        path_ucs2: &[u16],
        bs: *mut EfiBootServices,
    ) -> Result<(*mut u8, usize), &'static str> {
        // 1. Open the file in read-only mode
        let mut file_ptr: *mut EfiFileProtocol = core::ptr::null_mut();
        let status = unsafe {
            ((*self.root).open)(
                self.root,
                &mut file_ptr,
                path_ucs2.as_ptr(),
                EFI_FILE_MODE_READ,
                0,
            )
        };
        if status != EFI_SUCCESS || file_ptr.is_null() {
            return Err("Failed to open requested file on ESP");
        }

        // 2. Query file size via GetInfo(EFI_FILE_INFO_ID)
        // Allocate a buffer on the stack for EFI_FILE_INFO (approx 128 bytes is enough for small paths)
        let mut info_buf = [0u8; 256];
        let mut info_size = info_buf.len();

        let status = unsafe {
            ((*file_ptr).get_info)(
                file_ptr,
                &EFI_FILE_INFO_ID,
                &mut info_size,
                info_buf.as_mut_ptr() as *mut core::ffi::c_void,
            )
        };
        if status != EFI_SUCCESS {
            unsafe { ((*file_ptr).close)(file_ptr); }
            return Err("Failed to query file information (size)");
        }

        // EFI_FILE_INFO layout: file_size is a u64 at byte offset 8
        let file_size = unsafe {
            core::ptr::read_unaligned(info_buf.as_ptr().add(8) as *const u64) as usize
        };

        if file_size == 0 {
            unsafe { ((*file_ptr).close)(file_ptr); }
            return Err("File is empty (0 bytes)");
        }

        // 3. Allocate pages in EfiLoaderData memory
        // 4096 bytes per page
        let pages = (file_size + 4095) / 4096;
        let mut buffer_phys: u64 = 0;

        let status = unsafe {
            ((*bs).allocate_pages)(
                EFI_ALLOCATE_ANY_PAGES,
                EFI_LOADER_DATA,
                pages,
                &mut buffer_phys,
            )
        };
        if status != EFI_SUCCESS || buffer_phys == 0 {
            unsafe { ((*file_ptr).close)(file_ptr); }
            return Err("Failed to allocate LoaderData pages for file");
        }

        let buffer_ptr = buffer_phys as *mut u8;

        // 4. Read the file into the allocated buffer
        let mut bytes_to_read = file_size;
        let status = unsafe {
            ((*file_ptr).read)(
                file_ptr,
                &mut bytes_to_read,
                buffer_ptr as *mut core::ffi::c_void,
            )
        };

        // Close file handle
        unsafe {
            ((*file_ptr).close)(file_ptr);
        }

        if status != EFI_SUCCESS || bytes_to_read != file_size {
            return Err("Failed to read complete file data");
        }

        Ok((buffer_ptr, file_size))
    }
}
