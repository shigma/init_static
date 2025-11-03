use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::Parser;
use syn::punctuated::Punctuated;

#[proc_macro]
pub fn init_static(input: TokenStream) -> TokenStream {
    init_static_inner(input.into()).into()
}

fn init_static_inner(input: TokenStream2) -> TokenStream2 {
    let items = match Punctuated::<syn::ItemStatic, syn::Token![;]>::parse_terminated.parse2(input) {
        Ok(items) => items,
        Err(err) => return err.to_compile_error(),
    };

    items
        .into_iter()
        .map(|item| {
            let ident = &item.ident;
            let ty = &item.ty;
            let expr = &item.expr;
            let init_fn_name = syn::Ident::new(
                &format!("__init_{}", ident.to_string().to_ascii_lowercase()),
                ident.span(),
            );

            quote! {
                pub static #ident: ::init_static::InitStatic<#ty> = ::init_static::InitStatic::new();

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
