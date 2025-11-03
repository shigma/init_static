use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream, Parser};

/// Macro to declare statically stored values with explicit initialization. Similar to
/// [`lazy_static!`](lazy_static::lazy_static), but initialization is not automatic.
///
/// Each static declared using this macro:
///
/// - Wraps the value type in [`InitStatic`](init_static::InitStatic)
/// - Generates an init function that sets the value
/// - Registers the init function in a distributed slice
///
/// The values are initialized when [`init_static`](init_static::init_static) is called.
///
/// # Example
///
/// ```
/// use init_static::init_static;
/// use std::error::Error;
///
/// init_static! {
///     static VALUE: u32 = "42".parse()?;
/// }
///
/// fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
///     init_static()?;
///     println!("{}", *VALUE);
///     Ok(())
/// }
/// ```
#[proc_macro]
pub fn init_static(input: TokenStream) -> TokenStream {
    init_static_inner(input.into()).into()
}

fn parse_repeated<T: Parse>(tokens: TokenStream2) -> syn::Result<Vec<T>> {
    let parser = |input: ParseStream| {
        let mut items = vec![];
        while !input.is_empty() {
            items.push(input.parse::<T>()?);
        }
        Ok(items)
    };
    parser.parse2(tokens)
}

pub(crate) fn init_static_inner(input: TokenStream2) -> TokenStream2 {
    let items = match parse_repeated::<syn::ItemStatic>(input) {
        Ok(items) => items,
        Err(err) => return err.to_compile_error(),
    };

    items
        .into_iter()
        .map(|item| {
            let vis = &item.vis;
            let ident = &item.ident;
            let mutability = &item.mutability;
            let ty = &item.ty;
            let expr = &item.expr;
            let init_fn_name = syn::Ident::new(
                &format!("__init_{}", ident.to_string().to_ascii_lowercase()),
                ident.span(),
            );

            quote! {
                #vis static #mutability #ident: ::init_static::InitStatic<#ty> = ::init_static::InitStatic::new();

                #[::init_static::__private::linkme::distributed_slice(::init_static::__private::INIT_FUNCTIONS)]
                #[linkme(crate = ::init_static::__private::linkme)]
                fn #init_fn_name() -> Result<(), Box<dyn ::std::error::Error + ::std::marker::Send + ::std::marker::Sync>> {
                    ::init_static::InitStatic::init(&#ident, #expr);
                    Ok(())
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod test {
    use std::env::var;
    use std::fs::{create_dir_all, read_to_string, write};
    use std::path::{Path, PathBuf};

    use macro_expand::Context;
    use pretty_assertions::StrComparison;
    use prettyplease::unparse;
    use walkdir::WalkDir;

    use super::*;

    struct TestDiff {
        path: PathBuf,
        expect: String,
        actual: String,
    }

    #[test]
    fn fixtures() {
        let input_dir = "fixtures/input";
        let output_dir = "fixtures/output";
        let mut diffs = vec![];
        let will_emit = var("EMIT").is_ok_and(|v| !v.is_empty());
        for entry in WalkDir::new(input_dir).into_iter().filter_map(Result::ok) {
            let input_path = entry.path();
            if !input_path.is_file() || input_path.extension() != Some("rs".as_ref()) {
                continue;
            }
            let path = input_path.strip_prefix(input_dir).unwrap();
            let output_path = Path::new(output_dir).join(path);
            let input = read_to_string(input_path).unwrap().parse().unwrap();
            let mut ctx = Context::new();
            ctx.register_proc_macro("init_static".into(), init_static_inner);
            let actual = unparse(&syn::parse2(ctx.transform(input)).unwrap());
            let expect_result = read_to_string(&output_path);
            if let Ok(expect) = &expect_result
                && expect == &actual
            {
                continue;
            }
            if will_emit {
                create_dir_all(output_path.parent().unwrap()).unwrap();
                write(output_path, &actual).unwrap();
            }
            if let Ok(expect) = expect_result {
                diffs.push(TestDiff {
                    path: path.to_path_buf(),
                    expect,
                    actual,
                });
            }
        }
        let len = diffs.len();
        for diff in diffs {
            eprintln!("diff {}", diff.path.display());
            eprintln!("{}", StrComparison::new(&diff.expect, &diff.actual));
        }
        if len > 0 && !will_emit {
            panic!("Some tests failed");
        }
    }
}
