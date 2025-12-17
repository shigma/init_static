#[allow(unused_imports)]
use init_static_macro::init_static;
#[rustfmt::skip]
static V1: ::init_static::InitStatic<u32> = ::init_static::InitStatic::new();
#[rustfmt::skip]
static V2: ::init_static::InitStatic<u32> = ::init_static::InitStatic::new();
#[rustfmt::skip]
static V3: ::init_static::InitStatic<u32> = ::init_static::InitStatic::new();
#[rustfmt::skip]
static V4: ::init_static::InitStatic<u32> = ::init_static::InitStatic::new();
#[rustfmt::skip]
#[::init_static::__private::linkme::distributed_slice(::init_static::__private::INIT)]
#[linkme(crate = ::init_static::__private::linkme)]
static INIT_STATIC_V1_V2_V3_V4: ::init_static::__private::Init = {
    #[allow(non_snake_case)]
    fn INIT_STATIC_V1_V2_V3_V4() -> std::pin::Pin<
        Box<dyn Future<Output = Result<(), ::init_static::__private::anyhow::Error>>>,
    > {
        Box::pin(async {
            ::init_static::InitStatic::init(&V1, N1);
            ::init_static::InitStatic::init(&V2, "42".parse()?);
            ::init_static::InitStatic::init(&V3, async { N1 }.await);
            ::init_static::InitStatic::init(&V4, async { "42".parse() }.await?);
            Ok(())
        })
    }
    ::init_static::__private::Init {
        init: INIT_STATIC_V1_V2_V3_V4,
        names: &["V1", "V2", "V3", "V4"],
        deps: &["N1"],
    }
};
#[rustfmt::skip]
const N1: u32 = 42;
#[rustfmt::skip]
static V5: ::init_static::InitStatic<u32> = ::init_static::InitStatic::new();
#[rustfmt::skip]
#[::init_static::__private::linkme::distributed_slice(::init_static::__private::INIT)]
#[linkme(crate = ::init_static::__private::linkme)]
static INIT_STATIC_V5: ::init_static::__private::Init = {
    #[allow(non_snake_case)]
    fn INIT_STATIC_V5() -> std::pin::Pin<
        Box<dyn Future<Output = Result<(), ::init_static::__private::anyhow::Error>>>,
    > {
        Box::pin(async {
            let x = 42;
            ::init_static::InitStatic::init(
                &V5,
                {
                    const N2: u32 = 42;
                    *V1 + N1 + N2 + x
                },
            );
            Ok(())
        })
    }
    ::init_static::__private::Init {
        init: INIT_STATIC_V5,
        names: &["V5"],
        deps: &["V1"],
    }
};
