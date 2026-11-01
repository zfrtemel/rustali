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

/// The kind of a declared relation.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RelationKind {
    HasMany,
    HasOne,
    BelongsTo,
    ManyToMany,
}

/// A relation declared with a container attribute, e.g. `#[has_many(Comment)]`.
struct Relation {
    kind: RelationKind,
    /// Target entity type as written (`Comment`), used for `Comment::Sea`.
    target: syn::Path,
    /// Accessor method name (`comments`, `author`); defaults from the target.
    method: Ident,
    /// Meaning depends on `kind`:
    /// - `belongs_to`: foreign-key column on *this* table (required).
    /// - `has_many`/`has_one`: the local column the FK points back to (defaults to PK).
    /// - `many_to_many`: the junction column pointing at *this* entity (required).
    from: Option<Ident>,
    /// Meaning depends on `kind`:
    /// - `belongs_to`: referenced column on the target (defaults to its PK).
    /// - `has_many`/`has_one`: the foreign-key column on the target (required).
    /// - `many_to_many`: the junction column pointing at the *target* (required).
    to: Option<Ident>,
    /// `many_to_many` only: the junction entity type (`PostTag`).
    via: Option<syn::Path>,
}

struct Input {
    ident: Ident,
    vis: syn::Visibility,
    krate: syn::Path,
    table: String,
    columns: Vec<Column>,
    skipped: Vec<Ident>,
    relations: Vec<Relation>,
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
    let mut relations = Vec::new();
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
        } else if let Some(kind) = relation_kind(attr) {
            relations.push(parse_relation(attr, kind)?);
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
        relations,
    })
}

/// Which relation kind, if any, an attribute declares.
fn relation_kind(attr: &syn::Attribute) -> Option<RelationKind> {
    if attr.path().is_ident("has_many") {
        Some(RelationKind::HasMany)
    } else if attr.path().is_ident("has_one") {
        Some(RelationKind::HasOne)
    } else if attr.path().is_ident("belongs_to") {
        Some(RelationKind::BelongsTo)
    } else if attr.path().is_ident("many_to_many") {
        Some(RelationKind::ManyToMany)
    } else {
        None
    }
}

