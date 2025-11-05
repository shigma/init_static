use init_static::init_static;

init_static! {
    static BAR: u32 = *FOO;
}

init_static! {
    static FOO: u32 = "42".parse()?;
}

#[tokio::test]
async fn main() {
    init_static().await.unwrap();
}
