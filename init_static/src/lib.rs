#![doc = include_str!("../README.md")]

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::Context;
use futures_util::StreamExt;
use futures_util::future::{FutureExt, Shared};
use futures_util::stream::FuturesUnordered;

use crate::__private::{BoxFuture, INIT, InitFn};

mod error;
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

pub use crate::error::InitError;
pub use crate::init_static::{InitStatic, Symbol};

struct InitOptions {
    debug: bool,
}

static INIT_OPTIONS: Mutex<Option<InitOptions>> = Mutex::new(Some(InitOptions { debug: false }));

type SharedInit = Shared<BoxFuture<Result<(), Arc<anyhow::Error>>>>;

static INIT_FUTURE: OnceLock<SharedInit> = OnceLock::new();

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
/// This function may be called multiple times. All callers share a single underlying future, so the
/// initialization work runs exactly once; every call resolves to the same [`Arc`]-wrapped result.
/// Because the shared result is cached, a failed initialization is *not* retried on subsequent
/// calls.
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
pub async fn init_static() -> std::result::Result<(), Arc<anyhow::Error>> {
    INIT_FUTURE
        .get_or_init(|| {
            let fut: BoxFuture<_> = Box::pin(async { init_impl().await.map_err(Arc::new) });
            fut.shared()
        })
        .clone()
        .await
}

async fn init_impl() -> anyhow::Result<()> {
    // The shared future drives this exactly once, so `take` never sees `None`.
    let debug = INIT_OPTIONS
        .lock()
        .unwrap()
        .take()
        .expect("init_static ran more than once")
        .debug;

    let mut symbol_map: HashMap<&'static Symbol, usize> = HashMap::new();
    for (i, init) in INIT.iter().enumerate() {
        if symbol_map.insert(init.symbol, i).is_some() {
            return Err(InitError::Ambiguous { symbol: init.symbol }.into());
        }
    }

    let deps = INIT
        .iter()
        .map(|init| {
            (init.deps)()
                .into_iter()
                .filter_map(|symbol| Some(*symbol_map.get(symbol?)?))
                .collect::<HashSet<_>>()
        })
        .collect::<Vec<_>>();

    // Effective priority: a node inherits the highest priority among all nodes that
    // (transitively) depend on it, so a dependency's tier is never lower than its
    // dependent's. Propagate in reverse topological order (dependents before their
    // dependencies) so a single O(V + E) pass suffices. Nodes in a dependency cycle
    // are absent from `order` and keep their declared priority; the cycle is reported
    // later during execution.
    let mut eff = INIT.iter().map(|init| init.priority).collect::<Vec<_>>();
    let mut rdeps = vec![Vec::new(); INIT.len()];
    let mut remaining = vec![0usize; INIT.len()];
    for (i, deps) in deps.iter().enumerate() {
        remaining[i] = deps.len();
        for &k in deps {
            rdeps[k].push(i);
        }
    }
    let mut queue = (0..INIT.len()).filter(|&i| remaining[i] == 0).collect::<VecDeque<_>>();
    let mut order = Vec::with_capacity(INIT.len());
    while let Some(k) = queue.pop_front() {
        order.push(k);
        for &i in &rdeps[k] {
            remaining[i] -= 1;
            if remaining[i] == 0 {
                queue.push_back(i);
            }
        }
    }
    for &i in order.iter().rev() {
        for &k in &deps[i] {
            eff[k] = eff[k].max(eff[i]);
        }
    }

    // Process tiers by descending effective priority; each distinct value is a tier,
    // and a tier fully completes before the next one starts (hard barrier).
    let tiers = eff.iter().copied().collect::<BTreeSet<_>>();

    let mut join_set = FuturesUnordered::new();
    for tier in tiers.into_iter().rev() {
        let members = (0..INIT.len()).filter(|&i| eff[i] == tier).collect::<HashSet<_>>();
        // Dependencies outside this tier belong to higher tiers and are already done.
        let mut adjacent = members
            .iter()
            .map(|&i| (i, deps[i].intersection(&members).copied().collect::<HashSet<_>>()))
            .collect::<Vec<_>>();

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
                        if debug {
                            eprintln!("init_static: sync {}", INIT[i].symbol);
                        }
                        f().with_context(|| format!("failed to initialize {}", INIT[i].symbol))?;
                        for (_, deps) in &mut adjacent {
                            deps.remove(&i);
                        }
                    }
                    InitFn::Async(f) => join_set.push(async move {
                        if debug {
                            eprintln!("init_static: async begin {}", INIT[i].symbol);
                        }
                        let output = f()
                            .await
                            .map(|_| i)
                            .with_context(|| format!("failed to initialize {}", INIT[i].symbol));
                        if debug {
                            eprintln!("init_static: async end {}", INIT[i].symbol);
                        }
                        output
                    }),
                }
            }
            if has_sync {
                continue;
            }
            if join_set.is_empty() {
                let mut symbols = adjacent.iter().map(|(i, _)| INIT[*i].symbol).collect::<Vec<_>>();
                symbols.sort_unstable_by_key(|s| (s.file, s.line, s.column));
                return Err(InitError::Circular { symbols }.into());
            }
            let i = join_set.next().await.unwrap()?;
            for (_, deps) in &mut adjacent {
                deps.remove(&i);
            }
        }
    }

    Ok(())
}

#[doc(hidden)]
pub mod __private {
    use std::pin::Pin;

    pub use anyhow;
    pub use linkme;

    use crate::Symbol;
    pub use crate::init_static::MaybeInitStatic;

    pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

    pub enum InitFn {
        Sync(fn() -> anyhow::Result<()>),
        Async(fn() -> BoxFuture<anyhow::Result<()>>),
    }

    pub struct Init {
        pub symbol: &'static Symbol,
        pub init: InitFn,
        pub deps: fn() -> Vec<Option<&'static Symbol>>,
        /// Initialization priority declared via `#[priority = N]` (default `0`).
        ///
        /// Statics are initialized in descending priority order: higher values run first, negative
        /// values run after the default tier. Real dependencies always take precedence — a
        /// dependency inherits the highest priority among the nodes that depend on it, so it is
        /// never scheduled later than its dependents.
        pub priority: i32,
    }

    #[linkme::distributed_slice]
    pub static INIT: [Init];
}
