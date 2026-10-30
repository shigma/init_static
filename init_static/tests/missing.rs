//! A dependency declared by `init_static!` whose registration record is missing must fail before
//! anything is initialized. Such a record can only go missing in GC mode, so it is simulated here
//! by a static that carries an anchor without having a record behind it.
#![cfg(init_static_gc_enabled)]

use std::sync::atomic::{AtomicBool, Ordering};

use init_static::{InitError, InitStatic, Symbol, init_static};

static DROPPED: InitStatic<u32> = InitStatic::__with_anchor(Symbol!(DROPPED), || {});

static RAN: AtomicBool = AtomicBool::new(false);

init_static! {
    #[keep]
    static OTHER: () = RAN.store(true, Ordering::SeqCst);
    #[keep]
    static DEPENDENT: u32 = *DROPPED;
}

#[tokio::test]
async fn main() {
    let e = init_static().await.unwrap_err();
    match e.downcast_ref::<InitError>() {
        Some(InitError::Missing { symbol, required_by }) => {
            assert_eq!(symbol.ident, "DROPPED");
            assert_eq!(required_by.ident, "DEPENDENT");
        }
        _ => panic!("unexpected error: {e}"),
    }
    assert!(!RAN.load(Ordering::SeqCst));
}
