//! Mirrors the scenarios of the link-time GC design. With the `init_static_gc` cfg in effect, only
//! statics the binary actually uses are initialized (this requires lld); otherwise every static
//! is.

use std::sync::Mutex;

use init_static::init_static;

static ORDER: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

fn record(name: &'static str) -> u32 {
    ORDER.lock().unwrap().push(name);
    0
}

mod nested {
    use init_static::init_static;

    init_static! {
        pub static AFTER: u32 = super::record("AFTER");
    }
}

init_static! {
    // Read by the test below. `#[after]` only constrains the order, so it does not keep AFTER.
    #[after(nested::AFTER)]
    static USED: u32 = *TRANSITIVE + record("USED");
    // Only read by USED's initializer.
    static TRANSITIVE: u32 = record("TRANSITIVE");
    // Only read by a function nobody calls.
    static UNUSED: u32 = *UNUSED_TRANSITIVE + record("UNUSED");
    static UNUSED_TRANSITIVE: u32 = record("UNUSED_TRANSITIVE");
    // Never read, but kept for its side effect; its own `#[before]` target is unused.
    #[keep]
    #[before(UNUSED)]
    static SIDE_EFFECT: u32 = *KEPT_TRANSITIVE + record("SIDE_EFFECT");
    static KEPT_TRANSITIVE: u32 = record("KEPT_TRANSITIVE");
}

#[allow(dead_code)]
fn read_unused() -> u32 {
    *UNUSED
}

#[tokio::test]
async fn main() {
    init_static().await.unwrap();
    assert_eq!(*USED, 0);

    let mut order = ORDER.lock().unwrap().clone();
    order.sort_unstable();
    let mut expected = vec!["KEPT_TRANSITIVE", "SIDE_EFFECT", "TRANSITIVE", "USED"];
    if !cfg!(init_static_gc_enabled) {
        expected.extend(["AFTER", "UNUSED", "UNUSED_TRANSITIVE"]);
        expected.sort_unstable();
    }
    assert_eq!(order, expected);
}
