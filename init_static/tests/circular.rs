use init_static::init_static;

#[tokio::test]
async fn main() {
    init_static! {
        static FOO: u32 = async { *BAR }.await;
        static BAR: u32 = async { *FOO }.await;
    }

    let e = init_static().await.unwrap_err();
    assert_eq!(
        e.to_string(),
        [
            "Circular dependency detected among:\n",
            "    BAR (at init_static/tests/circular.rs:7:16)\n",
            "    FOO (at init_static/tests/circular.rs:6:16)\n"
        ]
        .join("")
    );
}
