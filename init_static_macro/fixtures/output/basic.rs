#[allow(unused_imports)]
use init_static_macro::init_static;
#[rustfmt::skip]
static V1: ::std::sync::LazyLock<u32> = ::std::sync::LazyLock::new(|| N1);
#[rustfmt::skip]
static V2: ::init_static::InitStatic<u32> = ::init_static::InitStatic!(V2);
#[rustfmt::skip]
static V3: ::init_static::InitStatic<u32> = ::init_static::InitStatic!(V3);
#[rustfmt::skip]
static V4: ::init_static::InitStatic<u32> = ::init_static::InitStatic!(V4);
#[rustfmt::skip]
const _: () = {
    #[::init_static::__private::linkme::distributed_slice(
        ::init_static::__private::INIT
    )]
    #[linkme(crate = ::init_static::__private::linkme)]
    static INIT_V1: ::init_static::__private::Init = {
        #[allow(non_snake_case)]
        fn INIT_V1() -> ::init_static::__private::anyhow::Result<()> {
            ::std::sync::LazyLock::force(&V1);
            Ok(())
        }
        ::init_static::__private::Init {
            symbol: ::init_static::Symbol!(V1),
            init: ::init_static::__private::InitFn::Sync(INIT_V1),
            deps: ::std::vec::Vec::new,
        }
    };
    #[::init_static::__private::linkme::distributed_slice(
        ::init_static::__private::INIT
    )]
    #[linkme(crate = ::init_static::__private::linkme)]
    static INIT_V2: ::init_static::__private::Init = {
        #[allow(non_snake_case)]
        fn INIT_V2() -> ::init_static::__private::anyhow::Result<()> {
            ::init_static::InitStatic::init(&V2, "42".parse()?);
            Ok(())
        }
        ::init_static::__private::Init {
            symbol: ::init_static::InitStatic::symbol(&V2),
            init: ::init_static::__private::InitFn::Sync(INIT_V2),
            deps: ::std::vec::Vec::new,
        }
    };
    #[::init_static::__private::linkme::distributed_slice(
        ::init_static::__private::INIT
    )]
    #[linkme(crate = ::init_static::__private::linkme)]
    static INIT_V3: ::init_static::__private::Init = {
        #[allow(non_snake_case)]
        fn INIT_V3() -> ::init_static::__private::BoxFuture<
            ::init_static::__private::anyhow::Result<()>,
        > {
            Box::pin(async {
                ::init_static::InitStatic::init(&V3, async { N1 }.await);
                Ok(())
            })
        }
        #[allow(non_snake_case, clippy::needless_borrow)]
        fn DEPS_V3() -> ::std::vec::Vec<
            ::std::option::Option<&'static ::init_static::Symbol>,
        > {
            use ::init_static::__private::MaybeInitStatic;
            ::std::vec![(& N1).__get_symbol()]
        }
        ::init_static::__private::Init {
            symbol: ::init_static::InitStatic::symbol(&V3),
            init: ::init_static::__private::InitFn::Async(INIT_V3),
            deps: DEPS_V3,
        }
    };
    #[::init_static::__private::linkme::distributed_slice(
        ::init_static::__private::INIT
    )]
    #[linkme(crate = ::init_static::__private::linkme)]
    static INIT_V4: ::init_static::__private::Init = {
        #[allow(non_snake_case)]
        fn INIT_V4() -> ::init_static::__private::BoxFuture<
            ::init_static::__private::anyhow::Result<()>,
        > {
            Box::pin(async {
                ::init_static::InitStatic::init(&V4, async { "42".parse() }.await?);
                Ok(())
            })
        }
        ::init_static::__private::Init {
            symbol: ::init_static::InitStatic::symbol(&V4),
            init: ::init_static::__private::InitFn::Async(INIT_V4),
            deps: ::std::vec::Vec::new,
        }
    };
};
#[rustfmt::skip]
const N1: u32 = 42;
#[rustfmt::skip]
static V5: ::init_static::InitStatic<u32> = ::init_static::InitStatic!(V5);
#[rustfmt::skip]
const _: () = {
    #[::init_static::__private::linkme::distributed_slice(
        ::init_static::__private::INIT
    )]
    #[linkme(crate = ::init_static::__private::linkme)]
    static INIT_V5: ::init_static::__private::Init = {
        #[allow(non_snake_case)]
        fn INIT_V5() -> ::init_static::__private::BoxFuture<
            ::init_static::__private::anyhow::Result<()>,
        > {
            Box::pin(async {
                ::init_static::InitStatic::init(
                    &V5,
                    async {
                        #[expect(non_snake_case)]
                        let X = 42;
                        const N2: u32 = 42;
                        *V1 + N1 + N2 + X
                    }
                        .await,
                );
                Ok(())
            })
        }
        #[allow(non_snake_case, clippy::needless_borrow)]
        fn DEPS_V5() -> ::std::vec::Vec<
            ::std::option::Option<&'static ::init_static::Symbol>,
        > {
            use ::init_static::__private::MaybeInitStatic;
            ::std::vec![(& N1).__get_symbol(), (& V1).__get_symbol()]
        }
        ::init_static::__private::Init {
            symbol: ::init_static::InitStatic::symbol(&V5),
            init: ::init_static::__private::InitFn::Async(INIT_V5),
            deps: DEPS_V5,
        }
    };
};
