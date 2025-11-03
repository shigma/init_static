#[allow(unused_imports)]
use init_static_macro::init_static;
#[rustfmt::skip]
static V1: ::init_static::InitStatic<u32> = ::init_static::InitStatic::new();
#[rustfmt::skip]
#[::init_static::__private::linkme::distributed_slice(
    ::init_static::__private::INIT_FUNCTIONS
)]
#[linkme(crate = ::init_static::__private::linkme)]
fn __init_v1() -> std::pin::Pin<
    Box<dyn Future<Output = Result<(), Box<dyn ::std::error::Error>>>>,
> {
    Box::pin(async {
        ::init_static::InitStatic::init(&V1, 42);
        Ok(())
    })
}
#[rustfmt::skip]
static V2: ::init_static::InitStatic<u32> = ::init_static::InitStatic::new();
#[rustfmt::skip]
#[::init_static::__private::linkme::distributed_slice(
    ::init_static::__private::INIT_FUNCTIONS
)]
#[linkme(crate = ::init_static::__private::linkme)]
fn __init_v2() -> std::pin::Pin<
    Box<dyn Future<Output = Result<(), Box<dyn ::std::error::Error>>>>,
> {
    Box::pin(async {
        ::init_static::InitStatic::init(&V2, "42".parse()?);
        Ok(())
    })
}
#[rustfmt::skip]
static V3: ::init_static::InitStatic<u32> = ::init_static::InitStatic::new();
#[rustfmt::skip]
#[::init_static::__private::linkme::distributed_slice(
    ::init_static::__private::INIT_FUNCTIONS
)]
#[linkme(crate = ::init_static::__private::linkme)]
fn __init_v3() -> std::pin::Pin<
    Box<dyn Future<Output = Result<(), Box<dyn ::std::error::Error>>>>,
> {
    Box::pin(async {
        ::init_static::InitStatic::init(&V3, async { 42 }.await);
        Ok(())
    })
}
