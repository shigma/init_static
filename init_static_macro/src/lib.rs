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
/// - Registers the init function, to be found by [`init_static()`](::init_static::init_static())
///
/// The values are initialized when [`init_static()`](::init_static::init_static()) is called.
///
/// With the `init_static_gc` cfg (on Linux or Android, x86_64 or aarch64, linked by lld), only the
/// statics the final binary uses are initialized, and the others are removed by the linker. Statics
/// whose initializer only matters for its side effects should then be annotated with [`keep`].
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

/// Declares a reverse ordering edge: every listed static is initialized *after* the annotated one.
///
/// # Semantics
///
/// The attribute only constrains the order. When both the annotated static and a listed static are
/// initialized, the ordering is guaranteed; when either of them is not, the constraint is ignored.
/// In particular, neither the attribute nor its targets make any static count as used under the
/// `init_static_gc` cfg (see [`keep`]). To make a static actually depend on another one, read the
/// other static in its initializer instead.
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
/// one.
///
/// # Semantics
///
/// The attribute only constrains the order. When both the annotated static and a listed static are
/// initialized, the ordering is guaranteed; when either of them is not, the constraint is ignored.
/// In particular, neither the attribute nor its targets make any static count as used under the
/// `init_static_gc` cfg (see [`keep`]). To make a static actually depend on another one, read the
/// other static in its initializer instead.
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

