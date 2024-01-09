//! A bootloader for the zeta microkernel.

#![no_std]
#![cfg_attr(not(test), no_main)]
#![feature(
    lint_reasons,
    maybe_uninit_slice,
    strict_provenance,
    error_in_core,
    debug_closure_helpers,
    core_intrinsics,
    const_heap,
    const_slice_from_raw_parts_mut,
    const_ptr_write,
    try_trait_v2,
    non_null_convenience
)]

use bootloader::PhysicalMemoryType;
use digest::sha512::Digest;
use filesystem::{acquire_loaded_image_directory, load_file};
use uefi::{
    boot::{acquire_boot_handle, ScopedProtocol},
    proto::loaded_image::LoadedImage,
    table::{
        boot::{MemoryAttribute, MemoryDescriptor, MemoryType, PAGE_SIZE},
        Boot, SystemTable,
    },
    CStr16, Handle, Status,
};

use crate::{
    arena::Arena,
    config::parse_configuration_file,
    logging::{set_framebuffer_filter, set_global_filter, set_serial_filter},
    vec::Vec,
};

mod arena;
mod config;
mod filesystem;
mod logging;
#[cfg_attr(not(feature = "framebuffer_logging"), expect(unused))]
mod terminal;
mod vec;

/// The default logging level.
pub const DEFAULT_LOGGING_LEVEL: log::LevelFilter = log::LevelFilter::Info;

/// The path to the configuration file.
const CONFIG_PATH: &CStr16 = uefi::cstr16!("zeta\\config.toml");

/// The current digest of the test configuration file.
#[export_name = "digest"]
#[link_section = ".config"]
static CONFIG_DIGEST: Digest = match Digest::from_str(
    "e66c4051f3d04110180967ad0de5780b685d04c813b07cd1457440d22f40a7a120c7c9ee5e15ebdd26d59f8bbf9c8ab15f60044155f18b63c13e13949c3abdaf"
) {
    Some(digest) => digest,
    None => panic!("invalid digest"),
};

/// The amount of time we stall before returning in the event of an error.
const ERROR_STALL_TIME: usize = 10_000_000;

