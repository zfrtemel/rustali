//! `#[derive(Entity)]` expansion.
//!
//! Code generators are long by nature and column descriptors are mostly flags, so
//! two pedantic lints are relaxed for this module only.

#![allow(clippy::too_many_lines, clippy::struct_excessive_bools)]

use std::collections::BTreeSet;

use heck::{ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, Error, Field, Fields, Ident, LitInt, LitStr, Result, Type, spanned::Spanned,
};

const INTEGER_TYPES: &[&str] = &["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64"];

struct Column {
    ident: Ident,
    ty: Type,
    name: String,
    nullable: bool,
    primary_key: bool,
    auto_increment: bool,
    unique: bool,
    max_len: Option<u32>,
}

struct Input {
    ident: Ident,
    vis: syn::Visibility,
    krate: syn::Path,
    table: String,
    columns: Vec<Column>,
    skipped: Vec<Ident>,
}

pub(crate) fn expand(input: TokenStream) -> Result<TokenStream> {
    let derive: DeriveInput = syn::parse2(input)?;
    let parsed = parse(&derive)?;
    generate(&parsed)
}

fn parse(input: &DeriveInput) -> Result<Input> {
    let Data::Struct(data) = &input.data else {
        return Err(Error::new(
            input.ident.span(),
            "`#[derive(Entity)]` only supports structs",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(Error::new(
            input.ident.span(),
            "`#[derive(Entity)]` requires named fields",
        ));
    };

    let mut krate: syn::Path = syn::parse_quote!(::rustali::orm);
    let mut table: Option<String> = None;
    for attr in &input.attrs {
        if attr.path().is_ident("table") {
            let lit: LitStr = attr.parse_args()?;
            table = Some(lit.value());
        } else if attr.path().is_ident("entity") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("crate") {
                    let lit: LitStr = meta.value()?.parse()?;
                    krate = lit.parse()?;
                    Ok(())
                } else {
                    Err(meta.error("unknown `entity` option; expected `crate = \"...\"`"))
                }
            })?;
        }
    }
    let table = table.unwrap_or_else(|| format!("{}s", input.ident.to_string().to_snake_case()));

    let mut columns = Vec::new();
    let mut skipped = Vec::new();
    let mut explicit_keys = 0usize;
    for field in &fields.named {
        let ident = field
            .ident
            .clone()
            .ok_or_else(|| Error::new(field.span(), "unnamed field"))?;
        if field.attrs.iter().any(|a| a.path().is_ident("skip")) {
            skipped.push(ident);
            continue;
        }
        let column = parse_column(field, ident)?;
        if column.primary_key {
            explicit_keys += 1;
        }
        columns.push(column);
    }

    match explicit_keys {
        0 => {
            let Some(id) = columns.iter_mut().find(|c| c.ident == "id") else {
                return Err(Error::new(
                    input.ident.span(),
                    "no primary key: mark a field with `#[key]` or add an `id` field",
                ));
            };
            id.primary_key = true;
            id.auto_increment = is_integer(&id.ty);
        }
        1 => {}
        _ => {
            return Err(Error::new(
                input.ident.span(),
                "composite primary keys are not supported yet; use a single `#[key]` field",
            ));
        }
    }

    Ok(Input {
        ident: input.ident.clone(),
        vis: input.vis.clone(),
        krate,
        table,
        columns,
        skipped,
    })
}

fn parse_column(field: &Field, ident: Ident) -> Result<Column> {
    let mut column = Column {
        name: ident.to_string(),
        nullable: is_option(&field.ty),
        ty: field.ty.clone(),
        ident,
        primary_key: false,
        auto_increment: false,
        unique: false,
        max_len: None,
    };

    for attr in &field.attrs {
        if attr.path().is_ident("key") {
            column.primary_key = true;
            column.auto_increment = is_integer(&field.ty);
            if !matches!(attr.meta, syn::Meta::Path(_)) {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("auto") {
                        let lit: syn::LitBool = meta.value()?.parse()?;
                        column.auto_increment = lit.value;
                        Ok(())
                    } else {
                        Err(meta.error("unknown `key` option; expected `auto = true|false`"))
                    }
                })?;
            }
        } else if attr.path().is_ident("column") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    let lit: LitStr = meta.value()?.parse()?;
                    column.name = lit.value();
                } else if meta.path.is_ident("unique") {
                    column.unique = true;
                } else if meta.path.is_ident("max_len") {
                    let lit: LitInt = meta.value()?.parse()?;
                    column.max_len = Some(lit.base10_parse()?);
                } else if meta.path.is_ident("index") || meta.path.is_ident("indexed") {
                    // Recorded for schema tooling; nothing to emit yet.
                } else {
                    return Err(meta.error(
                        "unknown `column` option; expected `name`, `unique`, `max_len` or `index`",
                    ));
                }
                Ok(())
            })?;
        }
    }

    if column.primary_key && column.nullable {
        return Err(Error::new(
            field.span(),
            "a primary key cannot be `Option<_>`",
        ));
    }
    Ok(column)
}

