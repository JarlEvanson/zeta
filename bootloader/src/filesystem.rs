//! Basic utilities related to loading and validating a file.

use core::{fmt::Display, mem::MaybeUninit};

use digest::sha512::{bytes::Sha512, Digest};
use uefi::{
    boot::acquire_boot_handle,
    data_types::Align,
    proto::{
        loaded_image::LoadedImage,
        media::{
            file::{Directory, File, FileAttribute, FileInfo, FileMode},
            fs::SimpleFileSystem,
        },
    },
    table::boot::{OpenProtocolAttributes, OpenProtocolParams},
    CStr16, Status,
};

use crate::vec::Vec;

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

/// Various errors that occur when running [`acquire_boot_partition_root_directory`].
pub enum AcquireRootError {
    /// Bootloader was not loaded from a device or bootloader was loaded from a device
    /// that does not support the [`SimpleFileSystem`] protocol.
    InvalidBootMethod,
    /// Opening the volume from which the bootloader image was loaded failed.
    InvalidVolume,
}

/// Loads a file from the specified [`Directory`] and validates that it matches the digest.
///
/// # Errors
/// - [`NotFound`][nfo]
///     - File was not found
/// - [`MediaError`][me]
///     - The underlying media changed
///     - There exists no underlying media
///     - A device errror occured
/// - [`VolumeCorrupted`][vc]
///     - The volume was corrupted
/// - [`AccessDenied`][ad]
///     - Access to the file was denied
/// - [`OutOfResources`][oor]
///     - Not enough resources were available to open the file
///     - A memory allocation error occured
/// - [`NotFile`][nfi]
///     - The file was found, but was not a file
/// - [`InvalidDigest`][id]
///     - The cryptographic hash of the loaded file did not match the expected hash
///
/// [nfo]: LoadFileError::NotFound
/// [me]: LoadFileError::MediaError
/// [vc]: LoadFileError::VolumeCorrupted
/// [ad]: LoadFileError::AccessDenied
/// [oor]: LoadFileError::OutOfResources
/// [nfi]: LoadFileError::NotFile
/// [id]: LoadFileError::InvalidDigest
pub fn load_file(
    directory: &mut Directory,
    path: &CStr16,
    valid_digest: Digest,
) -> Result<Vec<u8>, LoadFileError> {
    log::trace!(target: "filesystem", "attempting to load {}", path);

    let file = match directory.open(path, FileMode::Read, FileAttribute::empty()) {
        Ok(file) => file,
        Err(err) => match err.status() {
            Status::NOT_FOUND => return Err(LoadFileError::NotFound),
            Status::MEDIA_CHANGED | Status::NO_MEDIA | Status::DEVICE_ERROR => {
                return Err(LoadFileError::MediaError)
            }
            Status::VOLUME_CORRUPTED => return Err(LoadFileError::VolumeCorrupted),
            Status::ACCESS_DENIED => return Err(LoadFileError::AccessDenied),
            Status::OUT_OF_RESOURCES => return Err(LoadFileError::OutOfResources),
            general => unreachable!("unexpected error: {:?}", general),
        },
    };

    log::trace!(target: "filesystem", "acquired file handle");

    let mut file = file.into_regular_file().ok_or(LoadFileError::NotFile)?;

    log::trace!(target: "filesystem", "opened file");

    assert!(
        <FileInfo as Align>::alignment() <= 8,
        "allocate_pool is only guaranteed to allocate 8-byte aligned pools"
    );

    let info_size = match file.get_info::<FileInfo>(&mut []) {
        Ok(_) => unreachable!(),
        Err(err) => match err.split() {
            (Status::UNSUPPORTED, _) => return Err(LoadFileError::NotFile),
            (Status::NO_MEDIA | Status::DEVICE_ERROR, _) => return Err(LoadFileError::MediaError),
            (Status::VOLUME_CORRUPTED, _) => return Err(LoadFileError::VolumeCorrupted),
            (Status::BUFFER_TOO_SMALL, size) => {
                size.expect("expected buffer size must be available")
            }
            general => unreachable!("unexpected error: {:?}", general),
        },
    };

    let mut vec = Vec::<u8>::with_capacity(info_size).map_err(|_| LoadFileError::OutOfResources)?;

    vec.spare_capacity_mut().fill(MaybeUninit::new(0));

    // SAFETY:
    // `vec.capacity()` is less than or equal to `capacity`.
    // Uninitialized memory has been filled with zeros.
    unsafe {
        vec.set_len(vec.capacity());
    }

    let info = match file.get_info::<FileInfo>(vec.as_slice_mut()) {
        Ok(info) => info,
        Err(err) => match err.split() {
            (Status::UNSUPPORTED, _) => return Err(LoadFileError::NotFile),
            (Status::NO_MEDIA | Status::DEVICE_ERROR, _) => return Err(LoadFileError::MediaError),
            (Status::VOLUME_CORRUPTED, _) => return Err(LoadFileError::VolumeCorrupted),
            general => unreachable!("unexpected error: {:?}", general),
        },
    };

    let required_bytes =
        TryInto::<usize>::try_into(info.file_size()).map_err(|_| LoadFileError::OutOfResources)?;

    vec.try_reserve(required_bytes.saturating_sub(vec.len()))
        .map_err(|_| LoadFileError::OutOfResources)?;

    vec.spare_capacity_mut().fill(MaybeUninit::new(0));

    // SAFETY:
    // `vec.capacity()` is less than or equal to `capacity`.
    // Uninitialized memory has been filled with zeros.
    unsafe {
        vec.set_len(vec.capacity());
    }

    match file.read(vec.as_slice_mut()) {
        Ok(bytes) => {
            log::trace!(target: "filesystem", "read {bytes} bytes out of {required_bytes} total bytes");
        }
        Err(_) => return Err(LoadFileError::MediaError),
    }

    let mut sha512 = Sha512::new();

    sha512.update(vec.as_slice_mut()).unwrap();

    let digest = sha512.finalize();

    if digest != valid_digest {
        return Err(LoadFileError::InvalidDigest);
    }

    log::trace!(target: "filesystem", "validated file digest");

    Ok(vec)
}