#[allow(clippy::too_many_lines)]
#[uefi_macros::entry]
fn main(image_handle: Handle, system_table: SystemTable<Boot>) -> Status {
    if let Err(err) = logging::initialize() {
        return err;
    }

    let mut storage = [0u8; 16384];

    // SAFETY:
    // - `storage` is not used except for this arena.
    // - `storage` is a single stack-allocated object.
    // - `storage` is automatically checked for the size requirement.
    let mut arena = unsafe {
        Arena::new(
            core::ptr::NonNull::from(&mut storage).cast::<u8>(),
            core::mem::size_of_val(&storage),
        )
    };

    let binding = acquire_boot_handle();
    let loaded_image = get_image_protocol(&binding);

    let (loaded_at, size) = loaded_image.info();

    log::info!("{:p} {}", loaded_at, size);
    let mut base_frame = arena.base_frame();

    let mut root_dir = match acquire_loaded_image_directory(&loaded_image) {
        Ok(root_dir) => root_dir,
        Err(err) => {
            log::error!("{err}");
            acquire_boot_handle().stall(ERROR_STALL_TIME);
            return Status::LOAD_ERROR;
        }
    };

    core::mem::drop(loaded_image);
    core::mem::drop(binding);

    log::info!(target: "config", "loading configuration file");
    let result = match load_file(
        &mut root_dir,
        CONFIG_PATH,
        CONFIG_DIGEST,
        &base_frame.next_frame(),
    ) {
        Ok(bytes) => bytes,
        Err(err) => {
            log::error!("{err}");
            acquire_boot_handle().stall(ERROR_STALL_TIME);
            return Into::<Status>::into(err);
        }
    };

    let Ok(config_string) = core::str::from_utf8(result.as_slice()) else {
        log::error!("config file must be utf-8");
        acquire_boot_handle().stall(ERROR_STALL_TIME);
        return Status::INVALID_LANGUAGE;
    };

    let Ok(config) = parse_configuration_file(config_string, &mut base_frame) else {
        log::error!("error parsing config file: ");
        acquire_boot_handle().stall(ERROR_STALL_TIME);
        return Status::INVALID_LANGUAGE;
    };
    log::info!(target: "config", "configuration file parsed");

    set_global_filter(config.logging.global);
    set_serial_filter(config.logging.serial);
    set_framebuffer_filter(config.logging.framebuffer);

    let kernel_path = config.paths.lookup(config.kernel.path);
    log::info!("loading kernel file from {}", kernel_path);
    let kernel_bytes = match load_file(
        &mut root_dir,
        kernel_path,
        config.kernel.checksum,
        &base_frame.next_frame(),
    ) {
        Ok(bytes) => bytes,
        Err(err) => {
            log::error!(target: "filesystem", "{err}");
            acquire_boot_handle().stall(ERROR_STALL_TIME);
            return Into::<Status>::into(err);
        }
    };

    log::info!("kernel file loaded and verified");

    let mut modules = match Vec::with_capacity(config.modules.len()) {
        Ok(modules) => modules,
        Err(err) => {
            log::error!(target: "memory", "{err}");
            acquire_boot_handle().stall(ERROR_STALL_TIME);
            return Status::OUT_OF_RESOURCES;
        }
    };

    for module in config.modules.as_slice() {
        let module_bytes = match load_file(
            &mut root_dir,
            config.paths.lookup(module.path),
            module.checksum,
            &base_frame.next_frame(),
        ) {
            Ok(bytes) => bytes,
            Err(err) => {
                log::error!(target: "filesystem", "{err}");
                acquire_boot_handle().stall(ERROR_STALL_TIME);
                return Into::<Status>::into(err);
            }
        };
        assert!(modules.push_within_capacity(module_bytes).is_ok());
    }
    log::info!(target: "filesystem", "all modules loaded");

    drop(config);
    drop(modules);
    drop(result);
    drop(kernel_bytes);

    logging::prepare_to_exit_boot_services();

    let (system_table, mut memory_map) = system_table.exit_boot_services(MemoryType::LOADER_DATA);

    memory_map.sort();

    let mut region_start = 0;
    let mut region_end = 0;
    let mut region_type = PhysicalMemoryType::Conventional;
    let mut region_att = MemoryAttribute::default();
    let mut first = true;

    for entry in memory_map.entries() {
        let entry_type = match entry.ty {
            MemoryType::CONVENTIONAL
            | MemoryType::BOOT_SERVICES_CODE
            | MemoryType::BOOT_SERVICES_DATA => PhysicalMemoryType::Conventional,
            MemoryType::RUNTIME_SERVICES_CODE | MemoryType::RUNTIME_SERVICES_DATA => {
                PhysicalMemoryType::Runtime
            }
            MemoryType::ACPI_NON_VOLATILE => PhysicalMemoryType::AcpiNonVolatile,
            MemoryType::ACPI_RECLAIM => PhysicalMemoryType::AcpiReclaimable,
            custom => PhysicalMemoryType::Custom(custom.0),
        };

        if entry_type == region_type && entry.phys_start == region_end && entry.att == region_att {
            region_end += entry.page_count * PAGE_SIZE as u64;
        } else {
            if !first {
                log::debug!(
                    "Physical Start: {:X}; Page Count: {}: Type: {:?}",
                    region_start,
                    (region_end - region_start) / PAGE_SIZE as u64,
                    region_type
                );
            }

            region_start = entry.phys_start;
            region_end = entry.phys_start + entry.page_count * PAGE_SIZE as u64;
            region_type = entry_type;
            region_att = entry.att;

            first = false;
        }
    }

    loop {
        core::hint::spin_loop();
    }
}

/// Returns the [`LoadedImage`] protocol associated with this binary.
fn get_image_protocol(boot_handle: &uefi::boot::BootHandle) -> ScopedProtocol<LoadedImage> {
    let image_handle = boot_handle.image_handle();

    let open_params = uefi::table::boot::OpenProtocolParams {
        agent: image_handle,
        controller: None,
        handle: image_handle,
    };

    // SAFETY:
    // `image_handle` will remain valid until usage ends because this image is `image_handle`.
    // `image_handle` should always have the `LoadedImage` protocol, since it is a loaded image.
    unsafe {
        boot_handle
            .open_protocol::<LoadedImage>(
                open_params,
                uefi::table::boot::OpenProtocolAttributes::GetProtocol,
            )
            .expect("`image_handle` must support the `LoadedImage` protocol")
    }
}

/// The panic handler for the bootloader.
///
/// Logs the panic message and then spins.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!(target: "panic", "{info}");

    loop {
        core::hint::spin_loop();
    }
}
