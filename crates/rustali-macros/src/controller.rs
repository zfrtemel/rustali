//! `#[controller("/prefix")]` expansion.

#![allow(clippy::too_many_lines)]

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    Error, Expr, ImplItem, ImplItemFn, ItemImpl, LitStr, Result, Token,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

const METHODS: &[(&str, &str)] = &[
    ("get", "GET"),
    ("post", "POST"),
    ("put", "PUT"),
    ("patch", "PATCH"),
    ("delete", "DELETE"),
    ("head", "HEAD"),
    ("options", "OPTIONS"),
];

/// `#[controller("/posts")]` or `#[controller("/posts", crate = "rustali_core")]`.
struct Args {
    prefix: String,
    krate: syn::Path,
}

impl Parse for Args {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut prefix = String::new();
        let mut krate: syn::Path = syn::parse_quote!(::rustali);
        if input.peek(LitStr) {
            prefix = input.parse::<LitStr>()?.value();
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        while !input.is_empty() {
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: LitStr = input.parse()?;
            if key == "crate" {
                krate = value.parse()?;
            } else {
                return Err(Error::new(
                    key.span(),
                    "unknown option; expected `crate = \"...\"`",
                ));
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(Self { prefix, krate })
    }
}

/// `#[get("/{id}")]` or `#[get("/{id}", name = "posts.show")]`.
struct RouteArgs {
    path: String,
    name: Option<String>,
}

impl Parse for RouteArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let path = input.parse::<LitStr>()?.value();
        let mut name = None;
        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }
            let key: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let value: LitStr = input.parse()?;
            if key == "name" {
                name = Some(value.value());
            } else {
                return Err(Error::new(
                    key.span(),
                    "unknown route option; expected `name = \"...\"`",
                ));
            }
        }
        Ok(Self { path, name })
    }
}

/// `#[authorize("ability")]` or `#[authorize(role = "admin")]`.
struct AuthorizeArgs(String);

impl Parse for AuthorizeArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(LitStr) {
            return Ok(Self(input.parse::<LitStr>()?.value()));
        }
        let key: syn::Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: LitStr = input.parse()?;
        if key == "role" {
            Ok(Self(format!("role:{}", value.value())))
        } else {
            Err(Error::new(
                key.span(),
                "expected `\"ability\"` or `role = \"name\"`",
            ))
        }
    }
}

struct Route {
    method_const: &'static str,
    method_filter: syn::Ident,
    path: String,
    name: Option<String>,
    handler: syn::Ident,
    middleware: Vec<Expr>,
    authorize: Option<String>,
}

pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let args: Args = syn::parse2(attr)?;
    let mut imp: ItemImpl = syn::parse2(item)?;

    if imp.trait_.is_some() {
        return Err(Error::new(
            imp.span(),
            "`#[controller]` goes on an inherent `impl`, not a trait impl",
        ));
    }
    let self_ty = &imp.self_ty;
    let type_name = match &**self_ty {
        syn::Type::Path(path) => path.path.segments.last().map(|s| s.ident.to_string()),
        _ => None,
    }
    .ok_or_else(|| Error::new(self_ty.span(), "`#[controller]` needs a plain type name"))?;

    // Controller-level middleware.
    let mut controller_middleware: Vec<Expr> = Vec::new();
    let mut kept_attrs = Vec::new();
    for attr in imp.attrs.drain(..) {
        if attr.path().is_ident("middleware") {
            controller_middleware.push(attr.parse_args()?);
        } else {
            kept_attrs.push(attr);
        }
    }
    imp.attrs = kept_attrs;

    let mut routes: Vec<Route> = Vec::new();
    for item in &mut imp.items {
        let ImplItem::Fn(method) = item else { continue };
        routes.extend(extract_route(method, &args.prefix)?);
    }
    if routes.is_empty() {
        return Err(Error::new(
            self_ty.span(),
            "`#[controller]` found no routes; annotate methods with #[get(\"/\")], #[post(..)], ...",
        ));
    }

    let krate = &args.krate;
    let axum = quote!(#krate::__private::axum);
    let type_name_lit = LitStr::new(&type_name, Span::call_site());

    // Group routes by path, preserving first-seen order, so several methods on the
    // same path become one `MethodRouter`.
    let mut groups: Vec<(String, Vec<&Route>)> = Vec::new();
    for route in &routes {
        match groups.iter_mut().find(|(path, _)| *path == route.path) {
            Some((_, list)) => list.push(route),
            None => groups.push((route.path.clone(), vec![route])),
        }
    }

    let route_registrations = groups.iter().map(|(path, list)| {
        let path_lit = LitStr::new(path, Span::call_site());
        let ons = list.iter().map(|route| {
            let filter = &route.method_filter;
            let handler = &route.handler;
            let mut handler_expr = quote!(Self::#handler);
            for mw in &route.middleware {
                handler_expr = quote!(#axum::handler::Handler::layer(#handler_expr, #mw));
            }
            if let Some(ability) = &route.authorize {
                let ability_lit = LitStr::new(ability, Span::call_site());
                handler_expr = quote!(#axum::handler::Handler::layer(
                    #handler_expr,
                    #krate::__private::AuthorizeLayer::new(#ability_lit)
                ));
            }
            quote!(.on(#axum::routing::MethodFilter::#filter, #handler_expr))
        });
        quote! {
            router = router.route(
                #path_lit,
                #axum::routing::MethodRouter::<#krate::__private::AppContext>::new() #(#ons)*
            );
        }
    });

    let controller_layers = controller_middleware
        .iter()
        .map(|mw| quote!(router = router.layer(#mw);));
    let controller_mw_strings: Vec<String> = controller_middleware.iter().map(pretty).collect();

    let route_defs = routes.iter().map(|route| {
        let method = route.method_const;
        let path = LitStr::new(&route.path, Span::call_site());
        let name = route.name.as_ref().map_or_else(
            || quote!(::core::option::Option::None),
            |n| quote!(::core::option::Option::Some(#n)),
        );
        let handler = format!("{type_name}::{}", route.handler);
        let mut mws: Vec<String> = route.middleware.iter().map(pretty).collect();
        mws.extend(controller_mw_strings.iter().cloned());
        let authorize = route.authorize.as_ref().map_or_else(
            || quote!(::core::option::Option::None),
            |a| quote!(::core::option::Option::Some(#a)),
        );
        quote! {
            #krate::__private::RouteDef {
                method: #method,
                path: #path,
                name: #name,
                handler: #handler,
                middleware: &[#(#mws),*],
                authorize: #authorize,
            }
        }
    });

    Ok(quote! {
        #imp

        impl #krate::__private::Controller for #self_ty {
            const NAME: &'static str = #type_name_lit;

            fn router() -> #axum::Router<#krate::__private::AppContext> {
                let mut router = #axum::Router::<#krate::__private::AppContext>::new();
                #(#route_registrations)*
                #(#controller_layers)*
                router
            }

            fn routes() -> ::std::vec::Vec<#krate::__private::RouteDef> {
                ::std::vec![#(#route_defs),*]
            }
        }

        #krate::__private::inventory::submit! {
            #krate::__private::Registration::of::<#self_ty>()
        }
    })
}

/// Pull route/middleware/authorize attributes off a method, returning its route (if
/// any) and leaving only ordinary attributes on the method.
fn extract_route(method: &mut ImplItemFn, prefix: &str) -> Result<Option<Route>> {
    let mut route: Option<(&'static str, &'static str, RouteArgs, Span)> = None;
    let mut middleware = Vec::new();
    let mut authorize = None;
    let mut kept = Vec::new();

    for attr in method.attrs.drain(..) {
        let ident = attr.path().get_ident().map(ToString::to_string);
        match ident.as_deref() {
            Some(name) if METHODS.iter().any(|(m, _)| *m == name) => {
                if route.is_some() {
                    return Err(Error::new(
                        attr.span(),
                        "only one route attribute per method",
                    ));
                }
                let (_, upper) = METHODS
                    .iter()
                    .find(|(m, _)| *m == name)
                    .unwrap_or(&("get", "GET"));
                let filter: &'static str = match *upper {
                    "GET" => "GET",
                    "POST" => "POST",
                    "PUT" => "PUT",
                    "PATCH" => "PATCH",
                    "DELETE" => "DELETE",
                    "HEAD" => "HEAD",
                    _ => "OPTIONS",
                };
                let args: RouteArgs = attr.parse_args()?;
                route = Some((upper, filter, args, attr.span()));
            }
            Some("middleware") => middleware.push(attr.parse_args::<Expr>()?),
            Some("authorize") => {
                let args: AuthorizeArgs = attr.parse_args()?;
                authorize = Some(args.0);
            }
            _ => kept.push(attr),
        }
    }
    method.attrs = kept;

    let Some((method_const, filter, args, span)) = route else {
        if !middleware.is_empty() || authorize.is_some() {
            return Err(Error::new(
                method.sig.ident.span(),
                "`#[middleware]` / `#[authorize]` need a route attribute such as #[get(\"/\")]",
            ));
        }
        return Ok(None);
    };

    if method.sig.asyncness.is_none() {
        return Err(Error::new(span, "route handlers must be `async fn`"));
    }
    if method.sig.receiver().is_some() {
        return Err(Error::new(
            method.sig.ident.span(),
            "route handlers take no `self`; controllers are stateless namespaces (use extractors)",
        ));
    }

    Ok(Some(Route {
        method_const,
        method_filter: format_ident!("{filter}"),
        path: join_path(prefix, &args.path),
        name: args.name,
        handler: method.sig.ident.clone(),
        middleware,
        authorize,
    }))
}

/// Human-readable form of a middleware expression for route listings
/// (`quote!` output spaces every token: `Throttle :: per_minute (60)`).
fn pretty(expr: &Expr) -> String {
    quote!(#expr)
        .to_string()
        .replace(" :: ", "::")
        .replace(" (", "(")
        .replace("( ", "(")
        .replace(" )", ")")
        .replace(" ,", ",")
        .replace(" .", ".")
        .replace(". ", ".")
}

/// Same rules as `rustali_core::router::join_path`, applied at compile time.
fn join_path(prefix: &str, path: &str) -> String {
    let prefix = prefix.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    let mut full = String::new();
    if !prefix.starts_with('/') {
        full.push('/');
    }
    full.push_str(prefix);
    if !path.is_empty() {
        if !full.ends_with('/') {
            full.push('/');
        }
        full.push_str(path);
    }
    if full.is_empty() {
        "/".to_owned()
    } else {
        full
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_path_matches_runtime_rules() {
        assert_eq!(join_path("/posts", "/"), "/posts");
        assert_eq!(join_path("/posts", "{id}"), "/posts/{id}");
        assert_eq!(join_path("", "/"), "/");
        assert_eq!(join_path("api", "ping"), "/api/ping");
    }
}