fn type_ident(ty: &Type) -> Option<&Ident> {
    match ty {
        Type::Path(path) => path.path.segments.last().map(|segment| &segment.ident),
        _ => None,
    }
}

fn is_option(ty: &Type) -> bool {
    type_ident(ty).is_some_and(|ident| ident == "Option")
}

fn is_integer(ty: &Type) -> bool {
    type_ident(ty).is_some_and(|ident| INTEGER_TYPES.contains(&ident.to_string().as_str()))
}

fn is_datetime(ty: &Type) -> bool {
    type_ident(ty).is_some_and(|ident| ident.to_string().contains("DateTime"))
}

/// Names that resolve without an import (primitives and the std prelude).
const BUILTIN_TYPES: &[&str] = &[
    "bool", "char", "str", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64",
    "u128", "usize", "f32", "f64", "String", "Vec", "Option", "Box", "Result", "Self",
];

/// Collect single-segment type names used in `ty` (including generic arguments) so
/// the generated module can import them from the parent explicitly.
fn collect_type_idents(ty: &Type, out: &mut BTreeSet<String>) {
    match ty {
        Type::Path(path) => {
            let segments = &path.path.segments;
            if path.qself.is_none() && path.path.leading_colon.is_none() && segments.len() == 1 {
                let name = segments[0].ident.to_string();
                if !BUILTIN_TYPES.contains(&name.as_str()) {
                    out.insert(name);
                }
            }
            for segment in segments {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    for arg in &args.args {
                        if let syn::GenericArgument::Type(inner) = arg {
                            collect_type_idents(inner, out);
                        }
                    }
                }
            }
        }
        Type::Reference(reference) => collect_type_idents(&reference.elem, out),
        Type::Array(array) => collect_type_idents(&array.elem, out),
        Type::Slice(slice) => collect_type_idents(&slice.elem, out),
        Type::Tuple(tuple) => tuple
            .elems
            .iter()
            .for_each(|elem| collect_type_idents(elem, out)),
        Type::Paren(paren) => collect_type_idents(&paren.elem, out),
        Type::Group(group) => collect_type_idents(&group.elem, out),
        _ => {}
    }
}

