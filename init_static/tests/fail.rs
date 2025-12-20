use init_static::init_static;

init_static! {
    static FOO: u32 = "malformed".parse()?;
}

#[tokio::test]
async fn main() {
    let e = init_static().await.unwrap_err();
    assert_eq!(e.to_string(), "invalid digit found in string");
}
