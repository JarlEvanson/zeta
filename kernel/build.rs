//! Build script for the zeta kernel.

fn main() {
    println!("cargo::rerun-if-changed=kernel/build.rs");

    #[cfg(target_arch = "x86_64")]
    {
        println!("cargo::rerun-if-changed=kernel/linker_scripts/x86_64.ld");
        println!("cargo::rustc-link-arg=-Tkernel/linker_scripts/x86_64.ld");
    }
}
