#![allow(rustdoc::broken_intra_doc_links)]

use std::collections::{BTreeSet, HashSet};

use proc_macro::TokenStream;
use proc_macro2::{Span as Span2, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream, Parser};
use syn::spanned::Spanned;
use syn::visit::Visit;

/// Macro to declare statically stored values with explicit initialization. Similar to
/// [`lazy_static!`](lazy_static::lazy_static!), but initialization is not automatic.
///
/// Each static declared using this macro:
///
/// - Wraps the value type in [`InitStatic`](struct@::init_static::InitStatic)
/// - Generates an init function that sets the value
/// - Registers the init function in a distributed slice
///
/// The values are initialized when [`init_static()`](::init_static::init_static()) is called.
///
/// # Example
///
/// ```
/// use init_static::init_static;
///
/// init_static! {
///     static VALUE: u32 = "42".parse()?;
/// }
///
/// #[tokio::main]
/// async fn main() {
///     init_static().await.unwrap();
///     println!("{}", *VALUE);
/// }
/// ```
#[proc_macro]
pub fn init_static(input: TokenStream) -> TokenStream {
    init_static_inner(input.into()).into()
}

/// Emits the body of a stub attribute: these attributes are consumed by [`init_static!`], so
/// reaching the proc macro itself means it was applied somewhere it has no meaning.
fn stub(name: &str, item: TokenStream) -> TokenStream {
    let message = format!("`#[{name}]` is only meaningful on a static inside `init_static!`");
    let error = syn::Error::new(Span2::call_site(), message).to_compile_error();
    let item = TokenStream2::from(item);
    quote! { #error #item }.into()
}

/// Declares a reverse dependency: every listed static is initialized *after* the annotated one.
///
/// # Example
///
/// ```
/// use init_static::init_static;
///
/// init_static! {
///     // SCHEMA is ready before CONNECTION, even though CONNECTION never mentions it.
///     #[before(CONNECTION)]
///     static SCHEMA: u32 = 1;
///     static CONNECTION: u32 = 2;
/// }
/// # fn main() {}
/// ```
///
/// # Note
///
/// This attribute is inert: it is consumed by [`init_static!`] and has no effect anywhere else.
/// Applying it outside an [`init_static!`] block is a compile error.
#[doc(hidden)]
#[proc_macro_attribute]
pub fn before(_attr: TokenStream, item: TokenStream) -> TokenStream {
    stub("before", item)
}

/// Declares a forward ordering edge: every listed static is initialized *before* the annotated
/// one, exactly like a dependency inferred from the initializer expression.
///
/// # Example
///
/// ```
/// use init_static::init_static;
///
/// init_static! {
///     static LOGGER: u32 = 1;
///     // Nothing here reads LOGGER; the ordering is still guaranteed.
///     #[after(LOGGER)]
///     static SERVICE: u32 = 2;
/// }
/// # fn main() {}
/// ```
///
/// # Note
///
/// This attribute is inert: it is consumed by [`init_static!`] and has no effect anywhere else.
/// Applying it outside an [`init_static!`] block is a compile error.
#[doc(hidden)]
#[proc_macro_attribute]
pub fn after(_attr: TokenStream, item: TokenStream) -> TokenStream {
    stub("after", item)
}

/// Assigns an initialization priority to a static (default `0`).
///
/// Statics are initialized in descending priority order: higher values run first, negative values
/// run after the default tier. Each distinct value forms a tier, and a tier fully completes before
/// the next one starts.
///
/// Real dependencies always take precedence over priority. A dependency inherits the highest
/// priority among the nodes that depend on it, so it is never scheduled later than its dependents,
/// no matter what priority it declares.
///
/// # Example
///
/// ```
/// use init_static::init_static;
///
/// init_static! {
///     #[priority(10)]
///     static EARLY: u32 = 1;
///     static NORMAL: u32 = 2;
///     #[priority(-5)]
///     static LATE: u32 = 3;
/// }
/// # fn main() {}
/// ```
///
/// # Note
///
/// This attribute is inert: it is consumed by [`init_static!`] and has no effect anywhere else.
/// Applying it outside an [`init_static!`] block is a compile error.
#[doc(hidden)]
#[proc_macro_attribute]
pub fn priority(_attr: TokenStream, item: TokenStream) -> TokenStream {
    stub("priority", item)
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

fn parse_priority(attrs: &[syn::Attribute]) -> syn::Result<(i32, Vec<syn::Ident>)> {
    let mut priority = 0;
    let mut idents = vec![];
    for attr in attrs {
        if !attr.path().is_ident("priority") {
            continue;
        }
        if let Some(ident) = attr.path().get_ident() {
            idents.push(ident.clone());
        }
        let value = attr.parse_args::<syn::Expr>()?;
        let make_error = || syn::Error::new(value.span(), "expected an integer literal for `priority`");
        priority = match &value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(lit),
                ..
            }) => lit.base10_parse::<i32>()?,
            syn::Expr::Unary(syn::ExprUnary {
                op: syn::UnOp::Neg(_),
                expr,
                ..
            }) => {
                let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(lit),
                    ..
                }) = &**expr
                else {
                    return Err(make_error());
                };
                -lit.base10_parse::<i32>()?
            }
            _ => return Err(make_error()),
        };
    }
    Ok((priority, idents))
}

