//! Script to configure the building of the kernel.

fn main() {
    // Tell cargo to pass the linker script to the linker..
    println!("cargo:rustc-link-arg=-Tkernel/kernel.ld");
    // ..and to re-run if it changes.
    println!("cargo:rerun-if-changed=kernel/kernel.ld");
    println!("cargo:rerun-if-changed=kernel/build.rs");
}
