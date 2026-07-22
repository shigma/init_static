#[allow(unused_imports)]
use init_static_macro::init_static;
#[rustfmt::skip]
#[allow(clippy::type_complexity)]
static P1: ::init_static::InitStatic<u32> = ::init_static::InitStatic!(P1);
#[rustfmt::skip]
#[allow(clippy::type_complexity)]
static P2: ::init_static::InitStatic<u32> = ::init_static::InitStatic!(P2);
#[rustfmt::skip]
#[allow(clippy::type_complexity)]
static P3: ::init_static::InitStatic<u32> = ::init_static::InitStatic!(P3);
#[rustfmt::skip]
#[allow(clippy::type_complexity)]
static P4: ::init_static::InitStatic<u32> = ::init_static::InitStatic!(P4);
#[rustfmt::skip]
const _: () = {
    #[::init_static::__private::linkme::distributed_slice(
        ::init_static::__private::INIT
    )]
    #[linkme(crate = ::init_static::__private::linkme)]
    static INIT_P1: ::init_static::__private::Init = {
        #[allow(non_snake_case)]
        fn INIT_P1() -> ::init_static::__private::anyhow::Result<()> {
            ::init_static::InitStatic::init(&P1, 1);
            Ok(())
        }
        ::init_static::__private::Init {
            symbol: ::init_static::InitStatic::symbol(&P1),
            init: ::init_static::__private::InitFn::Sync(INIT_P1),
            deps: ::std::vec::Vec::new,
            priority: 10i32,
        }
    };
    #[::init_static::__private::linkme::distributed_slice(
        ::init_static::__private::INIT
    )]
    #[linkme(crate = ::init_static::__private::linkme)]
    static INIT_P2: ::init_static::__private::Init = {
        #[allow(non_snake_case)]
        fn INIT_P2() -> ::init_static::__private::anyhow::Result<()> {
            ::init_static::InitStatic::init(&P2, *P1 + 1);
            Ok(())
        }
        #[allow(non_snake_case, clippy::needless_borrow)]
        fn DEPS_P2() -> ::std::vec::Vec<
            ::std::option::Option<&'static ::init_static::Symbol>,
        > {
            use ::init_static::__private::MaybeInitStatic;
            ::std::vec![(& P1).__get_symbol()]
        }
        ::init_static::__private::Init {
            symbol: ::init_static::InitStatic::symbol(&P2),
            init: ::init_static::__private::InitFn::Sync(INIT_P2),
            deps: DEPS_P2,
            priority: 0i32,
        }
    };
    #[::init_static::__private::linkme::distributed_slice(
        ::init_static::__private::INIT
    )]
    #[linkme(crate = ::init_static::__private::linkme)]
    static INIT_P3: ::init_static::__private::Init = {
        #[allow(non_snake_case)]
        fn INIT_P3() -> ::init_static::__private::anyhow::Result<()> {
            ::init_static::InitStatic::init(&P3, "42".parse()?);
            Ok(())
        }
        ::init_static::__private::Init {
            symbol: ::init_static::InitStatic::symbol(&P3),
            init: ::init_static::__private::InitFn::Sync(INIT_P3),
            deps: ::std::vec::Vec::new,
            priority: 5i32,
        }
    };
    #[::init_static::__private::linkme::distributed_slice(
        ::init_static::__private::INIT
    )]
    #[linkme(crate = ::init_static::__private::linkme)]
    static INIT_P4: ::init_static::__private::Init = {
        #[allow(non_snake_case)]
        fn INIT_P4() -> ::init_static::__private::anyhow::Result<()> {
            ::init_static::InitStatic::init(&P4, 4);
            Ok(())
        }
        ::init_static::__private::Init {
            symbol: ::init_static::InitStatic::symbol(&P4),
            init: ::init_static::__private::InitFn::Sync(INIT_P4),
            deps: ::std::vec::Vec::new,
            priority: -5i32,
        }
    };
};
