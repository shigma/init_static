#[doc = include_str!("../README.md")]
use std::fmt::Debug;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::sync::OnceLock;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub module: &'static str,
    pub line: u32,
    pub column: u32,
    pub ident: &'static str,
}

impl Display for Symbol {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (at {}:{}:{})", self.ident, self.module, self.line, self.column)
    }
}

#[macro_export]
macro_rules! InitStatic {
    ($ident:ident) => {
        $crate::InitStatic::new(&$crate::Symbol {
            module: module_path!(),
            line: line!(),
            column: column!(),
            ident: stringify!($ident),
        })
    };
}

/// A wrapper around [`OnceLock`] providing safe initialization and [`Deref`] support to mimic the
/// ergonomics of `lazy_static!`.
///
/// Values must be initialized exactly once, either via [`InitStatic::init`] or by calling
/// [`init_static`]. Accessing an uninitialized value will panic.
pub struct InitStatic<T> {
    symbol: &'static Symbol,
    inner: OnceLock<T>,
}

impl<T> InitStatic<T> {
    /// Creates a new uninitialized `InitStatic`.
    ///
    /// The value must be initialized using [`InitStatic::init`] or via the initialization registry
    /// before access.
    #[inline]
    pub const fn new(symbol: &'static Symbol) -> Self {
        Self {
            symbol,
            inner: OnceLock::new(),
        }
    }

    /// Initializes the given static value.
    ///
    /// This must be called exactly once. Subsequent calls will panic.
    #[inline]
    pub fn init(this: &Self, value: T) {
        this.inner
            .set(value)
            .unwrap_or_else(|_| panic!("InitStatic is already initialized."));
    }

    #[inline]
    pub const fn symbol(this: &Self) -> &'static Symbol {
        this.symbol
    }
}

impl<T> Deref for InitStatic<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.inner
            .get()
            .expect("InitStatic is not initialized. Call `init_static` first!")
    }
}

impl<T> DerefMut for InitStatic<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
            .get_mut()
            .expect("InitStatic is not initialized. Call `init_static` first!")
    }
}

impl<T: Debug> Debug for InitStatic<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("InitStatic").field(&**self).finish()
    }
}

impl<T: Display> Display for InitStatic<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&**self, f)
    }
}

pub trait MaybeInitStatic {
    fn __get_symbol(&self) -> Option<&'static Symbol>;
}

impl<T> MaybeInitStatic for InitStatic<T> {
    #[inline]
    fn __get_symbol(&self) -> Option<&'static Symbol> {
        Some(self.symbol)
    }
}

impl<T> MaybeInitStatic for &T {
    #[inline]
    fn __get_symbol(&self) -> Option<&'static Symbol> {
        None
    }
}
