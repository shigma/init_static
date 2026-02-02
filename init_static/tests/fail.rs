use init_static::init_static;

init_static! {
    static FOO: u32 = "malformed".parse()?;
}

#[tokio::test]
async fn main() {
    let e = init_static().await.unwrap_err();
    assert_eq!(
        e.to_string(),
        "failed to initialize FOO (at init_static/tests/fail.rs:4:12)"
    );
}
