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
        pub static SUBSYSTEM: u32 = super::record("SUBSYSTEM");
    }
}

init_static! {
    static LOGGER: u32 = record("LOGGER");

    // A pure ordering constraint: the expression never mentions LOGGER, so there is no
    // inferred dependency to carry the edge.
    #[after(LOGGER)]
    static SERVICE: u32 = async { record("SERVICE") }.await;

    // Cross-module target, and a target that is *also* an inferred dependency: the
    // duplicate edge must collapse rather than look like a cycle.
    #[after(nested::SUBSYSTEM, SERVICE)]
    static CLIENT: u32 = *SERVICE + record("CLIENT");

    // Both directions on one static.
    #[after(CLIENT)]
    #[before(TAIL)]
    static MIDDLE: u32 = record("MIDDLE");
    static TAIL: u32 = record("TAIL");
}

#[tokio::test]
async fn main() {
    init_static().await.unwrap();

    let order = ORDER.lock().unwrap().clone();
    let index = |name| order.iter().position(|&n| n == name).unwrap();
    assert_eq!(order.len(), 6);
    assert!(index("LOGGER") < index("SERVICE"));
    assert!(index("SERVICE") < index("CLIENT"));
    assert!(index("SUBSYSTEM") < index("CLIENT"));
    assert!(index("CLIENT") < index("MIDDLE"));
    assert!(index("MIDDLE") < index("TAIL"));
}
