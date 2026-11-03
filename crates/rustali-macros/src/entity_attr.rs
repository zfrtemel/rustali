//! `#[entity]` attribute-macro expansion.
//!
//! A one-word shorthand that adds the derives every entity needs
//! (`Entity + Debug + Clone + Serialize`) so application code never repeats them and
//! never has to import `serde`. It expands to the plain `#[derive(Entity, ...)]` form,
//! so `cargo expand` shows exactly what it stands for.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Error, ItemStruct, LitStr, Result, Token,
    parse::{Parse, ParseStream},
};

/// `#[entity]` or `#[entity(crate = "path")]`.
///
/// `crate` is the base path of the `rustali` meta crate (default `::rustali`); every
/// emitted path is derived from it, so a renamed dependency still resolves.
struct Args {
    krate: syn::Path,
}

impl Parse for Args {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut krate: syn::Path = syn::parse_quote!(::rustali);
        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: LitStr = input.parse()?;
            if key == "crate" {
                krate = value.parse()?;
            } else {
                return Err(Error::new(
                    key.span(),
                    "unknown `entity` option; expected `crate = \"...\"`",
                ));
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(Self { krate })
    }
}

pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let item: ItemStruct = syn::parse2(item).map_err(|_| {
        Error::new(
            proc_macro2::Span::call_site(),
            "`#[entity]` only supports structs",
        )
    })?;
    let Args { krate } = syn::parse2(attr)?;

    // The serde crate path, as a string literal for `#[serde(crate = "...")]`. serde's
    // derive emits `serde::` internally; reached through a re-export it must be told the
    // path explicitly or its generated code fails to compile.
    let serde_crate = quote!(#krate::serde).to_string().replace(' ', "");

    // When the base crate is renamed, forward the ORM path to the `Entity` derive (its
    // own default is `::rustali::orm`). For the common bare `#[entity]` we emit nothing
    // and let the derive use that default.
    let entity_crate_attr = if quote!(#krate).to_string().replace(' ', "") == "::rustali" {
        quote!()
    } else {
        let orm = quote!(#krate::orm).to_string().replace(' ', "");
        quote!(#[entity(crate = #orm)])
    };

    Ok(quote! {
        #[derive(
            #krate::Entity,
            ::core::fmt::Debug,
            ::core::clone::Clone,
            #krate::serde::Serialize
        )]
        #[serde(crate = #serde_crate)]
        #entity_crate_attr
        #item
    })
}
