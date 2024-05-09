//! Build script for the zeta kernel.

fn main() {
    println!("cargo::rerun-if-changed=kernel/build.rs");

    // When testing, we shouldn't use the custom linker scripts.
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "none" {
        #[cfg(target_arch = "x86_64")]
        {
            println!("cargo::rerun-if-changed=kernel/linker_scripts/x86_64.ld");
            println!("cargo::rustc-link-arg=-Tkernel/linker_scripts/x86_64.ld");
        }
    }
}
