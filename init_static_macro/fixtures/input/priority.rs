#[allow(unused_imports)]
use init_static_macro::init_static;
#[rustfmt::skip]
init_static! {
    #[priority = 10]
    static P1: u32 = 1;
    static P2: u32 = *P1 + 1;
    #[priority = 5]
    static P3: u32 = "42".parse()?;
    #[priority = -5]
    static P4: u32 = 4;
}
