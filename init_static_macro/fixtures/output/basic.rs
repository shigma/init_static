#[allow(unused_imports)]
use init_static_macro::init_static;
#[rustfmt::skip]
static FOO: ::init_static::InitStatic<u32> = ::init_static::InitStatic::new();
#[rustfmt::skip]
#[::init_static::__private::linkme::distributed_slice(
    ::init_static::__private::INIT_FUNCTIONS
)]
#[linkme(crate = ::init_static::__private::linkme)]
fn __init_foo() -> Result<
    (),
    Box<dyn ::std::error::Error + ::std::marker::Send + ::std::marker::Sync>,
> {
    ::init_static::InitStatic::init(&FOO, 42);
    Ok(())
}
#[rustfmt::skip]
static BAR: ::init_static::InitStatic<u32> = ::init_static::InitStatic::new();
#[rustfmt::skip]
#[::init_static::__private::linkme::distributed_slice(
    ::init_static::__private::INIT_FUNCTIONS
)]
#[linkme(crate = ::init_static::__private::linkme)]
fn __init_bar() -> Result<
    (),
    Box<dyn ::std::error::Error + ::std::marker::Send + ::std::marker::Sync>,
> {
    ::init_static::InitStatic::init(&BAR, "42".parse()?);
    Ok(())
}