fn generate(input: &Input) -> Result<TokenStream> {
    let Input {
        ident,
        vis,
        krate,
        table,
        columns,
        skipped,
    } = input;
    let mod_name = format_ident!("{}_entity", ident.to_string().to_snake_case());
    let sea = quote!(#krate::sea_orm);

    let key = columns
        .iter()
        .find(|c| c.primary_key)
        .ok_or_else(|| Error::new(ident.span(), "no primary key column"))?;
    let key_ident = &key.ident;
    let key_ty = &key.ty;
    let key_variant = format_ident!("{}", key.ident.to_string().to_upper_camel_case());

    let has_created = columns
        .iter()
        .any(|c| c.ident == "created_at" && is_datetime(&c.ty));
    let has_updated = columns
        .iter()
        .any(|c| c.ident == "updated_at" && is_datetime(&c.ty));
    let is_timestamp = |c: &Column| {
        (c.ident == "created_at" && has_created) || (c.ident == "updated_at" && has_updated)
    };

    // --- SeaORM model fields -----------------------------------------------------
    let model_fields = columns.iter().map(|c| {
        let Column { ident, ty, .. } = c;
        let mut opts: Vec<TokenStream> = Vec::new();
        if c.primary_key {
            opts.push(quote!(primary_key));
            if !c.auto_increment {
                opts.push(quote!(auto_increment = false));
            }
        }
        if *ident != c.name {
            let name = &c.name;
            opts.push(quote!(column_name = #name));
        }
        if c.unique {
            opts.push(quote!(unique));
        }
        let attr = if opts.is_empty() {
            quote!()
        } else {
            quote!(#[sea_orm(#(#opts),*)])
        };
        quote! { #attr pub #ident: #ty, }
    });

    // --- timestamps hook ---------------------------------------------------------
    let set_created = has_created.then(|| {
        quote! {
            if insert && matches!(self.created_at, #sea::ActiveValue::NotSet) {
                self.created_at = #sea::ActiveValue::Set(now);
            }
        }
    });
    let set_updated =
        has_updated.then(|| quote! { self.updated_at = #sea::ActiveValue::Set(now); });
    let behavior = if has_created || has_updated {
        quote! {
            #[#sea::prelude::async_trait::async_trait]
            impl ActiveModelBehavior for ActiveModel {
                async fn before_save<C: ConnectionTrait>(
                    mut self,
                    _db: &C,
                    insert: bool,
                ) -> ::core::result::Result<Self, DbErr> {
                    let now = #krate::chrono::Utc::now();
                    #set_created
                    #set_updated
                    Ok(self)
                }
            }
        }
    } else {
        quote! { impl ActiveModelBehavior for ActiveModel {} }
    };

    // --- conversions -------------------------------------------------------------
    let col_idents: Vec<&Ident> = columns.iter().map(|c| &c.ident).collect();
    let skipped_defaults = skipped
        .iter()
        .map(|s| quote! { #s: ::core::default::Default::default(), });

    let insert_fields = columns.iter().map(|c| {
        let f = &c.ident;
        if (c.primary_key && c.auto_increment) || is_timestamp(c) {
            quote! { #f: #sea::ActiveValue::NotSet, }
        } else {
            quote! { #f: #sea::ActiveValue::Set(self.#f), }
        }
    });
    let update_fields = columns.iter().map(|c| {
        let f = &c.ident;
        if c.primary_key {
            quote! { #f: #sea::ActiveValue::Unchanged(self.#f), }
        } else if c.ident == "updated_at" && has_updated {
            quote! { #f: #sea::ActiveValue::NotSet, }
        } else {
            quote! { #f: #sea::ActiveValue::Set(self.#f), }
        }
    });

    // --- metadata ----------------------------------------------------------------
    let metas = columns.iter().map(|c| {
        let Column {
            name,
            nullable,
            primary_key,
            auto_increment,
            unique,
            ty,
            max_len,
            ..
        } = c;
        let rust_type = quote!(#ty).to_string().replace(' ', "");
        let max_len = max_len.as_ref().map_or_else(
            || quote!(::core::option::Option::None),
            |n| quote!(::core::option::Option::Some(#n)),
        );
        quote! {
            #krate::ColumnMeta {
                name: #name,
                rust_type: #rust_type,
                nullable: #nullable,
                primary_key: #primary_key,
                auto_increment: #auto_increment,
                unique: #unique,
                max_len: #max_len,
            }
        }
    });

    // --- typed column constants ------------------------------------------------
    let column_consts = columns.iter().map(|c| {
        let const_name = format_ident!("{}", c.ident.to_string().to_shouty_snake_case());
        let variant = format_ident!("{}", c.ident.to_string().to_upper_camel_case());
        let doc = format!("Typed handle for the `{}` column.", c.name);
        quote! {
            #[doc = #doc]
            pub const #const_name: #mod_name::Column = #mod_name::Column::#variant;
        }
    });

    let table_lit = LitStr::new(table, Span::call_site());

    let mut idents = BTreeSet::new();
    for column in columns {
        collect_type_idents(&column.ty, &mut idents);
    }
    let field_type_idents = idents.iter().map(|name| format_ident!("{name}"));

    Ok(quote! {
        #[doc(hidden)]
        #[allow(
            missing_docs,
            unreachable_pub,
            private_interfaces,
            private_bounds,
            unused_imports,
            clippy::all,
            clippy::pedantic,
            clippy::nursery
        )]
        // Same visibility as the struct, so a private entity never leaks through a
        // public module (E0446) and a public one stays reachable.
        #vis mod #mod_name {
            use super::*;
            // SeaORM's derives emit `sea_orm::...` paths and expect its prelude in scope;
            // make both available even when the application only depends on `rustali`.
            use #krate::sea_orm;
            use #sea::entity::prelude::*;
            // The prelude also exports names such as `DateTime`; explicit imports of the
            // types used by the fields win over both globs and remove the ambiguity.
            #(use super::#field_type_idents;)*

            #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
            #[sea_orm(table_name = #table_lit)]
            pub struct Model {
                #(#model_fields)*
            }

            #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
            pub enum Relation {}

            #behavior
        }

        impl ::core::convert::From<#mod_name::Model> for #ident {
            fn from(model: #mod_name::Model) -> Self {
                Self {
                    #(#col_idents: model.#col_idents,)*
                    #(#skipped_defaults)*
                }
            }
        }

        impl ::core::convert::From<#ident> for #mod_name::Model {
            fn from(value: #ident) -> Self {
                Self {
                    #(#col_idents: value.#col_idents,)*
                }
            }
        }

        impl #krate::Entity for #ident {
            type Sea = #mod_name::Entity;
            type Key = #key_ty;

            const TABLE: &'static str = #table_lit;
            const COLUMNS: &'static [#krate::ColumnMeta] = &[#(#metas),*];

            fn key(&self) -> Self::Key {
                ::core::clone::Clone::clone(&self.#key_ident)
            }

            fn into_active_insert(self) -> #mod_name::ActiveModel {
                #mod_name::ActiveModel {
                    #(#insert_fields)*
                }
            }

            fn into_active_update(self) -> #mod_name::ActiveModel {
                #mod_name::ActiveModel {
                    #(#update_fields)*
                }
            }
        }

        #[allow(dead_code, unreachable_pub)]
        impl #ident {
            #(#column_consts)*

            /// Start a typed query on this table.
            pub fn query() -> #krate::Query<Self> {
                #krate::Query::new()
            }

            /// Fetch one row by primary key.
            pub async fn find<C: #sea::ConnectionTrait>(
                db: &C,
                key: #key_ty,
            ) -> #krate::__private::Result<::core::option::Option<Self>> {
                let model = <#mod_name::Entity as #sea::EntityTrait>::find_by_id(key)
                    .one(db)
                    .await
                    .map_err(#krate::map_db_err)?;
                Ok(model.map(::core::convert::Into::into))
            }

            /// Fetch one row by primary key or fail with `Error::NotFound`.
            pub async fn find_or_404<C: #sea::ConnectionTrait>(
                db: &C,
                key: #key_ty,
            ) -> #krate::__private::Result<Self> {
                Self::find(db, key).await?.ok_or(#krate::__private::Error::NotFound)
            }

            /// Every row, in primary-key order.
            pub async fn all<C: #sea::ConnectionTrait>(
                db: &C,
            ) -> #krate::__private::Result<::std::vec::Vec<Self>> {
                use #sea::QueryOrder as _;
                Self::query().order_by_asc(#mod_name::Column::#key_variant).fetch_all(db).await
            }

            /// Number of rows.
            pub async fn count<C: #sea::ConnectionTrait>(
                db: &C,
            ) -> #krate::__private::Result<u64> {
                Self::query().count(db).await
            }

            /// Insert this value and return the stored row (with generated key and
            /// timestamps).
            pub async fn insert<C: #sea::ConnectionTrait>(
                self,
                db: &C,
            ) -> #krate::__private::Result<Self> {
                use #sea::ActiveModelTrait as _;
                let model = <Self as #krate::Entity>::into_active_insert(self)
                    .insert(db)
                    .await
                    .map_err(#krate::map_db_err)?;
                Ok(model.into())
            }

            /// Write every column of this value back to its row.
            pub async fn update<C: #sea::ConnectionTrait>(
                self,
                db: &C,
            ) -> #krate::__private::Result<Self> {
                use #sea::ActiveModelTrait as _;
                let model = <Self as #krate::Entity>::into_active_update(self)
                    .update(db)
                    .await
                    .map_err(#krate::map_db_err)?;
                Ok(model.into())
            }

            /// Delete this row.
            pub async fn delete<C: #sea::ConnectionTrait>(
                self,
                db: &C,
            ) -> #krate::__private::Result<()> {
                let key = <Self as #krate::Entity>::key(&self);
                <#mod_name::Entity as #sea::EntityTrait>::delete_by_id(key)
                    .exec(db)
                    .await
                    .map_err(#krate::map_db_err)?;
                Ok(())
            }
        }
    })
}
