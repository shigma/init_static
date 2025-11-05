#[allow(unused_imports)]
use init_static_macro::init_static;
#[rustfmt::skip]
static V1: ::init_static::InitStatic<u32> = ::init_static::InitStatic::new(&[]);
#[rustfmt::skip]
#[allow(non_snake_case)]
#[::init_static::__private::linkme::distributed_slice(
    ::init_static::__private::INIT
)]
#[linkme(crate = ::init_static::__private::linkme)]
fn init_static_V1() -> std::pin::Pin<
    Box<dyn Future<Output = Result<(), Box<dyn ::std::error::Error>>>>,
> {
    Box::pin(async {
        ::init_static::InitStatic::init(&V1, 42);
        Ok(())
    })
}
#[rustfmt::skip]
static V2: ::init_static::InitStatic<u32> = ::init_static::InitStatic::new(&[]);
#[rustfmt::skip]
#[allow(non_snake_case)]
#[::init_static::__private::linkme::distributed_slice(
    ::init_static::__private::INIT
)]
#[linkme(crate = ::init_static::__private::linkme)]
fn init_static_V2() -> std::pin::Pin<
    Box<dyn Future<Output = Result<(), Box<dyn ::std::error::Error>>>>,
> {
    Box::pin(async {
        ::init_static::InitStatic::init(&V2, "42".parse()?);
        Ok(())
    })
}
#[rustfmt::skip]
static V3: ::init_static::InitStatic<u32> = ::init_static::InitStatic::new(&[]);
#[rustfmt::skip]
#[allow(non_snake_case)]
#[::init_static::__private::linkme::distributed_slice(
    ::init_static::__private::INIT
)]
#[linkme(crate = ::init_static::__private::linkme)]
fn init_static_V3() -> std::pin::Pin<
    Box<dyn Future<Output = Result<(), Box<dyn ::std::error::Error>>>>,
> {
    Box::pin(async {
        ::init_static::InitStatic::init(&V3, async { 42 }.await);
        Ok(())
    })
}
#[rustfmt::skip]
static V4: ::init_static::InitStatic<u32> = ::init_static::InitStatic::new(&[]);
#[rustfmt::skip]
#[allow(non_snake_case)]
#[::init_static::__private::linkme::distributed_slice(
    ::init_static::__private::INIT
)]
#[linkme(crate = ::init_static::__private::linkme)]
fn init_static_V4() -> std::pin::Pin<
    Box<dyn Future<Output = Result<(), Box<dyn ::std::error::Error>>>>,
> {
    Box::pin(async {
        ::init_static::InitStatic::init(&V4, async { "42".parse() }.await?);
        Ok(())
    })
}
#[rustfmt::skip]
static V5: ::init_static::InitStatic<u32> = ::init_static::InitStatic::new(&["V1"]);
#[rustfmt::skip]
#[allow(non_snake_case)]
#[::init_static::__private::linkme::distributed_slice(
    ::init_static::__private::INIT
)]
#[linkme(crate = ::init_static::__private::linkme)]
fn init_static_V5() -> std::pin::Pin<
    Box<dyn Future<Output = Result<(), Box<dyn ::std::error::Error>>>>,
> {
    Box::pin(async {
        ::init_static::InitStatic::init(
            &V5,
            {
                const N: u32 = 42;
                *V1 + N
            },
        );
        Ok(())
    })
}
