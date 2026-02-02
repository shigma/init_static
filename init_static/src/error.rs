use crate::Symbol;

/// Error type returned by [`init_static()`](crate::init_static()) when initialization fails.
///
/// This enum represents the various failure modes that can occur during the static initialization
/// process.
///
/// ## Note on Execution Errors
///
/// Errors returned by initialization expressions (e.g., `"42".parse()?`) are **NOT**
/// wrapped in this enum. Instead, [`init_static()`](crate::init_static()) returns
/// [`anyhow::Result<()>`] directly, which preserves the original error's backtrace for better
/// debugging.
///
/// To distinguish between error types, use [`anyhow::Error::downcast`] or
/// [`anyhow::Error::downcast_ref`].
#[derive(Debug)]
pub enum InitError {
    /// A static symbol was defined multiple times.
    ///
    /// This typically occurs when the same [`init_static!`](crate::init_static!) block is included
    /// multiple times, or when two statics in different modules have the exact same source
    /// location metadata (which should not happen in normal usage).
    Ambiguous { symbol: &'static Symbol },

    /// A circular dependency was detected among statics.
    ///
    /// This occurs when static A depends on static B, and static B (directly or indirectly) depends
    /// on static A. The initialization system cannot determine a valid order to initialize such
    /// statics.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use init_static::init_static;
    ///
    /// init_static! {
    ///     // This creates a circular dependency and will fail at runtime
    ///     static A: u32 = *B + 1;
    ///     static B: u32 = *A + 1;
    /// }
    /// ```
    Circular { symbols: Vec<&'static Symbol> },
}

impl std::fmt::Display for InitError {
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
        }
    }
}

impl std::error::Error for InitError {}
