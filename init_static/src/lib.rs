#[doc = include_str!("../README.md")]
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{Debug, Display};
use std::sync::Mutex;

use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;
pub use init_static_macro::init_static;

use crate::__private::INIT;
pub use crate::init_static::{InitStatic, Symbol};

mod init_static;

struct InitOptionsInner {
    debug: bool,
}

pub struct InitOptions {
    inner: Mutex<Option<InitOptionsInner>>,
}

impl InitOptions {
    pub fn debug(&self, debug: bool) {
        self.inner
            .lock()
            .unwrap()
            .as_mut()
            .expect("INIT_OPTIONS can only be modified before `init_static` is called.")
            .debug = debug;
    }
}

pub static INIT_OPTIONS: InitOptions = InitOptions {
    inner: Mutex::new(Some(InitOptionsInner { debug: false })),
};

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
    let options = INIT_OPTIONS
        .inner
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
        for (_, deps) in &mut adjacent {
            deps.retain(|dep| !layer.contains(dep));
        }
        join_set.extend(layer.into_iter().map(|i| {
            if options.debug {
                eprintln!("init_static: {}", INIT[i].symbol);
            }
            (INIT[i].init)()
        }));
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
                write!(f, "Symbol {symbol} is defined multiple times.")
            }
            InitError::Circular { symbols } => {
                writeln!(f, "Circular dependency detected among:")?;
                for symbol in symbols {
                    writeln!(f, "    {symbol}")?;
                }
                Ok(())
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

#[doc(hidden)]
pub mod __private {
    use std::pin::Pin;

    pub use {anyhow, linkme};

    use crate::Symbol;
    pub use crate::init_static::MaybeInitStatic;

    pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T>>>;

    pub struct Init {
        pub symbol: &'static Symbol,
        pub init: fn() -> BoxFuture<anyhow::Result<()>>,
        pub deps: fn() -> Vec<Option<&'static Symbol>>,
    }

    #[linkme::distributed_slice]
    pub static INIT: [Init];
}
