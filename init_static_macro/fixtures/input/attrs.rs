#[allow(unused_imports)]
use init_static_macro::init_static;
#[rustfmt::skip]
init_static! {
    #[priority(10)]
    static P1: u32 = 1;
    static P2: u32 = *P1 + 1;
    #[priority(5)]
    static P3: u32 = "42".parse()?;
    #[priority(-5)]
    static P4: u32 = 4;

    #[before(B1)]
    static B0: u32 = 0;
    static B1: u32 = 1;
    // Several paths in one attribute, and the attribute repeated on one static.
    #[before(B0, B1)]
    #[before(P4)]
    #[priority(5)]
    static B2: u32 = 2;

    #[after(B0)]
    static A0: u32 = 0;
    // Both directions at once, plus a target that is also an inferred dependency.
    #[after(A0, B1)]
    #[before(A2)]
    static A1: u32 = *B1 + 1;
    static A2: u32 = 2;
}
