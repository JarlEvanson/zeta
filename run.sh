#!/bin/bash

set -e

cp target/x86_64-unknown-none/release/kernel run/fat_dir/zeta

qemu-system-x86_64 \
    -m 10G \
    -s  -no-shutdown \
    -enable-kvm \
    -drive if=pflash,format=raw,readonly=on,file=run/OVMF/CODE.fd,readonly=on \
    -drive if=pflash,format=raw,readonly=on,file=run/OVMF/VARS.fd \
    -drive file=fat:rw:run/fat_dir/,format=raw,media=disk \
    -d mmu,guest_errors,cpu_reset,int,exec \
    -D run/outputs/log.txt \
    -serial file:run/outputs/serial.txt \
    -debugcon file:run/outputs/debugcon.txt
