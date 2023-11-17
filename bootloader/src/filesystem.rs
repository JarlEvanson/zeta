use uefi::{
    boot::acquire_boot_handle,
    proto::{
        loaded_image::LoadedImage,
        media::{file::Directory, fs::SimpleFileSystem},
    },
    table::boot::{OpenProtocolAttributes, OpenProtocolParams},
};

/// Acquire the root directory of the boot partition.
///
/// Only supports devices that support the [`SimpleFileSystem`] protocol.
///
/// # Panics
/// Panics if a UEFI implementation does not install the [`LoadedImage`] protocol
/// on the image handle.
///
/// # Errors
/// - [`InvalidBootMethod`][ibm]
///     - Bootloader image does not have a device it was loaded from.
///     - Bootloader image was loaded from a device that does not support the
///         [`SimpleFileSystem`] protocol.
/// - [`InvalidVolume`][iv]
///     - Failure to open the volume from which the bootloader image was loaded.
///
/// [ibm]: [AcquireRootError::InvalidBootMethod]
/// [iv]: [AcquireRootError::InvalidVolume]
pub fn acquire_boot_partition_root_directory() -> Result<Directory, AcquireRootError> {
    let boot_handle = acquire_boot_handle();

    let image_handle = boot_handle.image_handle();

    let open_params = OpenProtocolParams {
        agent: image_handle,
        controller: None,
        handle: image_handle,
    };

    // SAFETY:
    // `image_handle` will remain valid until usage ends because this image is `image_handle`.
    // `image_handle` should always have the `LoadedImage` protocol, since it is a loaded image.
    let loaded_image = unsafe {
        boot_handle
            .open_protocol::<LoadedImage>(open_params, OpenProtocolAttributes::GetProtocol)
            .expect("`image_handle` must support the `LoadedImage` protocol")
    };

    let Some(device_handle) = loaded_image.device() else {
        return Err(AcquireRootError::InvalidBootMethod);
    };

    let Ok(mut simple_file_system) =
        boot_handle.open_protocol_exclusive::<SimpleFileSystem>(device_handle)
    else {
        return Err(AcquireRootError::InvalidBootMethod);
    };

    let Ok(root_directory) = simple_file_system.open_volume() else {
        return Err(AcquireRootError::InvalidVolume);
    };

    Ok(root_directory)
}

pub enum AcquireRootError {
    InvalidBootMethod,
    InvalidVolume,
}
