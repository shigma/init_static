use init_static::init_static;

init_static! {
    static FOO: u32 = *BAR;
}

init_static! {
    static BAR: u32 = "42".parse()?;
}

#[tokio::test]
async fn main() {
    init_static().await.unwrap();
    // Calling again shares the same completed future.
    init_static().await.unwrap();
    assert_eq!(*FOO, 42);
}
