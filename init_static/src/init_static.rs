#[doc = include_str!("../README.md")]
use std::fmt::Debug;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::sync::OnceLock;

/// Represents the source location and identity of a static variable declared via
/// [`init_static!`](crate::init_static).
///
/// This struct captures compile-time metadata about where a static was defined,
/// enabling meaningful error messages and debugging output during initialization.
///
/// # Example
///
/// ```
/// use init_static::Symbol;
///
/// let symbol: &Symbol = Symbol!(MY_VALUE);
/// println!("{symbol}"); // MY_VALUE (at src/main.rs:10:1)
/// ```
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Symbol {
    /// The source file path where this static is declared.
    ///
    /// See also: [`file!`](file)
    pub file: &'static str,
    /// The line number of the declaration.
    ///
    /// See also: [`line!`](line)
    pub line: u32,
    /// The column number of the declaration.
    ///
    /// See also: [`column!`](column)
    pub column: u32,
    /// The full module path containing this static.
    ///
    /// See also: [`module_path!`](module_path)
    pub module: &'static str,
    /// The identifier name of the static variable.
    pub ident: &'static str,
}

impl Display for Symbol {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (at {}:{}:{})", self.ident, self.file, self.line, self.column)
    }
}

/// Creates a [`Symbol`] reference capturing the current source location for the given identifier.
///
/// This macro is primarily used internally by [`init_static!`](crate::init_static) to record
/// metadata about each declared static variable. It captures compile-time information using
/// the standard location macros ([`file!`](file), [`line!`](line), [`column!`](column) and
/// [`module_path!`](module_path)).
///
/// # Example
///
/// ```
/// use init_static::{Symbol};
///
/// let symbol: &Symbol = Symbol!(MY_VALUE);
/// assert_eq!(symbol.ident, "MY_VALUE");
/// assert!(symbol.file.ends_with(".rs"));
/// ```
///
/// # Note
///
/// This macro returns a [`&'static Symbol`](Symbol) reference, which is suitable for use in
/// static contexts and error reporting.
#[macro_export]
macro_rules! Symbol {
    ($ident:ident) => {
        &$crate::Symbol {
            file: file!(),
            line: line!(),
            column: column!(),
            module: module_path!(),
            ident: stringify!($ident),
        }
    };
}

/// Creates a new uninitialized [`InitStatic<T>`] instance with source location metadata.
///
/// This macro is a convenience wrapper around [`InitStatic::new`] that automatically
/// captures the source location using the [`Symbol!`] macro.
///
/// # Example
///
/// ```
/// use init_static::InitStatic;
///
/// struct Config; // Placeholder for some configuration type
///
/// static MY_CONFIG: InitStatic<Config> = InitStatic!(MY_CONFIG);
///
/// // The static is uninitialized and will panic if accessed before initialization.
/// // Use `InitStatic::init(&MY_CONFIG, value)` to initialize it.
/// ```
#[macro_export]
macro_rules! InitStatic {
    ($ident:ident) => {
        $crate::InitStatic::new($crate::Symbol!($ident))
    };
}

/// A wrapper around [`OnceLock`] providing safe initialization and [`Deref`] support to mimic the
/// ergonomics of [`lazy_static!`](lazy_static::lazy_static).
///
/// Values must be initialized exactly once, either via [`InitStatic::init`] or by calling
/// [`init_static`](crate::init_static). Accessing an uninitialized value will panic.
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
            .unwrap_or_else(|_| panic!("Double initialization of init_static: {}", this.symbol));
    }

    /// Returns the [`Symbol`] associated with this static, containing source location metadata.
    ///
    /// This method provides access to compile-time information about where the static was
    /// declared, which is useful for debugging, logging, and error reporting.
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
            .unwrap_or_else(|| panic!("Access to uninitialized init_static: {}", self.symbol))
    }
}

impl<T> DerefMut for InitStatic<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
            .get_mut()
            .unwrap_or_else(|| panic!("Access to uninitialized init_static: {}", self.symbol))
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
