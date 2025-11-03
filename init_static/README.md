# init_static

[![Crates.io](https://img.shields.io/crates/v/init_static.svg)](https://crates.io/crates/init_static)
[![Documentation](https://docs.rs/init_static/badge.svg)](https://docs.rs/init_static)

A Rust library for explicit static initialization.

## Overview

`init_static` provides a macro similar to [`lazy_static`](https://crates.io/crates/lazy_static), but with **explicit initialization control**. This means you decide **when** your statics are initialized, rather than having them lazily
evaluate on first use.

Unlike `lazy_static`, `init_static` uses [`std::sync::OnceLock`](https://doc.rust-lang.org/std/sync/struct.OnceLock.html) internally and **does not** initialize your static values automatically. Instead, it gathers your initialization functions at compile-time using
[`linkme`](https://crates.io/crates/linkme), and provides a function to run them all at once early in your program (for example, inside `main()`).

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
init_static = { version = "0.1" }
```

# Example

```rust
use init_static::init_static;
use std::error::Error;

init_static! {
    static VALUE: u32 = "42".parse()?;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init_static().await?;
    println!("{}", *VALUE);
    Ok(())
}
```
