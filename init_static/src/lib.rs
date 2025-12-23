#[doc = include_str!("../README.md")]
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{Debug, Display};
use std::sync::Mutex;

use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;

use crate::__private::{INIT, InitFn};
pub use crate::init_static::{InitStatic, Symbol};

mod init_static;

/// Macro to declare statically stored values with explicit initialization. Similar to
/// [`lazy_static!`](lazy_static::lazy_static!), but initialization is not automatic.
///
/// Each static declared using this macro:
///
/// - Wraps the value type in [`InitStatic`](struct@InitStatic)
/// - Generates an init function that sets the value
/// - Registers the init function in a distributed slice
///
/// The values are initialized when [`init_static()`] is called.
///
/// # Example
///
/// ```
/// use init_static::init_static;
///
/// init_static! {
///     static VALUE: u32 = "42".parse()?;
/// }
///
/// #[tokio::main]
/// async fn main() {
///     init_static().await.unwrap();
///     println!("{}", *VALUE);
/// }
/// ```
pub use init_static_macro::init_static;

struct InitOptions {
    debug: bool,
}

static INIT_OPTIONS: Mutex<Option<InitOptions>> = Mutex::new(Some(InitOptions { debug: false }));

/// Enables or disables debug output during initialization.
///
/// When debug mode is enabled, the initialization process prints messages
/// to stderr indicating:
///
/// - When each synchronous static is initialized
/// - When each asynchronous static begins and completes initialization
///
/// This is useful for diagnosing initialization order issues or performance
/// problems during startup.
pub fn set_debug(debug: bool) {
    INIT_OPTIONS
        .lock()
        .unwrap()
        .as_mut()
        .expect("INIT_OPTIONS can only be modified before `init_static` is called.")
        .debug = debug;
}

/// Returns whether [`init_static()`] has already been called.
///
/// This function checks if the initialization process has been executed. It returns `true` if
/// [`init_static()`] has been called (regardless of whether it succeeded or failed), and `false`
/// otherwise.
pub fn is_initialized() -> bool {
    INIT_OPTIONS.lock().unwrap().is_none()
}

/// Runs initialization for all statics declared with [`init_static!`].
///
/// This function iterates over all init functions registered via the macro and executes them once.
/// Call this early in your program (e.g., at the beginning of `main()`) before accessing any
/// [`struct@InitStatic`] values.
///
/// # Examples
///
/// ```
/// use init_static::init_static;
///
/// init_static! {
///     static VALUE: u32 = "42".parse()?;
/// }
///
/// #[tokio::main]
/// async fn main() {
///     init_static().await.unwrap();
///     println!("{}", *VALUE);
/// }
/// ```
pub async fn init_static() -> Result<(), InitError> {
    let options = INIT_OPTIONS
        .lock()
        .unwrap()
        .take()
        .expect("`init_static` can only be called once.");

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
                .filter_map(|symbol| Some(*symbol_map.get(symbol?)?))
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
        let mut has_sync = false;
        for i in layer {
            match &INIT[i].init {
                InitFn::Sync(f) => {
                    has_sync = true;
                    if options.debug {
                        eprintln!("init_static: sync {}", INIT[i].symbol);
                    }
                    f()?;
                    for (_, deps) in &mut adjacent {
                        deps.remove(&i);
                    }
                }
                InitFn::Async(f) => join_set.push(async move {
                    if options.debug {
                        eprintln!("init_static: async begin {}", INIT[i].symbol);
                    }
                    let output = f().await;
                    if options.debug {
                        eprintln!("init_static: async end {}", INIT[i].symbol);
                    }
                    output.map(|_| i)
                }),
            }
        }
        if has_sync {
            continue;
        }
        if join_set.is_empty() {
            return Err(InitError::Circular {
                symbols: adjacent.iter().map(|(i, _)| INIT[*i].symbol).collect(),
            });
        }
        let i = join_set.next().await.unwrap()?;
        for (_, deps) in &mut adjacent {
            deps.remove(&i);
        }
    }

    Ok(())
}

/// Error type returned by [`init_static()`] when initialization fails.
///
/// This enum represents the various failure modes that can occur during the static initialization
/// process.
#[derive(Debug)]
pub enum InitError {
    /// A static symbol was defined multiple times.
    ///
    /// This typically occurs when the same [`init_static!`] block is included multiple times, or
    /// when two statics in different modules have the exact same source location metadata (which
    /// should not happen in normal usage).
    Ambiguous { symbol: &'static Symbol },

    /// A circular dependency was detected among statics.
    ///
    /// This occurs when static A depends on static B, and static B (directly or indirectly) depends
    /// on static A. The initialization system cannot determine a valid order to initialize such
    /// statics.
    Circular { symbols: Vec<&'static Symbol> },

    /// An initialization expression returned an error.
    ///
    /// This wraps any [`anyhow::Error`] returned by a static's initialization expression. The
    /// original error is preserved and can be accessed via the [`Error::source`] method.
    Execution(anyhow::Error),
}

impl From<anyhow::Error> for InitError {
    #[inline]
    fn from(e: anyhow::Error) -> Self {
        Self::Execution(e)
    }
}

impl Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ambiguous { symbol } => {
                write!(f, "Symbol {symbol} is defined multiple times.")
            }
            Self::Circular { symbols } => {
                writeln!(f, "Circular dependency detected among:")?;
                for symbol in symbols {
                    writeln!(f, "    {symbol}")?;
                }
                Ok(())
            }
            Self::Execution(e) => Display::fmt(e, f),
        }
    }
}

impl Error for InitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Execution(e) => Some(&**e),
            _ => None,
        }
    }
}

#[doc(hidden)]
pub mod __private {
    use std::pin::Pin;

    pub use {anyhow, linkme};

    use crate::Symbol;
    pub use crate::init_static::MaybeInitStatic;

    pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T>>>;

    pub enum InitFn {
        Sync(fn() -> anyhow::Result<()>),
        Async(fn() -> BoxFuture<anyhow::Result<()>>),
    }

    pub struct Init {
        pub symbol: &'static Symbol,
        pub init: InitFn,
        pub deps: fn() -> Vec<Option<&'static Symbol>>,
    }

    #[linkme::distributed_slice]
    pub static INIT: [Init];
}
