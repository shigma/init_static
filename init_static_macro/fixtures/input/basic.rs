#[allow(unused_imports)]
use init_static_macro::init_static;
#[rustfmt::skip]
init_static! {
    static FOO: u32 = 42;
    static BAR: u32 = "42".parse()?;
}