/// Always initializes the annotated static, even if nothing in the program reads it.
///
/// This only matters when the `init_static_gc` cfg is in effect, i.e. set and supported by the
/// target (Linux or Android on x86_64 or aarch64, linked by lld): statics that the final binary
/// never uses are then neither initialized nor linked in. Annotate statics whose initializer is run
/// purely for its side effects (e.g. registering a plugin) so that they are kept regardless.
/// Without the cfg, every static is initialized anyway and this attribute has no effect.
///
/// # Example
///
/// ```
/// use init_static::init_static;
///
/// fn register_plugin() {}
///
/// init_static! {
///     #[keep]
///     static REGISTER: () = register_plugin();
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
pub fn keep(_attr: TokenStream, item: TokenStream) -> TokenStream {
    stub("keep", item)
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

fn parse_keep(attrs: &[syn::Attribute]) -> syn::Result<(bool, Vec<syn::Ident>)> {
    let mut idents = vec![];
    for attr in attrs {
        if !attr.path().is_ident("keep") {
            continue;
        }
        attr.meta.require_path_only()?;
        if let Some(ident) = attr.path().get_ident() {
            idents.push(ident.clone());
        }
    }
    Ok((!idents.is_empty(), idents))
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
/// static data instead of a `fn` like `deps`. Being evaluated at compile time, they hold no
/// reference to the target static itself, so in GC mode an ordering attribute never keeps its
/// target alive.
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

        let (keep, keep_idents) = match parse_keep(&item_static.attrs) {
            Ok(result) => result,
            Err(err) => {
                output.extend(err.to_compile_error());
                continue;
            }
        };

        let stub_uses = [&priority_idents, &before_idents, &after_idents, &keep_idents]
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
        let symbol_expr = quote_spanned! { ident_span =>
            ::init_static::Symbol!(#item_ident)
        };

        let (deps_ident, deps_item) = if free_paths.is_empty() {
            (quote! { ::std::vec::Vec::new }, quote! {})
        } else {
            let deps_ident = syn::Ident::new(&format!("DEPS_{item_ident}"), ident_span);
            let deps_stmts = free_paths.iter().map(|path| {
                let path = &path.path;
                quote! {
                    (&#path).__get_dep()
                }
            });
            (
                quote! { #deps_ident },
                quote! {
                    #[allow(non_snake_case, clippy::needless_borrow)]
                    fn #deps_ident() -> ::std::vec::Vec<::std::option::Option<::init_static::__private::Dep>> {
                        use ::init_static::__private::MaybeInitStatic;
                        ::std::vec![#(#deps_stmts),*]
                    }
                },
            )
        };

        // `quote!` turns a negative `i32` into a single negative literal token, which rustc accepts
        // but rust-analyzer fails to match against `$init:expr` in `declare!`. Emit the sign as its
        // own token, as in source code.
        let priority_abs = proc_macro2::Literal::u32_unsuffixed(priority.unsigned_abs());
        let priority_expr = if priority < 0 {
            quote! { -#priority_abs }
        } else {
            quote! { #priority_abs }
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
        output.extend(quote! {
            ::init_static::__private::declare! {
                keep: #keep,
                static: {
                    #[allow(clippy::type_complexity)]
                    #item_vis static #item_mut #item_ident: #static_ty
                },
                symbol: #symbol_expr,
                entry: #init_ident = {
                    #stub_uses
                    #init_item
                    #deps_item
                    ::init_static::__private::Init {
                        symbol: ::init_static::InitStatic::symbol(&#item_ident),
                        init: ::init_static::__private::InitFn::#init_variant(#init_ident),
                        deps: #deps_ident,
                        before: #before_expr,
                        after: #after_expr,
                        priority: #priority_expr,
                    }
                },
            }
        });
    }

    output
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
    use syn::ext::IdentExt;
    use walkdir::WalkDir;

    use super::*;

    struct TestDiff {
        path: PathBuf,
        expect: String,
        actual: String,
    }

    /// Like [`unparse`], but also formats the body of every `declare!` invocation, which
    /// prettyplease leaves as a single run of tokens. The tokens themselves are unchanged.
    ///
    /// Only top-level invocations are formatted, which covers every fixture: `init_static!` blocks
    /// nested in other items are left to prettyplease.
    fn unparse_fixture(file: syn::File) -> String {
        let syn::File { shebang, attrs, items } = file;
        let mut output = unparse(&syn::File {
            shebang,
            attrs,
            items: vec![],
        });
        for item in items {
            output += &match item {
                syn::Item::Macro(item_macro)
                    if item_macro
                        .mac
                        .path
                        .segments
                        .last()
                        .is_some_and(|s| s.ident == "declare") =>
                {
                    unparse_declare(&item_macro)
                }
                item => unparse(&syn::File {
                    shebang: None,
                    attrs: vec![],
                    items: vec![item],
                }),
            };
        }
        output
    }

    fn unparse_declare(item_macro: &syn::ItemMacro) -> String {
        let parser = |input: ParseStream| {
            let mut fields = vec![];
            while !input.is_empty() {
                let name = input.call(syn::Ident::parse_any)?;
                input.parse::<syn::Token![:]>()?;
                let value = match name.to_string().as_str() {
                    "keep" => input.parse::<syn::LitBool>()?.value.to_string(),
                    "static" => {
                        let content;
                        syn::braced!(content in input);
                        let decl = content.parse::<TokenStream2>()?;
                        let item = unparse_item(quote! { #decl = (); });
                        let item = item.trim_end().strip_suffix(" = ();").unwrap();
                        format!("{{\n{}\n}}", indent(item))
                    }
                    "symbol" => unparse_expr(&input.parse()?),
                    "entry" => {
                        let entry = input.parse::<syn::Ident>()?;
                        input.parse::<syn::Token![=]>()?;
                        format!("{entry} = {}", unparse_expr(&input.parse()?))
                    }
                    _ => return Err(syn::Error::new(name.span(), "unknown `declare!` field")),
                };
                fields.push(format!("{name}: {value},"));
                if !input.is_empty() {
                    input.parse::<syn::Token![,]>()?;
                }
            }
            Ok(fields)
        };
        let fields = parser.parse2(item_macro.mac.tokens.clone()).unwrap();
        let attrs = &item_macro.attrs;
        let path = &item_macro.mac.path;
        let head = unparse_item(quote! { #(#attrs)* #path! {} });
        let head = head.trim_end().strip_suffix("{}").unwrap();
        format!("{head}{{\n{}\n}}\n", indent(&fields.join("\n")))
    }

    fn unparse_item(tokens: TokenStream2) -> String {
        unparse(&syn::parse2(tokens).unwrap())
    }

    fn unparse_expr(expr: &syn::Expr) -> String {
        let item = unparse_item(quote! { const _: () = #expr; });
        let item = item.trim_end().strip_prefix("const _: () = ").unwrap();
        item.strip_suffix(';').unwrap().to_owned()
    }

    fn indent(text: &str) -> String {
        let lines = text.lines().map(|line| match line {
            "" => String::new(),
            line => format!("    {line}"),
        });
        lines.collect::<Vec<_>>().join("\n")
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
            let actual = unparse_fixture(syn::parse2(ctx.transform(input)).unwrap());
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