/// Parse `#[has_many(Comment, to = post_id)]`, `#[has_one(Profile, to = user_id)]`,
/// `#[belongs_to(User, from = user_id, to = id, as = author)]`,
/// `#[many_to_many(Tag, via = PostTag, from = post_id, to = tag_id)]`.
///
/// For `has_many`/`has_one` (owner side): `to` is the foreign-key column on the target
/// (required); `from` is the local column it points back to (defaults to the primary
/// key). For `belongs_to` (reference side): `from` is the local foreign-key column
/// (required); `to` is the referenced column on the target (defaults to its primary key).
/// For `many_to_many`: `via` is the junction entity (required), `from` is the junction
/// column pointing at this entity and `to` the junction column pointing at the target
/// (both required).
fn parse_relation(attr: &syn::Attribute, kind: RelationKind) -> Result<Relation> {
    let mut target: Option<syn::Path> = None;
    let mut from: Option<Ident> = None;
    let mut to: Option<Ident> = None;
    let mut rename: Option<Ident> = None;
    let mut via: Option<syn::Path> = None;

    attr.parse_nested_meta(|meta| {
        if meta.input.peek(syn::Token![=]) {
            // key = value form
            if meta.path.is_ident("from") {
                from = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("to") {
                to = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("as") {
                rename = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("via") {
                via = Some(meta.value()?.parse()?);
            } else {
                return Err(
                    meta.error("unknown relation option; expected `from`, `to`, `as` or `via`")
                );
            }
        } else if target.is_none() {
            // the bare target type, e.g. `Comment` or `super::Comment`
            target = Some(meta.path);
        } else {
            return Err(meta.error("unexpected token in relation; expected `key = value`"));
        }
        Ok(())
    })?;

    let target = target.ok_or_else(|| {
        Error::new_spanned(
            attr,
            "relation needs a target type, e.g. #[has_many(Comment)]",
        )
    })?;
    if kind == RelationKind::BelongsTo && from.is_none() {
        return Err(Error::new_spanned(
            attr,
            "`belongs_to` needs the foreign-key column: #[belongs_to(User, from = user_id)]",
        ));
    }
    if matches!(kind, RelationKind::HasMany | RelationKind::HasOne) && to.is_none() {
        return Err(Error::new_spanned(
            attr,
            "`has_many`/`has_one` needs the target's foreign-key column: \
             #[has_many(Comment, to = post_id)]",
        ));
    }
    if kind == RelationKind::ManyToMany && (via.is_none() || from.is_none() || to.is_none()) {
        return Err(Error::new_spanned(
            attr,
            "`many_to_many` needs the junction entity and both junction columns: \
             #[many_to_many(Tag, via = PostTag, from = post_id, to = tag_id)]",
        ));
    }

    // Default accessor name from the target type's last segment.
    let target_name = target
        .segments
        .last()
        .map_or_else(|| "related".to_owned(), |s| s.ident.to_string());
    let method = rename.unwrap_or_else(|| {
        let snake = target_name.to_snake_case();
        let name = match kind {
            RelationKind::HasMany | RelationKind::ManyToMany => format!("{snake}s"), // comments, tags
            _ => snake, // user / profile
        };
        format_ident!("{name}")
    });

    Ok(Relation {
        kind,
        target,
        method,
        from,
        to,
        via,
    })
}

/// The `Relation` enum variant name for a relation, derived from its accessor name
/// (`comments` -> `Comments`, `author` -> `Author`). Unique because accessor names are.
fn rel_variant_ident(rel: &Relation) -> Ident {
    format_ident!("{}", rel.method.to_string().to_upper_camel_case())
}

/// `Comments,` — one arm of the generated `pub enum Relation`.
fn rel_variant(rel: &Relation) -> TokenStream {
    let variant = rel_variant_ident(rel);
    quote! { #variant, }
}

/// `Self::Comments => Entity::has_many(<Target as Entity>::Sea::default_sea()).into(),`
///
/// One match arm of the hand-written `impl RelationTrait`. SeaORM builds the def from
/// `Entity` (the local one, in scope inside the module) and the target's SeaORM entity,
/// reached as `<Target as rustali::orm::Entity>::Sea`.
fn rel_def_arm(
    rel: &Relation,
    krate: &syn::Path,
    sea: &TokenStream,
    key_ident: &Ident,
) -> TokenStream {
    let variant = rel_variant_ident(rel);
    let target = &rel.target;
    let target_sea = quote! { <#target as #krate::Entity>::Sea };
    let target_default = quote! { <#target_sea as ::core::default::Default>::default() };
    let col = |id: &Ident| format_ident!("{}", id.to_string().to_upper_camel_case());

    match rel.kind {
        // has_many / has_one are the "owner" side: `from` is a column on *this* table
        // (defaults to the primary key), `to` is the foreign-key column on the target.
        RelationKind::HasMany | RelationKind::HasOne => {
            let from = rel.from.clone().unwrap_or_else(|| key_ident.clone());
            let from_variant = col(&from);
            // `to` is required for these kinds (enforced in `parse_relation`); the
            // `id` fallback only keeps this total.
            let to = rel.to.clone().unwrap_or_else(|| key_ident.clone());
            let to_variant = col(&to);
            let builder = if rel.kind == RelationKind::HasMany {
                quote!(has_many)
            } else {
                quote!(has_one)
            };
            quote! {
                Self::#variant => Entity::#builder(#target_default)
                    .from(Column::#from_variant)
                    .to(<#target_sea as #sea::EntityTrait>::Column::#to_variant)
                    .into(),
            }
        }
        // belongs_to is the "reference" side: `from` is the foreign-key column on *this*
        // table, `to` is the referenced column on the target (defaults to its PK).
        RelationKind::BelongsTo => {
            // `from` is required for belongs_to (enforced in `parse_relation`); the
            // `key_ident` fallback only keeps this total.
            let from = rel.from.clone().unwrap_or_else(|| key_ident.clone());
            let from_variant = col(&from);
            // SeaORM's builder requires the referenced column; default to the target's
            // `Id` column (the framework's default primary key) when `to` is omitted.
            let to_variant = rel.to.as_ref().map_or_else(|| format_ident!("Id"), col);
            quote! {
                Self::#variant => Entity::belongs_to(#target_default)
                    .from(Column::#from_variant)
                    .to(<#target_sea as #sea::EntityTrait>::Column::#to_variant)
                    .into(),
            }
        }
        // Filtered out before this point: M-N contributes no `Relation` enum variant.
        RelationKind::ManyToMany => quote! {},
    }
}

/// `impl Related<Target::Sea> for Entity { ... }`.
///
/// For direct relations `to()` delegates to the generated `Relation` enum. For
/// many-to-many, `to()` and `via()` are built from the junction entity's columns so no
/// variant on either side's `Relation` enum is required (`via` points at the junction,
/// `to` from the junction to the target).
fn rel_related_impl(
    rel: &Relation,
    krate: &syn::Path,
    sea: &TokenStream,
    key_ident: &Ident,
) -> TokenStream {
    let target = &rel.target;
    let target_sea = quote! { <#target as #krate::Entity>::Sea };

    // `via` is present iff the relation is many-to-many (enforced in `parse_relation`).
    if let Some(via) = rel.via.as_ref() {
        let col = |id: &Ident| format_ident!("{}", id.to_string().to_upper_camel_case());
        let via_sea = quote! { <#via as #krate::Entity>::Sea };
        let target_default = quote! { <#target_sea as ::core::default::Default>::default() };
        // `from` / `to` are the junction's FK columns toward this entity / the target.
        let from = rel.from.clone().unwrap_or_else(|| key_ident.clone());
        let to = rel.to.clone().unwrap_or_else(|| key_ident.clone());
        let from_variant = col(&from);
        let to_variant = col(&to);
        // Mirrors SeaORM's generated many-to-many `Related` impl:
        //   to():  junction -> target  == junction.belongs_to(Target)
        //   via(): this -> junction     == junction.belongs_to(Self).rev()
        return quote! {
            impl #sea::entity::Related<#target_sea> for Entity {
                fn to() -> #sea::entity::RelationDef {
                    #via_sea::belongs_to(#target_default)
                        .from(<#via_sea as #sea::EntityTrait>::Column::#to_variant)
                        .to(<#target_sea as #sea::EntityTrait>::Column::Id)
                        .into()
                }
                fn via() -> ::core::option::Option<#sea::entity::RelationDef> {
                    ::core::option::Option::Some(
                        #sea::entity::RelationDef::rev(
                            #via_sea::belongs_to(Entity::default())
                                .from(<#via_sea as #sea::EntityTrait>::Column::#from_variant)
                                .to(Column::Id)
                                .into()
                        )
                    )
                }
            }
        };
    }

    let variant = rel_variant_ident(rel);
    quote! {
        impl #sea::entity::Related<#target_sea> for Entity {
            fn to() -> #sea::entity::RelationDef {
                Relation::#variant.def()
            }
        }
    }
}

/// Lazy accessor + eager loader methods on the entity struct for one relation.
///
/// - `has_many`: `async fn comments(&self, db) -> Result<Vec<Comment>>`
///   plus `async fn with_comments(parents, db) -> Result<Vec<(Self, Vec<Comment>)>>`.
/// - `has_one`/`belongs_to`: `async fn author(&self, db) -> Result<Option<User>>`
///   plus `async fn with_author(parents, db) -> Result<Vec<(Self, Option<User>)>>`.
/// - `many_to_many`: `async fn tags(&self, db) -> Result<Vec<Tag>>`
///   plus `async fn with_tags(parents, db) -> Result<Vec<(Self, Vec<Tag>)>>`.
fn rel_accessor(rel: &Relation, krate: &syn::Path, sea: &TokenStream) -> TokenStream {
    let method = &rel.method;
    let with_method = format_ident!("with_{method}");
    let target = &rel.target;

    let lazy_doc = format!("Lazily load the `{method}` relation for this row.");
    let eager_doc = format!(
        "Eagerly load the `{method}` relation for many rows in one extra query (no N+1), \
         pairing each row with its related data."
    );

    match rel.kind {
        RelationKind::ManyToMany => {
            // `via` is present for many-to-many (enforced in `parse_relation`); the
            // `target` fallback only keeps this total.
            let via = rel.via.as_ref().unwrap_or(target);
            quote! {
                #[doc = #lazy_doc]
                pub async fn #method<C: #sea::ConnectionTrait>(
                    &self,
                    db: &C,
                ) -> #krate::__private::Result<::std::vec::Vec<#target>> {
                    #krate::__private::relations::load_many::<Self, #target, C>(self, db).await
                }

                #[doc = #eager_doc]
                pub async fn #with_method<C: #sea::ConnectionTrait>(
                    parents: ::std::vec::Vec<Self>,
                    db: &C,
                ) -> #krate::__private::Result<::std::vec::Vec<(Self, ::std::vec::Vec<#target>)>> {
                    #krate::__private::relations::eager_many_to_many::<Self, #target, #via, C>(parents, db).await
                }
            }
        }
        RelationKind::HasMany => quote! {
            #[doc = #lazy_doc]
            pub async fn #method<C: #sea::ConnectionTrait>(
                &self,
                db: &C,
            ) -> #krate::__private::Result<::std::vec::Vec<#target>> {
                #krate::__private::relations::load_many::<Self, #target, C>(self, db).await
            }

            #[doc = #eager_doc]
            pub async fn #with_method<C: #sea::ConnectionTrait>(
                parents: ::std::vec::Vec<Self>,
                db: &C,
            ) -> #krate::__private::Result<::std::vec::Vec<(Self, ::std::vec::Vec<#target>)>> {
                #krate::__private::relations::eager_many::<Self, #target, C>(parents, db).await
            }
        },
        RelationKind::HasOne | RelationKind::BelongsTo => quote! {
            #[doc = #lazy_doc]
            pub async fn #method<C: #sea::ConnectionTrait>(
                &self,
                db: &C,
            ) -> #krate::__private::Result<::core::option::Option<#target>> {
                #krate::__private::relations::load_one::<Self, #target, C>(self, db).await
            }

            #[doc = #eager_doc]
            pub async fn #with_method<C: #sea::ConnectionTrait>(
                parents: ::std::vec::Vec<Self>,
                db: &C,
            ) -> #krate::__private::Result<::std::vec::Vec<(Self, ::core::option::Option<#target>)>> {
                #krate::__private::relations::eager_one::<Self, #target, C>(parents, db).await
            }
        },
    }
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
        relations,
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
    // A nullable `deleted_at` opts the entity into soft delete (gated at runtime by config).
    let has_deleted = columns
        .iter()
        .any(|c| c.ident == "deleted_at" && c.nullable);
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

    // --- soft delete -------------------------------------------------------------
    // An entity with a nullable `deleted_at` can soft-delete: `delete` stamps the column
    // instead of removing the row, and `query` hides trashed rows — both only when the
    // project enabled it in config (checked at runtime). Without the column, these compile
    // to the plain hard-delete / unfiltered query.
    let hard_delete = quote! {
        let key = <Self as #krate::Entity>::key(&self);
        <#mod_name::Entity as #sea::EntityTrait>::delete_by_id(key)
            .exec(db)
            .await
            .map_err(#krate::map_db_err)?;
        ::core::result::Result::Ok(())
    };
    let delete_body = if has_deleted {
        quote! {
            if #krate::soft_delete::enabled() {
                use #sea::ActiveModelTrait as _;
                let mut active = <Self as #krate::Entity>::into_active_update(self);
                active.deleted_at = #sea::ActiveValue::Set(::core::option::Option::Some(
                    #krate::chrono::Utc::now(),
                ));
                active.update(db).await.map_err(#krate::map_db_err)?;
                ::core::result::Result::Ok(())
            } else {
                #hard_delete
            }
        }
    } else {
        hard_delete
    };
    // The starting query: filtered to live rows when soft delete is on for this entity.
    let base_query = if has_deleted {
        quote! {
            {
                let mut q = #krate::Query::<Self>::new();
                if #krate::soft_delete::enabled() {
                    use #sea::{ColumnTrait as _, QueryFilter as _};
                    q = q.filter(#mod_name::Column::DeletedAt.is_null());
                }
                q
            }
        }
    } else {
        quote! { #krate::Query::new() }
    };

    let mut idents = BTreeSet::new();
    for column in columns {
        collect_type_idents(&column.ty, &mut idents);
    }
    let field_type_idents = idents.iter().map(|name| format_ident!("{name}"));

    // --- relations ---------------------------------------------------------------
    // SeaORM: a Relation enum variant + RelationTrait arm per direct relation, and an
    // `impl Related<Target::Sea> for Entity`. We generate the `impl RelationTrait`
    // by hand (the exact expansion of DeriveRelation) so the empty-enum case and the
    // populated case share one path. Many-to-many relations contribute no enum variant
    // (their `Related` impl builds the junction defs inline), so they are filtered out
    // of the enum and `RelationTrait` arms.
    let direct: Vec<&Relation> = relations
        .iter()
        .filter(|r| r.kind != RelationKind::ManyToMany)
        .collect();
    let rel_variants = direct.iter().map(|r| rel_variant(r));
    let rel_defs = direct
        .iter()
        .map(|r| rel_def_arm(r, krate, &sea, key_ident));
    let rel_related = relations
        .iter()
        .map(|r| rel_related_impl(r, krate, &sea, key_ident));
    let rel_methods = relations.iter().map(|r| rel_accessor(r, krate, &sea));

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

            #[derive(Copy, Clone, Debug, EnumIter)]
            pub enum Relation {
                #(#rel_variants)*
            }

            impl #sea::entity::RelationTrait for Relation {
                fn def(&self) -> #sea::entity::RelationDef {
                    match self {
                        #(#rel_defs)*
                        #[allow(unreachable_patterns)]
                        _ => panic!("no RelationDef for this variant"),
                    }
                }
            }

            #(#rel_related)*

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

        #krate::__private::inventory::submit! {
            #krate::__private::EntityRegistration { schema: <#ident as #krate::Entity>::schema }
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

            #(#rel_methods)*

            /// Start a typed query on this table.
            ///
            /// When soft delete is enabled and this entity has a `deleted_at`, the query
            /// starts filtered to live rows (`deleted_at IS NULL`).
            pub fn query() -> #krate::Query<Self> {
                #base_query
            }

            /// Fetch one row by primary key (respecting soft delete).
            pub async fn find<C: #sea::ConnectionTrait>(
                db: &C,
                key: #key_ty,
            ) -> #krate::__private::Result<::core::option::Option<Self>> {
                use #sea::{ColumnTrait as _, QueryFilter as _};
                Self::query()
                    .filter(#mod_name::Column::#key_variant.eq(key))
                    .fetch_one(db)
                    .await
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
                #delete_body
            }
        }
    })
}
