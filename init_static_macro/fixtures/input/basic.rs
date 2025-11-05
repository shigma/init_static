#[allow(unused_imports)]
use init_static_macro::init_static;
#[rustfmt::skip]
init_static! {
    static V1: u32 = 42;
    static V2: u32 = "42".parse()?;
    static V3: u32 = async { 42 }.await;
    static V4: u32 = async { "42".parse() }.await?;
}
#[rustfmt::skip]
init_static! {
    static V5: u32 = {
        const N: u32 = 42;
        *V1 + N
    };
}
