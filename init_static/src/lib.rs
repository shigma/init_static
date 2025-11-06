#[doc = include_str!("../README.md")]
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{Debug, Display};
use std::ops::{Deref, DerefMut};
use std::sync::OnceLock;

use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;
pub use init_static_macro::init_static;

use crate::__private::{BoxError, INIT};

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
    let mut name_map: HashMap<&'static str, Vec<usize>> = HashMap::new();
    for (i, init) in INIT.iter().enumerate() {
        for name in init.names {
            name_map.entry(name).or_default().push(i);
        }
    }
    let mut adjacent = INIT
        .iter()
        .enumerate()
        .map(|(i, init)| {
            let deps = init
                .deps
                .iter()
                .filter_map(|name| {
                    let indices = name_map.get(name)?;
                    if indices.len() > 1 {
                        Some(Err(InitError::AmbiguousDependency {
                            dependents: init.names,
                            dependency: name,
                        }))
                    } else {
                        Some(Ok(indices[0]))
                    }
                })
                .collect::<Result<Vec<_>, _>>();
            match deps {
                Ok(deps) => Ok((i, deps)),
                Err(e) => Err(e),
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
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
            return Err(InitError::CircularDependency {
                names: adjacent
                    .iter()
                    .flat_map(|(i, _)| INIT[*i].names.iter().cloned())
                    .collect(),
            });
        }
        join_set.next().await.unwrap().map_err(InitError::InitializationError)?;
    }
    Ok(())
}

#[derive(Debug)]
pub enum InitError {
    AmbiguousDependency {
        dependents: &'static [&'static str],
        dependency: &'static str,
    },
    CircularDependency {
        names: Vec<&'static str>,
    },
    InitializationError(BoxError),
}

impl Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitError::AmbiguousDependency { dependents, dependency } => {
                write!(
                    f,
                    "Cannot determine dependency {dependency} for {dependents:?}: multiple candidates found.",
                )
            }
            InitError::CircularDependency { names } => {
                write!(f, "Circular dependency detected among: {:?}", names)
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
        Debug::fmt(&self.inner, f)
    }
}

#[doc(hidden)]
pub mod __private {
    use std::pin::Pin;

    pub use linkme;

    use super::*;

    #[cfg(all(feature = "send", feature = "sync"))]
    pub type BoxError = Box<dyn Error + Send + Sync>;
    #[cfg(all(feature = "send", not(feature = "sync")))]
    pub type BoxError = Box<dyn Error + Send>;
    #[cfg(all(not(feature = "send"), feature = "sync"))]
    pub type BoxError = Box<dyn Error + Sync>;
    #[cfg(all(not(feature = "send"), not(feature = "sync")))]
    pub type BoxError = Box<dyn Error>;

    pub struct Init {
        #[expect(clippy::type_complexity)]
        pub init: fn() -> Pin<Box<dyn Future<Output = Result<(), BoxError>>>>,
        pub names: &'static [&'static str],
        pub deps: &'static [&'static str],
    }

    #[linkme::distributed_slice]
    pub static INIT: [Init];
}
