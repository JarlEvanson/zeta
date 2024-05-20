set -e

qemu-system-x86_64 \
    -enable-kvm \
    -m 3G \
    -drive if=pflash,file=run/OVMF/CODE.fd,format=raw,readonly=on \
    -drive if=pflash,file=run/OVMF/CODE.fd,format=raw,readonly=on \
    -drive file=fat:rw:run/fat_dir,format=raw,media=disk \
    -serial file:run/outputs/serial.txt \
    -debugcon file:run/outputs/debugcon.txt