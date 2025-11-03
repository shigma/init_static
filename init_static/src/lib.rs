#[doc = include_str!("../README.md")]
use std::error::Error;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::sync::OnceLock;

pub use init_static_macro::init_static;

/// Runs initialization for all statics declared with [`init_static!`].
///
/// This function iterates over all init functions registered via the macro and executes them once.
/// Call this early in your program (e.g., at the beginning of `main()`) before accessing any
/// [`InitStatic`] values.
///
/// # Examples
///
/// ```
/// use init_static::init_static;
/// use std::error::Error;
///
/// init_static! {
///     static VALUE: u32 = "42".parse()?;
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn Error>> {
///     init_static().await?;
///     println!("{}", *VALUE);
///     Ok(())
/// }
/// ```
pub async fn init_static() -> Result<(), Box<dyn Error>> {
    for init_fn in __private::INIT_FUNCTIONS {
        init_fn().await?;
    }
    Ok(())
}

/// A wrapper around [`OnceLock`] providing safe initialization and [`Deref`] support to mimic the
/// ergonomics of `lazy_static!`.
///
/// Values must be initialized exactly once, either via [`InitStatic::init`] or by calling
/// [`init_static`]. Accessing an uninitialized value will panic.
pub struct InitStatic<T> {
    inner: OnceLock<T>,
}

impl<T> Default for InitStatic<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> InitStatic<T> {
    /// Creates a new uninitialized `InitStatic`.
    ///
    /// The value must be initialized using [`InitStatic::init`] or via the initialization registry
    /// before access.
    pub const fn new() -> Self {
        Self { inner: OnceLock::new() }
    }

    /// Initializes the given static value.
    ///
    /// This must be called exactly once. Subsequent calls will panic.
    pub fn init(this: &Self, value: T) {
        this.inner
            .set(value)
            .unwrap_or_else(|_| panic!("InitStatic is already initialized."));
    }
}

impl<T> Deref for InitStatic<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner
            .get()
            .expect("InitStatic is not initialized. Call init_static() first!")
    }
}

impl<T> DerefMut for InitStatic<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
            .get_mut()
            .expect("InitStatic is not initialized. Call init_static() first!")
    }
}

impl<T: Debug> Debug for InitStatic<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

#[doc(hidden)]
pub mod __private {
    use std::pin::Pin;

    pub use linkme;

    use super::*;

    #[linkme::distributed_slice]
    pub static INIT_FUNCTIONS: [fn() -> Pin<Box<dyn Future<Output = Result<(), Box<dyn Error>>>>>];
}
