//! `#[derive(Validate)]` expansion.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Data, DeriveInput, Error, Expr, Fields, Ident, LitStr, Result, Type,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
};

enum Rule {
    Required,
    Length {
        min: Option<Expr>,
        max: Option<Expr>,
    },
    Range {
        min: Option<Expr>,
        max: Option<Expr>,
    },
    Email,
    Url,
    AlphaNum,
    OneOf(Vec<LitStr>),
    Confirmed,
    Custom(syn::Path),
    Nested,
}

struct RuleList(Vec<Rule>);

impl Parse for RuleList {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let metas: Punctuated<syn::Meta, Comma> = Punctuated::parse_terminated(input)?;
        let mut rules = Vec::new();
        for meta in metas {
            rules.push(parse_rule(&meta)?);
        }
        Ok(Self(rules))
    }
}

fn parse_rule(meta: &syn::Meta) -> Result<Rule> {
    let name = meta
        .path()
        .get_ident()
        .map(ToString::to_string)
        .ok_or_else(|| Error::new(meta.span(), "expected a rule name"))?;
    match (name.as_str(), meta) {
        ("required", syn::Meta::Path(_)) => Ok(Rule::Required),
        ("email", syn::Meta::Path(_)) => Ok(Rule::Email),
        ("url", syn::Meta::Path(_)) => Ok(Rule::Url),
        ("alpha_num", syn::Meta::Path(_)) => Ok(Rule::AlphaNum),
        ("confirmed", syn::Meta::Path(_)) => Ok(Rule::Confirmed),
        ("nested", syn::Meta::Path(_)) => Ok(Rule::Nested),
        ("length" | "range", syn::Meta::List(list)) => {
            let mut min = None;
            let mut max = None;
            list.parse_nested_meta(|m| {
                if m.path.is_ident("min") {
                    min = Some(m.value()?.parse::<Expr>()?);
                } else if m.path.is_ident("max") {
                    max = Some(m.value()?.parse::<Expr>()?);
                } else {
                    return Err(m.error("expected `min = ..` and/or `max = ..`"));
                }
                Ok(())
            })?;
            if min.is_none() && max.is_none() {
                return Err(Error::new(
                    list.span(),
                    "`length`/`range` need `min` and/or `max`",
                ));
            }
            Ok(if name == "length" {
                Rule::Length { min, max }
            } else {
                Rule::Range { min, max }
            })
        }
        ("one_of", syn::Meta::List(list)) => {
            let options: Punctuated<LitStr, Comma> =
                list.parse_args_with(Punctuated::parse_terminated)?;
            if options.is_empty() {
                return Err(Error::new(
                    list.span(),
                    "`one_of` needs at least one option",
                ));
            }
            Ok(Rule::OneOf(options.into_iter().collect()))
        }
        ("custom", syn::Meta::NameValue(nv)) => {
            let Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(path),
                ..
            }) = &nv.value
            else {
                return Err(Error::new(
                    nv.value.span(),
                    "`custom` expects a string path",
                ));
            };
            Ok(Rule::Custom(path.parse()?))
        }
        _ => Err(Error::new(
            meta.span(),
            "unknown rule; expected required, length(min, max), range(min, max), email, url, \
             alpha_num, one_of(\"a\", \"b\"), confirmed, custom = \"path\" or nested",
        )),
    }
}

fn is_option(ty: &Type) -> bool {
    matches!(ty, Type::Path(p) if p.path.segments.last().is_some_and(|s| s.ident == "Option"))
}

pub(crate) fn expand(input: TokenStream) -> Result<TokenStream> {
    let derive: DeriveInput = syn::parse2(input)?;
    let Data::Struct(data) = &derive.data else {
        return Err(Error::new(
            derive.ident.span(),
            "`#[derive(Validate)]` only supports structs",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(Error::new(
            derive.ident.span(),
            "`#[derive(Validate)]` requires named fields",
        ));
    };

    let mut krate: syn::Path = syn::parse_quote!(::rustali::validate);
    for attr in &derive.attrs {
        if attr.path().is_ident("validate") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("crate") {
                    let lit: LitStr = meta.value()?.parse()?;
                    krate = lit.parse()?;
                    Ok(())
                } else {
                    Err(meta.error("unknown `validate` option; expected `crate = \"...\"`"))
                }
            })?;
        }
    }
    let private = quote!(#krate::__private);

    let mut checks = Vec::new();
    for field in &fields.named {
        let ident = field
            .ident
            .clone()
            .ok_or_else(|| Error::new(field.span(), "unnamed field"))?;
        let name = ident.to_string();
        let optional = is_option(&field.ty);
        for attr in &field.attrs {
            if !attr.path().is_ident("rule") {
                continue;
            }
            let RuleList(rules) = attr.parse_args()?;
            for rule in rules {
                checks.push(check_for(&private, &ident, &name, optional, &rule));
            }
        }
    }

    let ident = &derive.ident;
    let (impl_generics, ty_generics, where_clause) = derive.generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics #private::Validate for #ident #ty_generics #where_clause {
            fn validate(&self) -> ::core::result::Result<(), #private::ValidationErrors> {
                let mut errors = #private::ValidationErrors::new();
                #(#checks)*
                errors.into_result()
            }
        }
    })
}

