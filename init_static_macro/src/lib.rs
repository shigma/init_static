use std::collections::{BTreeSet, HashSet};

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream, Parser};
use syn::visit::Visit;

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
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn Error>> {
///     init_static().await?;
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
    let input_items = match parse_repeated::<syn::Item>(input) {
        Ok(items) => items,
        Err(err) => return err.to_compile_error(),
    };

    let mut output = TokenStream2::new();
    let mut inner = TokenStream2::new();

    for item in input_items {
        let syn::Item::Static(item_static) = item else {
            output.extend(quote! { #item });
            continue;
        };

        let mut is_async = false;
        let mut free_paths = BTreeSet::new();
        let mut scope = Scope {
            is_async: &mut is_async,
            free_paths: &mut free_paths,
            locals: HashSet::new(),
        };
        scope.visit_item_static(&item_static);

        let item_vis = &item_static.vis;
        let item_ident = &item_static.ident;
        let item_mut = &item_static.mutability;
        let item_ty = &item_static.ty;
        let item_expr = &item_static.expr;
        let span = item_ident.span();
        let init_static = quote_spanned! { span =>
            ::init_static::InitStatic!(#item_ident)
        };
        output.extend(quote! {
            #item_vis static #item_mut #item_ident: ::init_static::InitStatic<#item_ty> = #init_static;
        });

        let (deps_ident, deps_item) = if free_paths.is_empty() {
            (quote! { ::std::vec::Vec::new }, quote! {})
        } else {
            let deps_ident = syn::Ident::new(&format!("DEPS_{item_ident}"), span);
            let deps_stmts = free_paths.iter().map(|path| {
                let path = &path.path;
                quote! {
                    (&#path).__get_symbol()
                }
            });
            (
                quote! { #deps_ident },
                quote! {
                    #[allow(non_snake_case, clippy::needless_borrow)]
                    fn #deps_ident() -> ::std::vec::Vec<::std::option::Option<&'static ::init_static::Symbol>> {
                        use ::init_static::__private::MaybeInitStatic;
                        ::std::vec![#(#deps_stmts),*]
                    }
                },
            )
        };

        let init_ident = syn::Ident::new(&format!("INIT_{item_ident}"), item_ident.span());
        let (init_variant, init_item) = if is_async {
            (
                quote! { Async },
                quote! {
                    #[allow(non_snake_case)]
                    fn #init_ident() -> ::init_static::__private::BoxFuture<::init_static::__private::anyhow::Result<()>> {
                        Box::pin(async {
                            ::init_static::InitStatic::init(&#item_ident, #item_expr);
                            Ok(())
                        })
                    }
                },
            )
        } else {
            (
                quote! { Sync },
                quote! {
                    #[allow(non_snake_case)]
                    fn #init_ident() -> ::init_static::__private::anyhow::Result<()> {
                        ::init_static::InitStatic::init(&#item_ident, #item_expr);
                        Ok(())
                    }
                },
            )
        };
        inner.extend(quote! {
            #[::init_static::__private::linkme::distributed_slice(::init_static::__private::INIT)]
            #[linkme(crate = ::init_static::__private::linkme)]
            static #init_ident: ::init_static::__private::Init = {
                #init_item
                #deps_item
                ::init_static::__private::Init {
                    symbol: ::init_static::InitStatic::symbol(&#item_ident),
                    init: ::init_static::__private::InitFn::#init_variant(#init_ident),
                    deps: #deps_ident,
                }
            };
        });
    }

    quote! {
        #output

        const _: () = {
            #inner
        };
    }
}

struct Path<'ast> {
    path: &'ast syn::Path,
    repr: String,
}

impl<'ast> Path<'ast> {
    fn new(inner: &'ast syn::Path) -> Self {
        let repr = quote! { #inner }.to_string();
        Self { path: inner, repr }
    }
}

impl<'ast> ::std::cmp::PartialEq for Path<'ast> {
    fn eq(&self, other: &Self) -> bool {
        self.repr == other.repr
    }
}

impl<'ast> ::std::cmp::Eq for Path<'ast> {}

impl<'ast> ::std::cmp::PartialOrd for Path<'ast> {
    fn partial_cmp(&self, other: &Self) -> Option<::std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'ast> ::std::cmp::Ord for Path<'ast> {
    fn cmp(&self, other: &Self) -> ::std::cmp::Ordering {
        self.repr.cmp(&other.repr)
    }
}

struct Scope<'a, 'ast> {
    is_async: &'a mut bool,
    free_paths: &'a mut BTreeSet<Path<'ast>>,
    locals: HashSet<&'ast syn::Ident>,
}

impl<'i, 'ast> Visit<'ast> for Scope<'i, 'ast> {
    fn visit_expr_path(&mut self, expr_path: &'ast syn::ExprPath) {
        if expr_path.qself.is_none()
            && self.locals.iter().all(|&ident| !expr_path.path.is_ident(ident))
            // We only consider ALL_CAPS identifiers as statics here.
            && let Some(last_segment) = expr_path.path.segments.last()
            && last_segment.ident == last_segment.ident.to_string().to_ascii_uppercase()
        {
            self.free_paths.insert(Path::new(&expr_path.path));
        }
        syn::visit::visit_expr_path(self, expr_path);
    }

    fn visit_pat_ident(&mut self, pat_ident: &'ast syn::PatIdent) {
        self.locals.insert(&pat_ident.ident);
        syn::visit::visit_pat_ident(self, pat_ident);
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        let mut locals = HashSet::new();
        for stmt in &block.stmts {
            if let syn::Stmt::Item(item) = stmt {
                match item {
                    syn::Item::Const(item_const) => {
                        locals.insert(&item_const.ident);
                    }
                    syn::Item::Static(item_static) => {
                        locals.insert(&item_static.ident);
                    }
                    _ => {}
                }
            }
        }
        let mut scope = Scope {
            is_async: self.is_async,
            free_paths: self.free_paths,
            locals: locals.union(&self.locals).cloned().collect(),
        };
        for stmt in &block.stmts {
            match stmt {
                syn::Stmt::Local(local) => {
                    for attrs in &local.attrs {
                        scope.visit_attribute(attrs);
                    }
                    if let Some(init) = &local.init {
                        scope.visit_local_init(init);
                    }
                    scope.visit_pat(&local.pat);
                    // syn::visit::visit_local(scope, local);
                }
                syn::Stmt::Expr(expr, _) => {
                    scope.visit_expr(expr);
                }
                syn::Stmt::Item(_item) => {
                    // skip
                }
                syn::Stmt::Macro(_macro) => {
                    // skip
                }
            }
            scope.visit_stmt(stmt);
        }
        // syn::visit::visit_block(self, block);
    }

    fn visit_expr_closure(&mut self, expr_closure: &'ast syn::ExprClosure) {
        for attrs in &expr_closure.attrs {
            self.visit_attribute(attrs);
        }
        let mut scope = Scope {
            is_async: self.is_async,
            free_paths: self.free_paths,
            locals: self.locals.clone(),
        };
        for pat in &expr_closure.inputs {
            scope.visit_pat(pat);
        }
        scope.visit_return_type(&expr_closure.output);
        scope.visit_expr(&expr_closure.body);
        // syn::visit::visit_expr_closure(self, expr_closure);
    }

    fn visit_expr_await(&mut self, expr_await: &'ast syn::ExprAwait) {
        *self.is_async = true;
        syn::visit::visit_expr_await(self, expr_await);
    }
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
