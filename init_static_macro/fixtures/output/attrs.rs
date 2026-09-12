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
#[allow(clippy::type_complexity)]
static B0: ::init_static::InitStatic<u32> = ::init_static::InitStatic!(B0);
#[rustfmt::skip]
#[allow(clippy::type_complexity)]
static B1: ::init_static::InitStatic<u32> = ::init_static::InitStatic!(B1);
#[rustfmt::skip]
#[allow(clippy::type_complexity)]
static B2: ::init_static::InitStatic<u32> = ::init_static::InitStatic!(B2);
#[rustfmt::skip]
#[allow(clippy::type_complexity)]
static A0: ::init_static::InitStatic<u32> = ::init_static::InitStatic!(A0);
#[rustfmt::skip]
#[allow(clippy::type_complexity)]
static A1: ::init_static::InitStatic<u32> = ::init_static::InitStatic!(A1);
#[rustfmt::skip]
#[allow(clippy::type_complexity)]
static A2: ::init_static::InitStatic<u32> = ::init_static::InitStatic!(A2);
#[rustfmt::skip]
const _: () = {
    #[::init_static::__private::linkme::distributed_slice(
        ::init_static::__private::INIT
    )]
    #[linkme(crate = ::init_static::__private::linkme)]
    static INIT_P1: ::init_static::__private::Init = {
        #[allow(unused_imports)]
        use ::init_static::priority as _;
        #[allow(non_snake_case)]
        fn INIT_P1() -> ::init_static::__private::anyhow::Result<()> {
            ::init_static::InitStatic::init(&P1, 1);
            Ok(())
        }
        ::init_static::__private::Init {
            symbol: ::init_static::InitStatic::symbol(&P1),
            init: ::init_static::__private::InitFn::Sync(INIT_P1),
            deps: ::std::vec::Vec::new,
            before: &[],
            after: &[],
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
            before: &[],
            after: &[],
            priority: 0i32,
        }
    };
    #[::init_static::__private::linkme::distributed_slice(
        ::init_static::__private::INIT
    )]
    #[linkme(crate = ::init_static::__private::linkme)]
    static INIT_P3: ::init_static::__private::Init = {
        #[allow(unused_imports)]
        use ::init_static::priority as _;
        #[allow(non_snake_case)]
        fn INIT_P3() -> ::init_static::__private::anyhow::Result<()> {
            ::init_static::InitStatic::init(&P3, "42".parse()?);
            Ok(())
        }
        ::init_static::__private::Init {
            symbol: ::init_static::InitStatic::symbol(&P3),
            init: ::init_static::__private::InitFn::Sync(INIT_P3),
            deps: ::std::vec::Vec::new,
            before: &[],
            after: &[],
            priority: 5i32,
        }
    };
    #[::init_static::__private::linkme::distributed_slice(
        ::init_static::__private::INIT
    )]
    #[linkme(crate = ::init_static::__private::linkme)]
    static INIT_P4: ::init_static::__private::Init = {
        #[allow(unused_imports)]
        use ::init_static::priority as _;
        #[allow(non_snake_case)]
        fn INIT_P4() -> ::init_static::__private::anyhow::Result<()> {
            ::init_static::InitStatic::init(&P4, 4);
            Ok(())
        }
        ::init_static::__private::Init {
            symbol: ::init_static::InitStatic::symbol(&P4),
            init: ::init_static::__private::InitFn::Sync(INIT_P4),
            deps: ::std::vec::Vec::new,
            before: &[],
            after: &[],
            priority: -5i32,
        }
    };
    #[::init_static::__private::linkme::distributed_slice(
        ::init_static::__private::INIT
    )]
    #[linkme(crate = ::init_static::__private::linkme)]
    static INIT_B0: ::init_static::__private::Init = {
        #[allow(unused_imports)]
        use ::init_static::before as _;
        #[allow(non_snake_case)]
        fn INIT_B0() -> ::init_static::__private::anyhow::Result<()> {
            ::init_static::InitStatic::init(&B0, 0);
            Ok(())
        }
        ::init_static::__private::Init {
            symbol: ::init_static::InitStatic::symbol(&B0),
            init: ::init_static::__private::InitFn::Sync(INIT_B0),
            deps: ::std::vec::Vec::new,
            before: &[::init_static::InitStatic::symbol(&B1)],
            after: &[],
            priority: 0i32,
        }
    };
    #[::init_static::__private::linkme::distributed_slice(
        ::init_static::__private::INIT
    )]
    #[linkme(crate = ::init_static::__private::linkme)]
    static INIT_B1: ::init_static::__private::Init = {
        #[allow(non_snake_case)]
        fn INIT_B1() -> ::init_static::__private::anyhow::Result<()> {
            ::init_static::InitStatic::init(&B1, 1);
            Ok(())
        }
        ::init_static::__private::Init {
            symbol: ::init_static::InitStatic::symbol(&B1),
            init: ::init_static::__private::InitFn::Sync(INIT_B1),
            deps: ::std::vec::Vec::new,
            before: &[],
            after: &[],
            priority: 0i32,
        }
    };
    #[::init_static::__private::linkme::distributed_slice(
        ::init_static::__private::INIT
    )]
    #[linkme(crate = ::init_static::__private::linkme)]
    static INIT_B2: ::init_static::__private::Init = {
        #[allow(unused_imports)]
        use ::init_static::priority as _;
        #[allow(unused_imports)]
        use ::init_static::before as _;
        #[allow(unused_imports)]
        use ::init_static::before as _;
        #[allow(non_snake_case)]
        fn INIT_B2() -> ::init_static::__private::anyhow::Result<()> {
            ::init_static::InitStatic::init(&B2, 2);
            Ok(())
        }
        ::init_static::__private::Init {
            symbol: ::init_static::InitStatic::symbol(&B2),
            init: ::init_static::__private::InitFn::Sync(INIT_B2),
            deps: ::std::vec::Vec::new,
            before: &[
                ::init_static::InitStatic::symbol(&B0),
                ::init_static::InitStatic::symbol(&B1),
                ::init_static::InitStatic::symbol(&P4),
            ],
            after: &[],
            priority: 5i32,
        }
    };
    #[::init_static::__private::linkme::distributed_slice(
        ::init_static::__private::INIT
    )]
    #[linkme(crate = ::init_static::__private::linkme)]
    static INIT_A0: ::init_static::__private::Init = {
        #[allow(unused_imports)]
        use ::init_static::after as _;
        #[allow(non_snake_case)]
        fn INIT_A0() -> ::init_static::__private::anyhow::Result<()> {
            ::init_static::InitStatic::init(&A0, 0);
            Ok(())
        }
        ::init_static::__private::Init {
            symbol: ::init_static::InitStatic::symbol(&A0),
            init: ::init_static::__private::InitFn::Sync(INIT_A0),
            deps: ::std::vec::Vec::new,
            before: &[],
            after: &[::init_static::InitStatic::symbol(&B0)],
            priority: 0i32,
        }
    };
    #[::init_static::__private::linkme::distributed_slice(
        ::init_static::__private::INIT
    )]
    #[linkme(crate = ::init_static::__private::linkme)]
    static INIT_A1: ::init_static::__private::Init = {
        #[allow(unused_imports)]
        use ::init_static::before as _;
        #[allow(unused_imports)]
        use ::init_static::after as _;
        #[allow(non_snake_case)]
        fn INIT_A1() -> ::init_static::__private::anyhow::Result<()> {
            ::init_static::InitStatic::init(&A1, *B1 + 1);
            Ok(())
        }
        #[allow(non_snake_case, clippy::needless_borrow)]
        fn DEPS_A1() -> ::std::vec::Vec<
            ::std::option::Option<&'static ::init_static::Symbol>,
        > {
            use ::init_static::__private::MaybeInitStatic;
            ::std::vec![(& B1).__get_symbol()]
        }
        ::init_static::__private::Init {
            symbol: ::init_static::InitStatic::symbol(&A1),
            init: ::init_static::__private::InitFn::Sync(INIT_A1),
            deps: DEPS_A1,
            before: &[::init_static::InitStatic::symbol(&A2)],
            after: &[
                ::init_static::InitStatic::symbol(&A0),
                ::init_static::InitStatic::symbol(&B1),
            ],
            priority: 0i32,
        }
    };
    #[::init_static::__private::linkme::distributed_slice(
        ::init_static::__private::INIT
    )]
    #[linkme(crate = ::init_static::__private::linkme)]
    static INIT_A2: ::init_static::__private::Init = {
        #[allow(non_snake_case)]
        fn INIT_A2() -> ::init_static::__private::anyhow::Result<()> {
            ::init_static::InitStatic::init(&A2, 2);
            Ok(())
        }
        ::init_static::__private::Init {
            symbol: ::init_static::InitStatic::symbol(&A2),
            init: ::init_static::__private::InitFn::Sync(INIT_A2),
            deps: ::std::vec::Vec::new,
            before: &[],
            after: &[],
            priority: 0i32,
        }
    };
};