/// Generate the check for one rule. For string/number rules the predicate runs on the
/// inner value, so optional fields skip the rule when `None` (only `required` and
/// `confirmed`/`nested` see the field directly).
fn check_for(
    private: &TokenStream,
    ident: &Ident,
    name: &str,
    optional: bool,
    rule: &Rule,
) -> TokenStream {
    let msgs = quote!(#private::rules::messages);
    match rule {
        Rule::Required => required_check(private, ident, name, optional),
        Rule::Confirmed => {
            let confirmation = quote::format_ident!("{name}_confirmation");
            quote! {
                if self.#ident != self.#confirmation {
                    errors.add(#name, #msgs::confirmed(#name));
                }
            }
        }
        Rule::Custom(path) => quote! {
            if let ::core::result::Result::Err(message) = #path(&self.#ident) {
                errors.add(#name, #msgs::custom(message));
            }
        },
        Rule::Nested => nested_check(private, ident, name, optional),
        value_rule => guard(ident, optional, &value_check(private, name, value_rule)),
    }
}

/// Wrap a value-rule body so it binds `value = &inner` and skips `None` options.
fn guard(ident: &Ident, optional: bool, body: &TokenStream) -> TokenStream {
    if optional {
        quote! {
            if let ::core::option::Option::Some(value) = &self.#ident {
                #body
            }
        }
    } else {
        quote! {{
            let value = &self.#ident;
            #body
        }}
    }
}

/// The body for a rule that inspects the (bound) `value`.
fn value_check(private: &TokenStream, name: &str, rule: &Rule) -> TokenStream {
    let rules = quote!(#private::rules);
    let msgs = quote!(#private::rules::messages);
    let as_str = quote!(::core::convert::AsRef::<str>::as_ref(value));
    match rule {
        Rule::Length { min, max } => {
            let (min_t, max_t) = (opt_tokens(min.as_ref()), opt_tokens(max.as_ref()));
            quote! {
                if !#rules::length(#as_str, #min_t, #max_t) {
                    errors.add(#name, #msgs::length(#name, #min_t, #max_t));
                }
            }
        }
        Rule::Range { min, max } => {
            let (min_t, max_t) = (opt_f64(min.as_ref()), opt_f64(max.as_ref()));
            quote! {
                #[allow(clippy::cast_precision_loss, clippy::cast_lossless)]
                let number = *value as f64;
                if !#rules::range(number, #min_t, #max_t) {
                    errors.add(#name, #msgs::range(#name, #min_t, #max_t));
                }
            }
        }
        Rule::Email => quote! {
            if !#rules::email(#as_str) { errors.add(#name, #msgs::email(#name)); }
        },
        Rule::Url => quote! {
            if !#rules::url(#as_str) { errors.add(#name, #msgs::url(#name)); }
        },
        Rule::AlphaNum => quote! {
            if !#rules::alpha_num(#as_str) { errors.add(#name, #msgs::alpha_num(#name)); }
        },
        Rule::OneOf(options) => quote! {
            if !#rules::one_of(#as_str, &[#(#options),*]) {
                errors.add(#name, #msgs::one_of(#name, &[#(#options),*]));
            }
        },
        // The non-value rules are handled by `check_for`; unreachable here.
        _ => quote!(),
    }
}

fn required_check(private: &TokenStream, ident: &Ident, name: &str, optional: bool) -> TokenStream {
    let rules = quote!(#private::rules);
    let msgs = quote!(#private::rules::messages);
    if optional {
        // `Some("")` / `Some("  ")` fail; `None` fails; `Some(non-blank)` passes.
        quote! {
            match &self.#ident {
                ::core::option::Option::Some(value)
                    if #rules::required_str(::core::convert::AsRef::<str>::as_ref(value)) => {}
                _ => errors.add(#name, #msgs::required(#name)),
            }
        }
    } else {
        quote! {
            if !#rules::required_str(::core::convert::AsRef::<str>::as_ref(&self.#ident)) {
                errors.add(#name, #msgs::required(#name));
            }
        }
    }
}

fn nested_check(private: &TokenStream, ident: &Ident, name: &str, optional: bool) -> TokenStream {
    let merge = quote! {
        for (field, messages) in nested.fields {
            for message in messages {
                errors.add(::std::format!("{}.{}", #name, field), message);
            }
        }
    };
    if optional {
        quote! {
            if let ::core::option::Option::Some(inner) = &self.#ident
                && let ::core::result::Result::Err(nested) = #private::Validate::validate(inner)
            {
                #merge
            }
        }
    } else {
        quote! {
            if let ::core::result::Result::Err(nested) = #private::Validate::validate(&self.#ident) {
                #merge
            }
        }
    }
}

fn opt_tokens(expr: Option<&Expr>) -> TokenStream {
    expr.map_or_else(
        || quote!(::core::option::Option::None),
        |e| quote!(::core::option::Option::Some(#e)),
    )
}

fn opt_f64(expr: Option<&Expr>) -> TokenStream {
    expr.map_or_else(
        || quote!(::core::option::Option::None),
        |e| quote!(::core::option::Option::Some((#e) as f64)),
    )
}
