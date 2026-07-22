use init_static::init_static;

#[tokio::test]
async fn main() {
    init_static! {
        static FOO: u32 = async { *BAR }.await;
        static BAR: u32 = async { *FOO }.await;
    }

    let e = init_static().await.unwrap_err();
    let expected = concat!(
        "Circular dependency detected among:\n",
        "    FOO (at init_static/tests/circular.rs:6:16)\n",
        "    BAR (at init_static/tests/circular.rs:7:16)\n",
    );
    assert_eq!(e.to_string(), expected);

    // The failure is cached and shared across calls.
    let e2 = init_static().await.unwrap_err();
    assert_eq!(e2.to_string(), expected);
}
