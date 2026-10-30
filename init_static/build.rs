use std::env::var;

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rustc-check-cfg=cfg(init_static_gc_enabled)");

    // `init_static_gc` is only honored where the anchor + `SHF_LINK_ORDER` scheme is known to
    // work. Everywhere else the crate silently falls back to the default (linkme) mode. Keep this
    // condition in sync with the `linkme` dependency in `Cargo.toml`.
    let os = var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let arch = var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if var("CARGO_CFG_INIT_STATIC_GC").is_ok()
        && matches!(os.as_str(), "linux" | "android")
        && matches!(arch.as_str(), "x86_64" | "aarch64")
    {
        println!("cargo::rustc-cfg=init_static_gc_enabled");
    }
}
