use std::error::Error;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::sync::OnceLock;

pub use init_static_macro::init_static;

pub fn init_static() -> Result<(), Box<dyn Error + Send + Sync>> {
    for init_fn in __private::INIT_FUNCTIONS {
        init_fn()?;
    }
    Ok(())
}

pub struct InitStatic<T> {
    inner: OnceLock<T>,
}

impl<T> Default for InitStatic<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> InitStatic<T> {
    pub const fn new() -> Self {
        Self { inner: OnceLock::new() }
    }

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
    pub use linkme;

    use super::*;

    #[linkme::distributed_slice]
    pub static INIT_FUNCTIONS: [fn() -> Result<(), Box<dyn Error + Send + Sync>>];
}
