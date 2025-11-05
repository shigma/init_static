#[allow(unused_imports)]
use init_static_macro::init_static;
#[rustfmt::skip]
static V1: ::init_static::InitStatic<u32> = ::init_static::InitStatic::new();
#[rustfmt::skip]
#[allow(non_snake_case)]
#[::init_static::__private::linkme::distributed_slice(::init_static::__private::INIT)]
#[linkme(crate = ::init_static::__private::linkme)]
static INIT_STATIC_V1: ::init_static::__private::Init = {
    fn INIT_STATIC_V1() -> std::pin::Pin<
        Box<dyn Future<Output = Result<(), Box<dyn ::std::error::Error>>>>,
    > {
        Box::pin(async {
            ::init_static::InitStatic::init(&V1, 42);
            Ok(())
        })
    }
    ::init_static::__private::Init {
        name: "V1",
        init: INIT_STATIC_V1,
        deps: &[],
    }
};
#[rustfmt::skip]
static V2: ::init_static::InitStatic<u32> = ::init_static::InitStatic::new();
#[rustfmt::skip]
#[allow(non_snake_case)]
#[::init_static::__private::linkme::distributed_slice(::init_static::__private::INIT)]
#[linkme(crate = ::init_static::__private::linkme)]
static INIT_STATIC_V2: ::init_static::__private::Init = {
    fn INIT_STATIC_V2() -> std::pin::Pin<
        Box<dyn Future<Output = Result<(), Box<dyn ::std::error::Error>>>>,
    > {
        Box::pin(async {
            ::init_static::InitStatic::init(&V2, "42".parse()?);
            Ok(())
        })
    }
    ::init_static::__private::Init {
        name: "V2",
        init: INIT_STATIC_V2,
        deps: &[],
    }
};
#[rustfmt::skip]
static V3: ::init_static::InitStatic<u32> = ::init_static::InitStatic::new();
#[rustfmt::skip]
#[allow(non_snake_case)]
#[::init_static::__private::linkme::distributed_slice(::init_static::__private::INIT)]
#[linkme(crate = ::init_static::__private::linkme)]
static INIT_STATIC_V3: ::init_static::__private::Init = {
    fn INIT_STATIC_V3() -> std::pin::Pin<
        Box<dyn Future<Output = Result<(), Box<dyn ::std::error::Error>>>>,
    > {
        Box::pin(async {
            ::init_static::InitStatic::init(&V3, async { 42 }.await);
            Ok(())
        })
    }
    ::init_static::__private::Init {
        name: "V3",
        init: INIT_STATIC_V3,
        deps: &[],
    }
};
#[rustfmt::skip]
static V4: ::init_static::InitStatic<u32> = ::init_static::InitStatic::new();
#[rustfmt::skip]
#[allow(non_snake_case)]
#[::init_static::__private::linkme::distributed_slice(::init_static::__private::INIT)]
#[linkme(crate = ::init_static::__private::linkme)]
static INIT_STATIC_V4: ::init_static::__private::Init = {
    fn INIT_STATIC_V4() -> std::pin::Pin<
        Box<dyn Future<Output = Result<(), Box<dyn ::std::error::Error>>>>,
    > {
        Box::pin(async {
            ::init_static::InitStatic::init(&V4, async { "42".parse() }.await?);
            Ok(())
        })
    }
    ::init_static::__private::Init {
        name: "V4",
        init: INIT_STATIC_V4,
        deps: &[],
    }
};
#[rustfmt::skip]
static V5: ::init_static::InitStatic<u32> = ::init_static::InitStatic::new();
#[rustfmt::skip]
#[allow(non_snake_case)]
#[::init_static::__private::linkme::distributed_slice(::init_static::__private::INIT)]
#[linkme(crate = ::init_static::__private::linkme)]
static INIT_STATIC_V5: ::init_static::__private::Init = {
    fn INIT_STATIC_V5() -> std::pin::Pin<
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
    ::init_static::__private::Init {
        name: "V5",
        init: INIT_STATIC_V5,
        deps: &["V1"],
    }
};
