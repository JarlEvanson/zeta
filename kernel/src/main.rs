//! The kernel for zeta.

#![no_std]
#![no_main]

use core::mem::MaybeUninit;

mod const_parsing;
mod memory;

/// Zeta expects the latest version of the Limine protocol.
#[export_name = "LIMINE_BASE_REVISION"]
pub static LIMINE_BASE_REVISION: limine::BaseRevision = limine::BaseRevision::new();

/// Zeta requires the physical and virtual base addresses of the kernel.
#[export_name = "LIMINE_KERNEL_ADDRESS"]
pub static LIMINE_KERNEL_ADDRESS: limine::request::KernelAddressRequest =
    limine::request::KernelAddressRequest::new();

/// Zeta requires the physical memory map.
#[export_name = "LIMINE_KERNEL_ADDRESS"]
pub static LIMINE_MEMORY_MAP: limine::request::MemoryMapRequest =
    limine::request::MemoryMapRequest::new();

pub static mut MEMORY_MAP: [MaybeUninit<MemoryMapEntry>; 100] = [MaybeUninit::uninit(); 100];

#[derive(Clone, Copy)]
struct MemoryMapEntry {
    pub base: u64,
    pub length: u64,
    pub kind: u64,
}

#[export_name = "_start"]
extern "C" fn main() -> ! {
    if !LIMINE_BASE_REVISION.is_supported() {
        fail();
    }

    let Some(response) = LIMINE_MEMORY_MAP.get_response() else {
        fail();
    };

    let entries = response.entries();

    for (index0, entry0) in entries.iter().enumerate() {
        let mut new_entry = MemoryMapEntry {
            base: entry0.base,
            length: entry0.length,
            kind: 0,
        };
        for index1 in (index0 + 1)..entries.len() {
            let entry1 = entries[index1];
        }
    }

    loop {
        core::hint::spin_loop();
    }
}

pub fn format_address(value: u64, buffer: &mut [u8; 16]) {
    const HEX_CHARS: [u8; 16] = [
        b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'A', b'B', b'C', b'D', b'E',
        b'F',
    ];

    buffer[0] = HEX_CHARS[((value >> 60) & 0xF) as usize];
    buffer[1] = HEX_CHARS[((value >> 56) & 0xF) as usize];
    buffer[2] = HEX_CHARS[((value >> 52) & 0xF) as usize];
    buffer[3] = HEX_CHARS[((value >> 48) & 0xF) as usize];
    buffer[4] = HEX_CHARS[((value >> 44) & 0xF) as usize];
    buffer[5] = HEX_CHARS[((value >> 40) & 0xF) as usize];
    buffer[6] = HEX_CHARS[((value >> 36) & 0xF) as usize];
    buffer[7] = HEX_CHARS[((value >> 32) & 0xF) as usize];
    buffer[8] = HEX_CHARS[((value >> 28) & 0xF) as usize];
    buffer[9] = HEX_CHARS[((value >> 24) & 0xF) as usize];
    buffer[10] = HEX_CHARS[((value >> 20) & 0xF) as usize];
    buffer[11] = HEX_CHARS[((value >> 16) & 0xF) as usize];
    buffer[12] = HEX_CHARS[((value >> 12) & 0xF) as usize];
    buffer[13] = HEX_CHARS[((value >> 8) & 0xF) as usize];
    buffer[14] = HEX_CHARS[((value >> 4) & 0xF) as usize];
    buffer[15] = HEX_CHARS[((value >> 0) & 0xF) as usize];
}

fn fail() -> ! {
    loop {}
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    fail()
}
