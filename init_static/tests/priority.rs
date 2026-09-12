use std::sync::Mutex;

use init_static::init_static;

static ORDER: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

fn record(name: &'static str) -> u32 {
    ORDER.lock().unwrap().push(name);
    0
}

init_static! {
    // Highest priority; depends on DEP, which has default priority 0.
    #[priority(10)]
    static HIGH: u32 = async { *DEP + record("HIGH") }.await;
    // Default priority, but pulled into the top tier via effective-priority
    // propagation because HIGH depends on it.
    static DEP: u32 = record("DEP");
    #[priority(5)]
    static MID: u32 = record("MID");
    static LOW: u32 = record("LOW");
    // Negative priority runs after all default (0) nodes.
    #[priority(-5)]
    static LATE: u32 = record("LATE");
}

#[tokio::test]
async fn main() {
    init_static().await.unwrap();

    // Tiers run in descending effective priority: 10 (DEP then HIGH, ordered by
    // dependency), then 5 (MID), then 0 (LOW), then -5 (LATE).
    let order = ORDER.lock().unwrap().clone();
    assert_eq!(order, vec!["DEP", "HIGH", "MID", "LOW", "LATE"]);
}