/// Various errors occurring when attempting to load a file.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoadFileError {
    /// The file was not found.
    NotFound,
    /// An error occurred when locating or reading from the media.
    MediaError,
    /// There are not enough resources to load the file.
    OutOfResources,
    /// The requested file is not a file.
    NotFile,
    /// Access to the file or media was denied.
    AccessDenied,
    /// The volume's structure was corrupted.
    VolumeCorrupted,
    /// The digest of the file did not match the expected digest.
    InvalidDigest,
}

impl Display for LoadFileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LoadFileError::NotFound => f.write_str("the requested file was not found"),
            LoadFileError::MediaError => f.write_str("a media error ocurred"),
            LoadFileError::OutOfResources => f.write_str("out of resources"),
            LoadFileError::NotFile => f.write_str("requested item was not a file"),
            LoadFileError::AccessDenied => f.write_str("access to the file was not allowed"),
            LoadFileError::VolumeCorrupted => f.write_str("the volume was corrupted"),
            LoadFileError::InvalidDigest => {
                f.write_str("the digest of the file was unexpected: verify its authenticity")
            }
        }
    }
}

impl From<LoadFileError> for Status {
    fn from(value: LoadFileError) -> Self {
        match value {
            LoadFileError::NotFound => Status::NOT_FOUND,
            LoadFileError::MediaError => Status::ABORTED,
            LoadFileError::OutOfResources => Status::OUT_OF_RESOURCES,
            LoadFileError::NotFile => Status::INVALID_PARAMETER,
            LoadFileError::AccessDenied => Status::ACCESS_DENIED,
            LoadFileError::VolumeCorrupted => Status::VOLUME_CORRUPTED,
            LoadFileError::InvalidDigest => Status::SECURITY_VIOLATION,
        }
    }
}
