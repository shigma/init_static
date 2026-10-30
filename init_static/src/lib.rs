#![doc = include_str!("../README.md")]

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::Context;
use futures_util::StreamExt;
use futures_util::future::{FutureExt, Shared};
use futures_util::stream::FuturesUnordered;

use crate::__private::{BoxFuture, Init, InitFn};

mod error;
mod init_static;

pub use init_static_macro::{after, before, init_static, keep, priority};

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

    let inits = __private::registry();

    let mut symbol_map: HashMap<&'static Symbol, usize> = HashMap::new();
    for (i, init) in inits.iter().enumerate() {
        if symbol_map.insert(init.symbol, i).is_some() {
            return Err(InitError::Ambiguous { symbol: init.symbol }.into());
        }
    }

    let mut deps = vec![HashSet::new(); inits.len()];
    for (i, init) in inits.iter().enumerate() {
        for dep in (init.deps)().into_iter().flatten() {
            match symbol_map.get(dep.symbol) {
                Some(&k) => {
                    deps[i].insert(k);
                }
                // An anchored static is kept by the linker exactly when its record is, and this
                // record refers to it, so its record should be here too. If it is not, the linker
                // dropped a static that is actually used. Anything else without a record (e.g. a
                // hand-made `InitStatic`) was never registered, and is not ours to initialize.
                None if dep.anchored => {
                    let (symbol, required_by) = (dep.symbol, init.symbol);
                    return Err(InitError::Missing { symbol, required_by }.into());
                }
                None => {}
            }
        }
    }
    // `#[after(X)]` is an ordinary dependency edge; `#[before(X)]` is its mirror, so it is
    // recorded on `X` instead. Duplicates of an inferred edge collapse into the same `HashSet`.
    // Both only constrain the order, so a target without a record is ignored. In GC mode, that is
    // simply a static nobody uses.
    let ignore = |name: &str, symbol: &Symbol, init: &Init| {
        if cfg!(init_static_gc_enabled) && debug {
            eprintln!(
                "init_static: {symbol} is not registered, ignoring #[{name}] on {}",
                init.symbol
            );
        }
    };
    for (i, init) in inits.iter().enumerate() {
        for symbol in init.after {
            match symbol_map.get(symbol) {
                Some(&k) => {
                    deps[i].insert(k);
                }
                None => ignore("after", symbol, init),
            }
        }
        for symbol in init.before {
            match symbol_map.get(symbol) {
                Some(&k) => {
                    deps[k].insert(i);
                }
                None => ignore("before", symbol, init),
            }
        }
    }
    let deps = deps;

    // Effective priority: a node inherits the highest priority among all nodes that
    // (transitively) depend on it, so a dependency's tier is never lower than its
    // dependent's. Propagate in reverse topological order (dependents before their
    // dependencies) so a single O(V + E) pass suffices. Nodes in a dependency cycle
    // are absent from `order` and keep their declared priority; the cycle is reported
    // later during execution.
    let mut eff = inits.iter().map(|init| init.priority).collect::<Vec<_>>();
    let mut rdeps = vec![Vec::new(); inits.len()];
    let mut remaining = vec![0usize; inits.len()];
    for (i, deps) in deps.iter().enumerate() {
        remaining[i] = deps.len();
        for &k in deps {
            rdeps[k].push(i);
        }
    }
    let mut queue = (0..inits.len()).filter(|&i| remaining[i] == 0).collect::<VecDeque<_>>();
    let mut order = Vec::with_capacity(inits.len());
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
        let members = (0..inits.len()).filter(|&i| eff[i] == tier).collect::<HashSet<_>>();
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
                let init = inits[i];
                match &init.init {
                    InitFn::Sync(f) => {
                        has_sync = true;
                        if debug {
                            eprintln!("init_static: sync {}", init.symbol);
                        }
                        f().with_context(|| format!("failed to initialize {}", init.symbol))?;
                        for (_, deps) in &mut adjacent {
                            deps.remove(&i);
                        }
                    }
                    InitFn::Async(f) => join_set.push(async move {
                        if debug {
                            eprintln!("init_static: async begin {}", init.symbol);
                        }
                        let output = f()
                            .await
                            .map(|_| i)
                            .with_context(|| format!("failed to initialize {}", init.symbol));
                        if debug {
                            eprintln!("init_static: async end {}", init.symbol);
                        }
                        output
                    }),
                }
            }
            if has_sync {
                continue;
            }
            if join_set.is_empty() {
                let mut symbols = adjacent.iter().map(|(i, _)| inits[*i].symbol).collect::<Vec<_>>();
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

    // Each mode defines `declare!` and `registry()`, plus whatever their expansion and
    // implementation need. Only the one in effect is compiled.
    #[cfg(not(init_static_gc_enabled))]
    pub(crate) use self::default::registry;
    #[cfg(not(init_static_gc_enabled))]
    pub use self::default::{INIT, declare, linkme};
    #[cfg(init_static_gc_enabled)]
    pub use self::gc::declare;
    #[cfg(init_static_gc_enabled)]
    pub(crate) use self::gc::registry;
    use crate::Symbol;
    pub use crate::init_static::{Dep, MaybeInitStatic};

    pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

    pub enum InitFn {
        Sync(fn() -> anyhow::Result<()>),
        Async(fn() -> BoxFuture<anyhow::Result<()>>),
    }

    pub struct Init {
        pub symbol: &'static Symbol,
        pub init: InitFn,
        pub deps: fn() -> Vec<Option<Dep>>,
        pub before: &'static [&'static Symbol],
        pub after: &'static [&'static Symbol],
        pub priority: i32,
    }

    /// The default mode: every record is collected into a linkme slice.
    #[cfg(not(init_static_gc_enabled))]
    mod default {
        pub use linkme;

        use super::Init;

        #[linkme::distributed_slice]
        pub static INIT: [Init];

        /// Declares a static together with its registration record. Invoked by
        /// [`init_static!`](crate::init_static!) as
        ///
        /// ```ignore
        /// declare! {
        ///     keep: false,
        ///     static: { pub static FOO: InitStatic<u32> },
        ///     symbol: Symbol!(FOO),
        ///     entry: INIT_FOO = { /* the `Init` record */ },
        /// }
        /// ```
        ///
        /// The macro is defined here instead of being expanded by the proc macro directly, so that
        /// the mode is decided by the `init_static_gc` cfg of this crate alone.
        ///
        /// The record is added to the linkme slice `INIT`, which keeps every record, so `keep` has
        /// no effect.
        #[macro_export]
        #[doc(hidden)]
        macro_rules! __declare {
            (
                keep: $keep:ident,
                static: { $($decl:tt)* },
                symbol: $symbol:expr,
                entry: $entry:ident = $init:expr $(,)?
            ) => {
                $($decl)* = $crate::InitStatic::new($symbol);

                const _: () = {
                    #[$crate::__private::linkme::distributed_slice($crate::__private::INIT)]
                    #[linkme(crate = $crate::__private::linkme)]
                    static $entry: $crate::__private::Init = $init;
                };
            };
        }

        pub use __declare as declare;

        /// Returns every registration record that made it into the final binary.
        pub(crate) fn registry() -> Vec<&'static Init> {
            INIT.iter().collect()
        }
    }

    /// GC mode: every record is referenced by a slot in the `init_static_gc_0_8` section, which
    /// the linker keeps only if the static is used.
    #[cfg(init_static_gc_enabled)]
    mod gc {
        use super::Init;

        /// Declares a static together with its registration record. Invoked by
        /// [`init_static!`](crate::init_static!) as
        ///
        /// ```ignore
        /// declare! {
        ///     keep: false,
        ///     static: { pub static FOO: InitStatic<u32> },
        ///     symbol: Symbol!(FOO),
        ///     entry: INIT_FOO = { /* the `Init` record */ },
        /// }
        /// ```
        ///
        /// The macro is defined here instead of being expanded by the proc macro directly, so that
        /// the mode is decided by the `init_static_gc` cfg of this crate alone.
        ///
        /// The record is referenced by a slot in the `init_static_gc_0_8` section. With `keep:
        /// true`, the slot is `#[used]` and therefore a GC root. Otherwise it is emitted by an
        /// anchor function with `SHF_LINK_ORDER` pointing at that function, so the linker keeps
        /// the slot only if it keeps the anchor; and the static refers to its anchor, so this
        /// happens exactly when the static itself is kept.
        #[macro_export]
        #[doc(hidden)]
        macro_rules! __declare {
            (
                keep: false,
                static: { $($decl:tt)* },
                symbol: $symbol:expr,
                entry: $entry:ident = $init:expr $(,)?
            ) => {
                $($decl)* = $crate::InitStatic::__with_anchor($symbol, {
                    // rustc treats items allowing `dead_code` as live roots, so everything the
                    // record refers to counts as used, and no `dead_code` warning is reported for
                    // the initializer of a static nothing reads. This only affects the lint: the
                    // linker still drops unused statics.
                    #[allow(dead_code)]
                    #[inline(never)]
                    fn __anchor() {
                        static $entry: $crate::__private::Init = $init;
                        // Gives every anchor distinct contents, so that identical code folding
                        // never merges two of them.
                        ::core::hint::black_box(&$entry);
                        unsafe {
                            ::core::arch::asm!(
                                // "o" is SHF_LINK_ORDER; the last argument is the section it
                                // depends on.
                                ".pushsection init_static_gc_0_8,\"awo\",%progbits,{anchor}",
                                ".p2align 3",
                                ".8byte {entry}",
                                ".popsection",
                                anchor = sym __anchor,
                                entry = sym $entry,
                                options(nomem, nostack, preserves_flags),
                            );
                        }
                    }
                    __anchor
                });
            };
            (
                keep: true,
                static: { $($decl:tt)* },
                symbol: $symbol:expr,
                entry: $entry:ident = $init:expr $(,)?
            ) => {
                $($decl)* = $crate::InitStatic::new($symbol);

                const _: () = {
                    static $entry: $crate::__private::Init = $init;

                    #[used]
                    #[unsafe(link_section = "init_static_gc_0_8")]
                    static SLOT: ::core::option::Option<&$crate::__private::Init> =
                        ::core::option::Option::Some(&$entry);
                };
            };
        }

        pub use __declare as declare;

        /// Guarantees that the `init_static_gc_0_8` output section exists, and thus that its
        /// `__start_`/`__stop_` symbols are defined, even if the linker removes every other slot.
        #[used]
        #[unsafe(link_section = "init_static_gc_0_8")]
        static SENTINEL: Option<&Init> = None;

        /// Returns every registration record that made it into the final binary.
        pub(crate) fn registry() -> Vec<&'static Init> {
            unsafe extern "Rust" {
                #[link_name = "__start_init_static_gc_0_8"]
                static START: Option<&'static Init>;
                #[link_name = "__stop_init_static_gc_0_8"]
                static STOP: Option<&'static Init>;
            }

            // Referring to the sentinel makes sure the object file defining it is linked in.
            std::hint::black_box(&SENTINEL);
            // `START` and `STOP` are distinct objects as far as Rust is concerned, so neither may
            // be used to reach the slots between them. Work on plain addresses instead, and
            // recreate the pointer with exposed provenance. Rust does not formally specify access
            // to memory laid out by the linker; this is the closest it offers, and Miri does not
            // accept it.
            let start = (&raw const START).addr();
            let stop = (&raw const STOP).addr();
            let len = (stop - start) / size_of::<Option<&Init>>();
            let slots = std::ptr::with_exposed_provenance::<Option<&'static Init>>(start);
            // SAFETY: the linker places all slots, each an `Option<&Init>`, contiguously between
            // the two symbols, and nothing writes to them.
            let slots = unsafe { std::slice::from_raw_parts(slots, len) };
            slots.iter().flatten().copied().collect()
        }
    }
}
