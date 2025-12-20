#[doc = include_str!("../README.md")]
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{Debug, Display};
use std::ops::{Deref, DerefMut};
use std::sync::OnceLock;

use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;
pub use init_static_macro::init_static;

use crate::__private::INIT;

// TODO: custom impl for Debug?
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Symbol {
    module: &'static str,
    line: u32,
    column: u32,
    ident: &'static str,
}

#[macro_export]
macro_rules! symbol {
    ($ident:ident) => {
        &$crate::Symbol {
            module: module_path!(),
            line: line!(),
            column: column!(),
            ident: stringify!($ident),
        }
    };
}

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
pub async fn init_static() -> Result<(), InitError> {
    let mut symbol_map: HashMap<&'static Symbol, usize> = HashMap::new();
    for (i, init) in INIT.iter().enumerate() {
        if symbol_map.insert(init.symbol, i).is_some() {
            return Err(InitError::Ambiguous { symbol: init.symbol });
        }
    }
    let mut adjacent = INIT
        .iter()
        .enumerate()
        .map(|(i, init)| {
            let deps = (init.deps)()
                .into_iter()
                .filter_map(|symbol| symbol_map.get(symbol).copied())
                .collect::<HashSet<_>>();
            (i, deps)
        })
        .collect::<Vec<_>>();
    let mut join_set = FuturesUnordered::new();
    while !adjacent.is_empty() || !join_set.is_empty() {
        let layer = adjacent
            .extract_if(.., |(_, deps)| deps.is_empty())
            .map(|(i, _)| i)
            .collect::<HashSet<_>>();
        for (_, deps) in &mut adjacent {
            deps.retain(|dep| !layer.contains(dep));
        }
        join_set.extend(layer.into_iter().map(|i| (INIT[i].init)()));
        if join_set.is_empty() {
            return Err(InitError::Circular {
                symbols: adjacent.iter().map(|(i, _)| INIT[*i].symbol).collect(),
            });
        }
        join_set.next().await.unwrap().map_err(InitError::InitializationError)?;
    }
    Ok(())
}

#[derive(Debug)]
pub enum InitError {
    Ambiguous { symbol: &'static Symbol },
    Circular { symbols: Vec<&'static Symbol> },
    InitializationError(anyhow::Error),
}

impl Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitError::Ambiguous { symbol } => {
                write!(f, "Symbol {symbol:?} is referenced by multiple InitStatic variables.")
            }
            InitError::Circular { symbols } => {
                write!(f, "Circular dependency detected among: {:?}", symbols)
            }
            InitError::InitializationError(e) => Display::fmt(e, f),
        }
    }
}

impl Error for InitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            InitError::InitializationError(e) => Some(&**e),
            _ => None,
        }
    }
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
}

impl<T> Deref for InitStatic<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.inner
            .get()
            .expect("InitStatic is not initialized. Call init_static() first!")
    }
}

impl<T> DerefMut for InitStatic<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
            .get_mut()
            .expect("InitStatic is not initialized. Call init_static() first!")
    }
}

impl<T: Debug> Debug for InitStatic<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("InitStatic").field(&**self).finish()
    }
}

#[doc(hidden)]
pub mod __private {
    use std::pin::Pin;

    pub use {anyhow, linkme};

    use crate::{InitStatic, Symbol};

    type BoxFuture<T> = Pin<Box<dyn Future<Output = T>>>;

    pub struct Init {
        pub symbol: &'static Symbol,
        pub init: fn() -> BoxFuture<anyhow::Result<()>>,
        pub deps: fn() -> Vec<&'static Symbol>,
    }

    #[linkme::distributed_slice]
    pub static INIT: [Init];

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
}
