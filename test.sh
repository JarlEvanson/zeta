#!/bin/sh

set -e

export TEST_BASE=testing
export RUN_BASE=$TEST_BASE/boot_dir

export PRE_CONFIG_GLOBAL="trace"
export PRE_CONFIG_SERIAL="trace" 
export PRE_CONFIG_FRAMEBUFFER="trace"

cargo build --bin bootloader --target x86_64-unknown-uefi --release

mkdir -p $RUN_BASE/efi/boot

cp target/x86_64-unknown-uefi/release/bootloader.efi $RUN_BASE/efi/boot/BOOTX64.EFI

cargo xtask update-checksum $1 $2

mkdir -p $TEST_BASE/outputs

qemu-system-x86_64 -enable-kvm \
    -cpu host \
    -m 20G \
    -drive if=pflash,format=raw,readonly=on,file=$TEST_BASE/OVMF/CODE.fd \
    -drive if=pflash,format=raw,readonly=on,file=$TEST_BASE/OVMF/VARS.fd \
    -drive file=fat:rw:$RUN_BASE,format=raw,media=disk \
    -serial file:$TEST_BASE/outputs/serial.txt \
    -debugcon file:$TEST_BASE/outputs/debugcon.txt