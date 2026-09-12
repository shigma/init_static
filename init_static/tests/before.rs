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
        pub static TAIL: u32 = super::record("TAIL");
    }
}

init_static! {
    // Reverse dependency: HEAD is initialized before MIDDLE, even though MIDDLE
    // never mentions HEAD in its expression.
    #[before(MIDDLE)]
    static HEAD: u32 = record("HEAD");
    static MIDDLE: u32 = record("MIDDLE");
    // Multiple targets in one attribute, including a path into another module and
    // an async initializer.
    #[before(HEAD, nested::TAIL)]
    static ROOT: u32 = async { record("ROOT") }.await;
}

#[tokio::test]
async fn main() {
    init_static().await.unwrap();

    let order = ORDER.lock().unwrap().clone();
    let index = |name| order.iter().position(|&n| n == name).unwrap();
    assert_eq!(order.len(), 4);
    assert!(index("ROOT") < index("HEAD"));
    assert!(index("ROOT") < index("TAIL"));
    assert!(index("HEAD") < index("MIDDLE"));
}
