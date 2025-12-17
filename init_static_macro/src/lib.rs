use std::collections::HashSet;

use proc_macro::TokenStream;
use proc_macro2::{Span as Span2, TokenStream as TokenStream2};
use quote::quote;
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
    let stmts = match parse_repeated::<syn::Stmt>(input) {
        Ok(stmts) => stmts,
        Err(err) => return err.to_compile_error(),
    };

    let mut init_items = TokenStream2::new();
    let mut init_stmts = TokenStream2::new();
    let mut fn_name = String::from("INIT_STATIC");
    let mut names = vec![];
    let init_deps = Scope::collect_free_idents(&stmts)
        .iter()
        .map(|name| quote! { #name, })
        .collect::<TokenStream2>();

    for stmt in stmts {
        match stmt {
            syn::Stmt::Item(item) => match item {
                syn::Item::Static(item_static) => {
                    let vis = &item_static.vis;
                    let ident = &item_static.ident;
                    let mutability = &item_static.mutability;
                    let ty = &item_static.ty;
                    let expr = &item_static.expr;

                    names.push(ident.to_string());
                    fn_name.push('_');
                    fn_name.push_str(ident.to_string().as_str());

                    init_items.extend(quote! {
                        #vis static #mutability #ident: ::init_static::InitStatic<#ty> = ::init_static::InitStatic::new();
                    });
                    init_stmts.extend(quote! {
                        ::init_static::InitStatic::init(&#ident, #expr);
                    });
                }
                _ => init_items.extend(quote! { #item }),
            },
            _ => init_stmts.extend(quote! { #stmt }),
        }
    }

    let init_fn_ident = syn::Ident::new(&fn_name, Span2::call_site());
    let init_names = names.iter().map(|name| quote! { #name, }).collect::<TokenStream2>();

    quote! {
        #init_items

        #[::init_static::__private::linkme::distributed_slice(::init_static::__private::INIT)]
        #[linkme(crate = ::init_static::__private::linkme)]
        static #init_fn_ident: ::init_static::__private::Init = {
            #[allow(non_snake_case)]
            fn #init_fn_ident() -> std::pin::Pin<Box<dyn Future<Output = Result<(), ::init_static::__private::anyhow::Error>>>> {
                Box::pin(async {
                    #init_stmts
                    Ok(())
                })
            }
            ::init_static::__private::Init {
                init: #init_fn_ident,
                names: &[#init_names],
                deps: &[#init_deps],
            }
        };
    }
}

struct Scope<'i, 'ast> {
    free: &'i mut HashSet<String>,
    locals: HashSet<&'ast syn::Ident>,
}

impl<'i, 'ast> Scope<'i, 'ast> {
    fn collect_free_idents(stmts: &'ast [syn::Stmt]) -> HashSet<String> {
        let mut free = HashSet::new();
        let mut scope = Scope {
            free: &mut free,
            locals: HashSet::new(),
        };
        scope.visit_block_stmts(stmts);
        free
    }

    fn visit_block_stmts(&mut self, stmts: &'ast [syn::Stmt]) {
        let mut locals = HashSet::new();
        for stmt in stmts {
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
            free: self.free,
            locals: locals.union(&self.locals).cloned().collect(),
        };
        for stmt in stmts {
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
    }
}

impl<'i, 'ast> Visit<'ast> for Scope<'i, 'ast> {
    fn visit_expr_path(&mut self, expr_path: &'ast syn::ExprPath) {
        if expr_path.qself.is_none()
            && self.locals.iter().all(|&ident| !expr_path.path.is_ident(ident))
            && let Some(segment) = expr_path.path.segments.last()
        {
            self.free.insert(segment.ident.to_string());
        }
        syn::visit::visit_expr_path(self, expr_path);
    }

    fn visit_pat_ident(&mut self, pat_ident: &'ast syn::PatIdent) {
        self.locals.insert(&pat_ident.ident);
        syn::visit::visit_pat_ident(self, pat_ident);
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.visit_block_stmts(&block.stmts);
        // syn::visit::visit_block(self, block);
    }

    fn visit_expr_closure(&mut self, expr_closure: &'ast syn::ExprClosure) {
        for attrs in &expr_closure.attrs {
            self.visit_attribute(attrs);
        }
        let mut scope = Scope {
            free: self.free,
            locals: self.locals.clone(),
        };
        for pat in &expr_closure.inputs {
            scope.visit_pat(pat);
        }
        scope.visit_return_type(&expr_closure.output);
        scope.visit_expr(&expr_closure.body);
        // syn::visit::visit_expr_closure(self, expr_closure);
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