/// Collects the paths listed by every `#[#name(PATH, ...)]` attribute. Used for both
/// `#[before]` and `#[after]`, which differ only in the direction of the edge they produce.
fn parse_edges(attrs: &[syn::Attribute], name: &str) -> syn::Result<(Vec<syn::Path>, Vec<syn::Ident>)> {
    let mut paths = vec![];
    let mut idents = vec![];
    for attr in attrs {
        if !attr.path().is_ident(name) {
            continue;
        }
        if let Some(ident) = attr.path().get_ident() {
            idents.push(ident.clone());
        }
        let parsed =
            attr.parse_args_with(syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated)?;
        if parsed.is_empty() {
            let message = format!("expected at least one path for `{name}`");
            return Err(syn::Error::new(attr.span(), message));
        }
        paths.extend(parsed);
    }
    Ok((paths, idents))
}

/// Emits an inert `use` of the stub attribute for each occurrence of an attribute the macro
/// consumes.
///
/// The imported ident is the user-written one, so it carries that token's span: editors resolve
/// `#[before(...)]` to `init_static::before` and show its documentation. Underscore imports bind
/// nothing and may repeat, so one attribute can appear several times on the same static.
fn build_stub_uses(idents: &[syn::Ident]) -> TokenStream2 {
    let uses = idents.iter().map(|ident| {
        quote! {
            #[allow(unused_imports)]
            use ::init_static::#ident as _;
        }
    });
    quote! { #(#uses)* }
}

/// Builds the `&'static [&'static Symbol]` expression for an ordering attribute.
///
/// `InitStatic::symbol` takes `&Self`, so naming anything else is a type error at the offending
/// path. Spanning each call at the path makes that error point inside the attribute rather than
/// at the macro call site. It is also a `const fn`, which is what lets these edges be plain
/// static data instead of a `fn` like `deps`.
fn build_edges(paths: &[syn::Path]) -> TokenStream2 {
    let exprs = paths.iter().map(|path| {
        quote_spanned! { path.span() =>
            ::init_static::InitStatic::symbol(&#path)
        }
    });
    quote! { &[#(#exprs),*] }
}

pub(crate) fn init_static_inner(input: TokenStream2) -> TokenStream2 {
    let input_stmts = match parse_repeated::<syn::Stmt>(input) {
        Ok(items) => items,
        Err(err) => return err.to_compile_error(),
    };

    let mut output = TokenStream2::new();
    let mut inner = TokenStream2::new();

    for stmt in input_stmts {
        let syn::Stmt::Item(item) = stmt else {
            output.extend(
                syn::Error::new(stmt.span(), "only static item declarations are allowed in init_static!")
                    .to_compile_error(),
            );
            continue;
        };

        let syn::Item::Static(item_static) = item else {
            output.extend(quote! { #item });
            continue;
        };

        let (priority, priority_idents) = match parse_priority(&item_static.attrs) {
            Ok(result) => result,
            Err(err) => {
                output.extend(err.to_compile_error());
                continue;
            }
        };

        let edges = ["before", "after"].map(|name| parse_edges(&item_static.attrs, name));
        let [Ok((before_paths, before_idents)), Ok((after_paths, after_idents))] = edges else {
            for err in edges.into_iter().filter_map(Result::err) {
                output.extend(err.to_compile_error());
            }
            continue;
        };

        let stub_uses = [&priority_idents, &before_idents, &after_idents]
            .map(|idents| build_stub_uses(idents))
            .into_iter()
            .collect::<TokenStream2>();

        let mut is_try = false;
        let mut is_async = false;
        let mut free_paths = BTreeSet::new();
        let mut scope = Scope {
            is_try: &mut is_try,
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
        let ty_span = item_ty.span();
        let ident_span = item_ident.span();
        let static_ty = quote_spanned! { ty_span =>
            ::init_static::InitStatic<#item_ty>
        };
        let static_expr = quote_spanned! { ident_span =>
            ::init_static::InitStatic!(#item_ident)
        };
        output.extend(quote! {
            #[allow(clippy::type_complexity)]
            #item_vis static #item_mut #item_ident: #static_ty = #static_expr;
        });

        let (deps_ident, deps_item) = if free_paths.is_empty() {
            (quote! { ::std::vec::Vec::new }, quote! {})
        } else {
            let deps_ident = syn::Ident::new(&format!("DEPS_{item_ident}"), ident_span);
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

        let before_expr = build_edges(&before_paths);
        let after_expr = build_edges(&after_paths);

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
                #stub_uses
                #init_item
                #deps_item
                ::init_static::__private::Init {
                    symbol: ::init_static::InitStatic::symbol(&#item_ident),
                    init: ::init_static::__private::InitFn::#init_variant(#init_ident),
                    deps: #deps_ident,
                    before: #before_expr,
                    after: #after_expr,
                    priority: #priority,
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
    is_try: &'a mut bool,
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
            is_try: self.is_try,
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
            is_try: self.is_try,
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

    fn visit_expr_try(&mut self, expr_try: &'ast syn::ExprTry) {
        *self.is_try = true;
        syn::visit::visit_expr_try(self, expr_try);
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
