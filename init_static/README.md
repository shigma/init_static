# init_static

[![Crates.io](https://img.shields.io/crates/v/init_static.svg)](https://crates.io/crates/init_static)
[![Documentation](https://docs.rs/init_static/badge.svg)](https://docs.rs/init_static)

A Rust library for explicit static initialization.

## Overview

`init_static` provides a macro similar to [`lazy_static`](https://crates.io/crates/lazy_static), but with **explicit initialization control**. This means you decide **when** your statics are initialized, rather than having them lazily
evaluate on first use.

Unlike `lazy_static`, `init_static` uses [`std::sync::OnceLock`](https://doc.rust-lang.org/std/sync/struct.OnceLock.html) internally and **does not** initialize your static values automatically. Instead, it gathers your initialization functions at link time (using [`linkme`](https://crates.io/crates/linkme), or a dedicated linker section in [GC mode](#link-time-gc)), and provides a function to run them all at once early in your program (for example, inside `main()`).

## Compare to `lazy_static`

| Feature            | `lazy_static`           | `init_static`                     |
| ------------------ | ----------------------- | --------------------------------- |
| Initialization     | Implicit (on first use) | Explicit (`init_static()` call)   |
| `Result` support   | Not supported           | Supported                         |
| Async support      | Not supported           | Supported                         |
| Early failure      | No (fail at runtime)    | Yes (optional, before app starts) |

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
init_static = { version = "0.8" }
```

# Example

```rust
use init_static::init_static;

init_static! {
    static VALUE: u32 = "42".parse()?;
}

#[tokio::main]
async fn main() {
    init_static().await.unwrap();
    println!("{}", *VALUE);
}
```

## Link-time GC

By default, every static declared with `init_static!` anywhere in the final program is initialized, whether or not the program uses it. In GC mode, `init_static()` only initializes the statics the program actually uses (reads or takes the address of, from code reachable from `main`, or from the exported functions of a `cdylib`), and the linker removes all the others, together with their initialization code.

To enable it, add the following to the `.cargo/config.toml` of the project building the final binary:

```toml
[target.'cfg(all(target_os = "linux", target_env = "gnu", target_pointer_width = "64", any(target_arch = "x86_64", target_arch = "aarch64")))']
rustflags = ["--cfg", "init_static_gc"]
```

GC mode is supported for the `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu` targets. Setting the cfg for any other target is a compile error, which is why the example scopes it to supported targets. Note that target-specific `rustflags` replace `[build] rustflags` instead of adding to them: if you already have the latter, repeat its flags in this section.

Pruning also requires the final binary to be linked by lld with `--gc-sections`. Whether rustc uses lld by default depends on the toolchain, not only on the target: only the official toolchain for x86_64 Linux hosts (`<channel>-x86_64-unknown-linux-gnu`, as installed by rustup) links both targets above with its bundled `rust-lld`, while the official toolchains for other hosts, such as aarch64 Linux or macOS, link them through `cc` with the system's default linker. With such a toolchain, install lld and add `-C link-arg=-fuse-ld=lld` to the same `rustflags`. Every static will be kept if the binary is linked by another linker, or without `--gc-sections`.

Statics that are initialized only for their side effects, and never read, should be annotated with `#[keep]`. `#[keep]` has no effect outside GC mode, so libraries can use it regardless of how they are built:

```rust
use init_static::init_static;

fn register_plugin() {}

init_static! {
    #[keep]
    static REGISTER: () = register_plugin();
}
```

`#[after(X)]` and `#[before(X)]` only constrain the order: they never keep `X` or the annotated static, and are ignored if either side is not kept. To make a static depend on `X`, read `X` in its initializer.
