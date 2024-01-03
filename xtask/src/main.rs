//! An integrated utility script meant to simplify testing and updates to the zeta project and its binaries.

use config::update_config_checksum;

mod config;

fn main() {
    let config_file = std::fs::read("testing/boot_dir/zeta/config.toml").unwrap();

    let mut executable = std::fs::File::options()
        .write(true)
        .read(true)
        .open("testing/boot_dir/efi/boot/BOOTX64.EFI")
        .unwrap();

    update_config_checksum(
        &mut executable,
        digest::sha512::bytes::Sha512::hash(&config_file),
    )
    .unwrap()
}
