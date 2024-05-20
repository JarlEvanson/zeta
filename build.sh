set -e

# Build the bootloader
cargo build --package bootloader --target x86_64-unknown-uefi --release
cp target/x86_64-unknown-uefi/release/bootloader.efi run/fat_dir/EFI/BOOT/BOOTX64.EFI

# Build the kernel
cargo build --package kernel --target x86_64-unknown-none --release
cp target/x86_64-unknown-none/release/kernel run/fat_dir/zeta