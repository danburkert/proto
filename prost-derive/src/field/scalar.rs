use std::convert::TryFrom;
use std::fmt;

use anyhow::{anyhow, bail, ensure, Error};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse_str, Ident, Lit, LitByteStr, Meta, MetaList, MetaNameValue, NestedMeta, Path, Type,
};

use crate::field::{bool_attr, set_option, tag_attr, Label, MsgFns};
use crate::options::Options;

/// A scalar protobuf field.
#[derive(Clone)]
pub struct Field {
    pub field_ty: Type,
    pub ty: Ty,
    pub kind: Kind,
    pub tag: u32,
    pub clear: bool,
    pub msg_fns: MsgFns,
}

impl Field {
    pub fn new(
        field_ty: &Type,
        attrs: &[Meta],
        inferred_tag: Option<u32>,
        options: &Options,
    ) -> Result<Option<Field>, Error> {
        let mut ty = None;
        let mut label = None;
        let mut packed = None;
        let mut default = None;
        let mut tag = None;
        let mut clear = None;
        let mut msg_fns = MsgFns::new();

        let mut unknown_attrs = Vec::new();

        for attr in attrs {
            if let Some(t) = Ty::from_attr(attr)? {
                set_option(&mut ty, t, "duplicate type attributes")?;
            } else if let Some(p) = bool_attr("packed", attr)? {
                set_option(&mut packed, p, "duplicate packed attributes")?;
            } else if let Some(t) = tag_attr(attr)? {
                set_option(&mut tag, t, "duplicate tag attributes")?;
            } else if let Some(l) = Label::from_attr(attr) {
                set_option(&mut label, l, "duplicate label attributes")?;
            } else if let Some(d) = DefaultValue::from_attr(attr)? {
                set_option(&mut default, d, "duplicate default attributes")?;
            } else if let Some(c) = bool_attr("clear", attr)? {
                set_option(&mut clear, c, "duplicate clear attributes")?;
            } else if msg_fns.attr(attr)?.is_some() {
                continue;
            } else {
                unknown_attrs.push(attr);
            }
        }

        let ty = match ty {
            Some(ty) => ty,
            None => return Ok(None),
        };

        match unknown_attrs.len() {
            0 => (),
            1 => bail!("unknown attribute: {:?}", unknown_attrs[0]),
            _ => bail!("unknown attributes: {:?}", unknown_attrs),
        }

        let tag = match tag.or(inferred_tag) {
            Some(tag) => tag,
            None => bail!("missing tag attribute"),
        };

        let has_default = default.is_some();
        let default = default.map_or_else(
            || Ok(DefaultValue::new(&ty)),
            |lit| DefaultValue::from_lit(&ty, lit),
        )?;

        let kind = match (label, packed, has_default) {
            (None, Some(true), _)
            | (Some(Label::Optional), Some(true), _)
            | (Some(Label::Required), Some(true), _) => {
                bail!("packed attribute may only be applied to repeated fields");
            }
            (Some(Label::Repeated), Some(true), _) if !ty.is_numeric() => {
                bail!("packed attribute may only be applied to numeric types");
            }
            (Some(Label::Repeated), _, true) => {
                bail!("repeated fields may not have a default value");
            }

            (None, _, _) => Kind::Plain(default),
            (Some(Label::Optional), _, _) => Kind::Optional(default),
            (Some(Label::Required), _, _) => Kind::Required(default),
            (Some(Label::Repeated), packed, false) if packed.unwrap_or_else(|| ty.is_numeric()) => {
                Kind::Packed
            }
            (Some(Label::Repeated), _, false) => Kind::Repeated,
        };

        match kind {
            Kind::Repeated | Kind::Packed => msg_fns.check(true, options)?,
            _ => msg_fns.check(false, options)?,
        }

        Ok(Some(Field {
            field_ty: field_ty.clone(),
            ty,
            kind,
            tag,
            clear: clear.unwrap_or(true),
            msg_fns,
        }))
    }

    pub fn new_oneof(attrs: &[Meta], options: &Options) -> Result<Option<Field>, Error> {
        if let Some(mut field) = Field::new(&Type::Verbatim(quote!()), attrs, None, options)? {
            ensure!(
                field.msg_fns.is_empty(),
                "oneof messages cannot use as_msg, to_msg, from_msg, merge_msg, as_msgs or to_msgs",
            );

            match field.kind {
                Kind::Plain(default) => {
                    field.kind = Kind::Required(default);
                    Ok(Some(field))
                }
                Kind::Optional(..) => bail!("invalid optional attribute on oneof field"),
                Kind::Required(..) => bail!("invalid required attribute on oneof field"),
                Kind::Packed | Kind::Repeated => bail!("invalid repeated attribute on oneof field"),
            }
        } else {
            Ok(None)
        }
    }

    pub fn encode(&self, ident: TokenStream) -> TokenStream {
        let module = self.ty.module();
        let encoding_mod = quote!(::prost::encoding::#module);
        let tag = self.tag;

        match self.kind {
            Kind::Plain(ref default) => {
                let default = default.typed();
                self.msg_fns.map(
                    &ident,
                    quote! {
                        if *value != #default {
                            #encoding_mod::encode(#tag, value, buf);
                        }
                    },
                )
            }
            Kind::Optional(..) => self.msg_fns.map_as_ref(
                &ident,
                quote! {
                    if let ::core::option::Option::Some(value) = value {
                        #encoding_mod::encode(#tag, value, buf);
                    }
                },
            ),
            Kind::Required(..) => self.msg_fns.map(
                &ident,
                quote! {
                    #encoding_mod::encode(#tag, value, buf);
                },
            ),
            Kind::Repeated => self.msg_fns.for_each(
                &ident,
                quote! {
                    #encoding_mod::encode(#tag, value, buf);
                },
            ),
            Kind::Packed => {
                let get_slice = self.msg_fns.get_slice(&ident);
                quote! {
                    #encoding_mod::encode_packed(#tag, #get_slice, buf);
                }
            }
        }
    }

    /// Returns an expression which evaluates to the result of merging a decoded
    /// scalar value into the field.
    pub fn merge(&self, ident: TokenStream) -> TokenStream {
        let module = self.ty.module();
        let encoding_mod = quote!(::prost::encoding::#module);

        match self.kind {
            Kind::Plain(..) | Kind::Required(..) => {
                let set = self.msg_fns.set(&ident, quote!(msg));
                if let Some(set) = set {
                    quote! {{
                        let mut msg = Default::default();
                        #encoding_mod::merge(wire_type, &mut msg, buf, ctx)
                            .map(|_| #set)
                    }}
                } else {
                    quote! {
                        #encoding_mod::merge(wire_type, #ident, buf, ctx)
                    }
                }
            }
            Kind::Optional(..) => {
                let set = self.msg_fns.set(&ident, quote!(Some(msg)));
                if let Some(set) = set {
                    quote! {{
                        let mut msg = Default::default();
                        #encoding_mod::merge(wire_type, &mut msg, buf, ctx)
                            .map(|_| #set)
                    }}
                } else {
                    quote! {
                        #encoding_mod::merge(
                            wire_type,
                            #ident.get_or_insert_with(Default::default),
                            buf,
                            ctx,
                        )
                    }
                }
            }
            Kind::Repeated | Kind::Packed => {
                let push = self.msg_fns.push(&ident, quote!(msg));
                if let Some(push) = push {
                    quote! {{
                        let mut msg = Default::default();
                        #encoding_mod::merge(wire_type, &mut msg, buf, ctx)
                            .map(|_| #push)
                    }}
                } else {
                    quote! {
                        #encoding_mod::merge_repeated(wire_type, #ident, buf, ctx)
                    }
                }
            }
        }
    }

    /// Returns an expression which evaluates to the encoded length of the field.
    pub fn encoded_len(&self, ident: TokenStream) -> TokenStream {
        let module = self.ty.module();
        let encoding_mod = quote!(::prost::encoding::#module);
        let tag = self.tag;

        match self.kind {
            Kind::Plain(ref default) => {
                let default = default.typed();
                self.msg_fns.map(
                    &ident,
                    quote! {
                        if *value != #default {
                            #encoding_mod::encoded_len(#tag, value)
                        } else {
                            0
                        }
                    },
                )
            }
            Kind::Optional(..) => self.msg_fns.map_as_ref(
                &ident,
                quote! {
                    value.map_or(0, |value| #encoding_mod::encoded_len(#tag, value))
                },
            ),
            Kind::Required(..) => self.msg_fns.map(
                &ident,
                quote! {
                    #encoding_mod::encoded_len(#tag, value)
                },
            ),
            Kind::Repeated => {
                let iter_map = self.msg_fns.iter_map(
                    &ident,
                    quote! {
                        #encoding_mod::encoded_len(#tag, value)
                    },
                );

                quote! {
                    #iter_map.sum::<usize>()
                }
            }
            Kind::Packed => {
                let get_slice = self.msg_fns.get_slice(&ident);
                quote! {
                    #encoding_mod::encoded_len_packed(#tag, #get_slice)
                }
            }
        }
    }

    pub fn clear(&self, ident: TokenStream) -> TokenStream {
        if !self.clear {
            return quote!();
        }

        let ty = &self.ty;
        match self.kind {
            Kind::Plain(ref default) | Kind::Required(ref default) => self
                .msg_fns
                .set(&quote!(&mut #ident), default.owned())
                .unwrap_or_else(|| {
                    let default = default.typed();
                    match ty {
                        Ty::String | Ty::Bytes(..) => quote!(#ident.clear()),
                        _ => quote!(#ident = #default),
                    }
                }),
            Kind::Optional(..) => self
                .msg_fns
                .set(
                    &quote!(&mut #ident),
                    quote! {
                        ::core::option::Option::None
                    },
                )
                .unwrap_or_else(|| {
                    quote! {
                        #ident = ::core::option::Option::None
                    }
                }),
            Kind::Repeated | Kind::Packed if self.msg_fns.as_to_msgs() => self
                .msg_fns
                .set(
                    &quote!(&mut ident),
                    quote! {
                        Default::default()
                    },
                )
                .unwrap_or_else(|| {
                    quote! {
                        #ident = Default::default()
                    }
                }),
            Kind::Repeated | Kind::Packed => quote! {
                #ident.clear()
            },
        }
    }

    pub fn default(&self) -> TokenStream {
        match self.kind {
            Kind::Plain(ref default) | Kind::Required(ref default) => {
                self.msg_fns.from(default.owned())
            }
            Kind::Optional(..) => self.msg_fns.from(quote!(::core::option::Option::None)),
            Kind::Repeated | Kind::Packed if self.msg_fns.as_to_msgs() => quote! {
                Default::default()
            },
            Kind::Repeated | Kind::Packed => quote! {
                ::prost::alloc::vec::Vec::new()
            },
        }
    }

    /// An inner debug wrapper, around the base type.
    fn debug_inner(&self, wrap_name: TokenStream) -> TokenStream {
        if let Ty::Enumeration(ref ty) = self.ty {
            quote! {
                struct #wrap_name<'a>(&'a i32);
                impl<'a> ::core::fmt::Debug for #wrap_name<'a> {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        match #ty::from_i32(*self.0) {
                            None => ::core::fmt::Debug::fmt(&self.0, f),
                            Some(en) => ::core::fmt::Debug::fmt(&en, f),
                        }
                    }
                }
            }
        } else {
            quote! {
                fn #wrap_name<T>(v: T) -> T { v }
            }
        }
    }

    /// Returns a fragment for formatting the field `ident` in `Debug`.
    pub fn debug(&self, wrapper_name: TokenStream) -> TokenStream {
        let wrapper = self.debug_inner(quote!(Inner));
        let inner_ty = self.ty.rust_type();
        let field_ty = &self.field_ty;

        match self.kind {
            Kind::Plain(..) | Kind::Required(..) | Kind::Optional(..)
                if self.msg_fns.as_to_msg() =>
            {
                let get = self.msg_fns.get(&quote!(self.0));
                quote! {
                    struct #wrapper_name<'a>(&'a #field_ty);
                    impl<'a> ::core::fmt::Debug for #wrapper_name<'a> {
                        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                            ::core::fmt::Debug::fmt(#get, f)
                        }
                    }
                }
            }
            Kind::Plain(..) | Kind::Required(..) => self.debug_inner(wrapper_name),
            Kind::Optional(..) => quote! {
                struct #wrapper_name<'a>(&'a ::core::option::Option<#inner_ty>);
                impl<'a> ::core::fmt::Debug for #wrapper_name<'a> {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        #wrapper
                        ::core::fmt::Debug::fmt(&self.0.as_ref().map(Inner), f)
                    }
                }
            },
            Kind::Repeated | Kind::Packed => {
                let for_each = self.msg_fns.for_each(
                    &quote!(self.0),
                    quote! {
                        vec_builder.entry(&Inner(value));
                    },
                );

                quote! {
                    struct #wrapper_name<'a>(&'a #field_ty);
                    impl<'a> ::core::fmt::Debug for #wrapper_name<'a> {
                        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                            #wrapper

                            let mut vec_builder = f.debug_list();
                            #for_each
                            vec_builder.finish()
                        }
                    }
                }
            }
        }
    }

    /// Returns methods to embed in the message.
    pub fn methods(&self, ident: &Ident) -> Option<TokenStream> {
        if !self.msg_fns.is_empty() {
            return None;
        }

        let mut ident_str = ident.to_string();
        if ident_str.starts_with("r#") {
            ident_str = ident_str[2..].to_owned();
        }

        if let Ty::Enumeration(ref ty) = self.ty {
            let set = Ident::new(&format!("set_{}", ident_str), Span::call_site());
            let set_doc = format!("Sets `{}` to the provided enum value.", ident_str);
            Some(match self.kind {
                Kind::Plain(ref default) | Kind::Required(ref default) => {
                    let get_doc = format!(
                        "Returns the enum value of `{}`, \
                         or the default if the field is set to an invalid enum value.",
                        ident_str,
                    );
                    quote! {
                        #[doc=#get_doc]
                        pub fn #ident(&self) -> #ty {
                            #ty::from_i32(self.#ident).unwrap_or(#default)
                        }

                        #[doc=#set_doc]
                        pub fn #set(&mut self, value: #ty) {
                            self.#ident = value as i32;
                        }
                    }
                }
                Kind::Optional(ref default) => {
                    let get_doc = format!(
                        "Returns the enum value of `{}`, \
                         or the default if the field is unset or set to an invalid enum value.",
                        ident_str,
                    );
                    quote! {
                        #[doc=#get_doc]
                        pub fn #ident(&self) -> #ty {
                            self.#ident.and_then(#ty::from_i32).unwrap_or(#default)
                        }

                        #[doc=#set_doc]
                        pub fn #set(&mut self, value: #ty) {
                            self.#ident = ::core::option::Option::Some(value as i32);
                        }
                    }
                }
                Kind::Repeated | Kind::Packed => {
                    let iter_doc = format!(
                        "Returns an iterator which yields the valid enum values contained in `{}`.",
                        ident_str,
                    );
                    let push = Ident::new(&format!("push_{}", ident_str), Span::call_site());
                    let push_doc = format!("Appends the provided enum value to `{}`.", ident_str);
                    quote! {
                        #[doc=#iter_doc]
                        pub fn #ident(&self) -> ::core::iter::FilterMap<
                            ::core::iter::Cloned<::core::slice::Iter<i32>>,
                            fn(i32) -> ::core::option::Option<#ty>,
                        > {
                            self.#ident.iter().cloned().filter_map(#ty::from_i32)
                        }
                        #[doc=#push_doc]
                        pub fn #push(&mut self, value: #ty) {
                            self.#ident.push(value as i32);
                        }
                    }
                }
            })
        } else if let Kind::Optional(ref default) = self.kind {
            let ty = self.ty.rust_ref_type();

            let match_some = if self.ty.is_numeric() {
                quote!(::core::option::Option::Some(val) => val,)
            } else {
                quote!(::core::option::Option::Some(ref val) => &val[..],)
            };

            let get_doc = format!(
                "Returns the value of `{0}`, or the default value if `{0}` is unset.",
                ident_str,
            );

            Some(quote! {
                #[doc=#get_doc]
                pub fn #ident(&self) -> #ty {
                    match self.#ident {
                        #match_some
                        ::core::option::Option::None => #default,
                    }
                }
            })
        } else {
            None
        }
    }
}

/// A scalar protobuf field type.
#[derive(Clone, PartialEq, Eq)]
pub enum Ty {
    Double,
    Float,
    Int32,
    Int64,
    Uint32,
    Uint64,
    Sint32,
    Sint64,
    Fixed32,
    Fixed64,
    Sfixed32,
    Sfixed64,
    Bool,
    String,
    Bytes(BytesTy),
    Enumeration(Path),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BytesTy {
    Vec,
    Bytes,
}

impl BytesTy {
    fn try_from_str(s: &str) -> Result<Self, Error> {
        match s {
            "vec" => Ok(BytesTy::Vec),
            "bytes" => Ok(BytesTy::Bytes),
            _ => bail!("Invalid bytes type: {}", s),
        }
    }

    fn rust_type(&self) -> TokenStream {
        match self {
            BytesTy::Vec => quote! { ::prost::alloc::vec::Vec<u8> },
            BytesTy::Bytes => quote! { ::prost::bytes::Bytes },
        }
    }
}

impl Ty {
    pub fn from_attr(attr: &Meta) -> Result<Option<Ty>, Error> {
        let ty = match *attr {
            Meta::Path(ref name) if name.is_ident("float") => Ty::Float,
            Meta::Path(ref name) if name.is_ident("double") => Ty::Double,
            Meta::Path(ref name) if name.is_ident("int32") => Ty::Int32,
            Meta::Path(ref name) if name.is_ident("int64") => Ty::Int64,
            Meta::Path(ref name) if name.is_ident("uint32") => Ty::Uint32,
            Meta::Path(ref name) if name.is_ident("uint64") => Ty::Uint64,
            Meta::Path(ref name) if name.is_ident("sint32") => Ty::Sint32,
            Meta::Path(ref name) if name.is_ident("sint64") => Ty::Sint64,
            Meta::Path(ref name) if name.is_ident("fixed32") => Ty::Fixed32,
            Meta::Path(ref name) if name.is_ident("fixed64") => Ty::Fixed64,
            Meta::Path(ref name) if name.is_ident("sfixed32") => Ty::Sfixed32,
            Meta::Path(ref name) if name.is_ident("sfixed64") => Ty::Sfixed64,
            Meta::Path(ref name) if name.is_ident("bool") => Ty::Bool,
            Meta::Path(ref name) if name.is_ident("string") => Ty::String,
            Meta::Path(ref name) if name.is_ident("bytes") => Ty::Bytes(BytesTy::Vec),
            Meta::NameValue(MetaNameValue {
                ref path,
                lit: Lit::Str(ref l),
                ..
            }) if path.is_ident("bytes") => Ty::Bytes(BytesTy::try_from_str(&l.value())?),
            Meta::NameValue(MetaNameValue {
                ref path,
                lit: Lit::Str(ref l),
                ..
            }) if path.is_ident("enumeration") => Ty::Enumeration(parse_str::<Path>(&l.value())?),
            Meta::List(MetaList {
                ref path,
                ref nested,
                ..
            }) if path.is_ident("enumeration") => {
                // TODO(rustlang/rust#23121): slice pattern matching would make this much nicer.
                if nested.len() == 1 {
                    if let NestedMeta::Meta(Meta::Path(ref path)) = nested[0] {
                        Ty::Enumeration(path.clone())
                    } else {
                        bail!("invalid enumeration attribute: item must be an identifier");
                    }
                } else {
                    bail!("invalid enumeration attribute: only a single identifier is supported");
                }
            }
            _ => return Ok(None),
        };
        Ok(Some(ty))
    }

    pub fn from_str(s: &str) -> Result<Ty, Error> {
        let enumeration_len = "enumeration".len();
        let error = Err(anyhow!("invalid type: {}", s));
        let ty = match s.trim() {
            "float" => Ty::Float,
            "double" => Ty::Double,
            "int32" => Ty::Int32,
            "int64" => Ty::Int64,
            "uint32" => Ty::Uint32,
            "uint64" => Ty::Uint64,
            "sint32" => Ty::Sint32,
            "sint64" => Ty::Sint64,
            "fixed32" => Ty::Fixed32,
            "fixed64" => Ty::Fixed64,
            "sfixed32" => Ty::Sfixed32,
            "sfixed64" => Ty::Sfixed64,
            "bool" => Ty::Bool,
            "string" => Ty::String,
            "bytes" => Ty::Bytes(BytesTy::Vec),
            s if s.len() > enumeration_len && &s[..enumeration_len] == "enumeration" => {
                let s = &s[enumeration_len..].trim();
                match s.chars().next() {
                    Some('<') | Some('(') => (),
                    _ => return error,
                }
                match s.chars().next_back() {
                    Some('>') | Some(')') => (),
                    _ => return error,
                }

                Ty::Enumeration(parse_str::<Path>(s[1..s.len() - 1].trim())?)
            }
            _ => return error,
        };
        Ok(ty)
    }

    /// Returns the type as it appears in protobuf field declarations.
    pub fn as_str(&self) -> &'static str {
        match *self {
            Ty::Double => "double",
            Ty::Float => "float",
            Ty::Int32 => "int32",
            Ty::Int64 => "int64",
            Ty::Uint32 => "uint32",
            Ty::Uint64 => "uint64",
            Ty::Sint32 => "sint32",
            Ty::Sint64 => "sint64",
            Ty::Fixed32 => "fixed32",
            Ty::Fixed64 => "fixed64",
            Ty::Sfixed32 => "sfixed32",
            Ty::Sfixed64 => "sfixed64",
            Ty::Bool => "bool",
            Ty::String => "string",
            Ty::Bytes(..) => "bytes",
            Ty::Enumeration(..) => "enum",
        }
    }

    // TODO: rename to 'owned_type'.
    pub fn rust_type(&self) -> TokenStream {
        match self {
            Ty::String => quote!(::prost::alloc::string::String),
            Ty::Bytes(ty) => ty.rust_type(),
            _ => self.rust_ref_type(),
        }
    }

    // TODO: rename to 'ref_type'
    pub fn rust_ref_type(&self) -> TokenStream {
        match *self {
            Ty::Double => quote!(f64),
            Ty::Float => quote!(f32),
            Ty::Int32 => quote!(i32),
            Ty::Int64 => quote!(i64),
            Ty::Uint32 => quote!(u32),
            Ty::Uint64 => quote!(u64),
            Ty::Sint32 => quote!(i32),
            Ty::Sint64 => quote!(i64),
            Ty::Fixed32 => quote!(u32),
            Ty::Fixed64 => quote!(u64),
            Ty::Sfixed32 => quote!(i32),
            Ty::Sfixed64 => quote!(i64),
            Ty::Bool => quote!(bool),
            Ty::String => quote!(&str),
            Ty::Bytes(..) => quote!(&[u8]),
            Ty::Enumeration(..) => quote!(i32),
        }
    }

    pub fn module(&self) -> Ident {
        match *self {
            Ty::Enumeration(..) => Ident::new("int32", Span::call_site()),
            _ => Ident::new(self.as_str(), Span::call_site()),
        }
    }

    /// Returns false if the scalar type is length delimited (i.e., `string` or `bytes`).
    pub fn is_numeric(&self) -> bool {
        !matches!(self, Ty::String | Ty::Bytes(..))
    }
}

impl fmt::Debug for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Scalar Protobuf field types.
#[derive(Clone)]
pub enum Kind {
    /// A plain proto3 scalar field.
    Plain(DefaultValue),
    /// An optional scalar field.
    Optional(DefaultValue),
    /// A required proto2 scalar field.
    Required(DefaultValue),
    /// A repeated scalar field.
    Repeated,
    /// A packed repeated scalar field.
    Packed,
}

/// Scalar Protobuf field default value.
#[derive(Clone, Debug)]
pub enum DefaultValue {
    F64(f64),
    F32(f32),
    I32(i32),
    I64(i64),
    U32(u32),
    U64(u64),
    Bool(bool),
    String(String),
    Bytes(Vec<u8>),
    Enumeration(TokenStream),
    Path(Path),
}

impl DefaultValue {
    pub fn from_attr(attr: &Meta) -> Result<Option<Lit>, Error> {
        if !attr.path().is_ident("default") {
            Ok(None)
        } else if let Meta::NameValue(ref name_value) = *attr {
            Ok(Some(name_value.lit.clone()))
        } else {
            bail!("invalid default value attribute: {:?}", attr)
        }
    }

    pub fn from_lit(ty: &Ty, lit: Lit) -> Result<DefaultValue, Error> {
        let is_i32 = *ty == Ty::Int32 || *ty == Ty::Sint32 || *ty == Ty::Sfixed32;
        let is_i64 = *ty == Ty::Int64 || *ty == Ty::Sint64 || *ty == Ty::Sfixed64;

        let is_u32 = *ty == Ty::Uint32 || *ty == Ty::Fixed32;
        let is_u64 = *ty == Ty::Uint64 || *ty == Ty::Fixed64;

        let empty_or_is = |expected, actual: &str| expected == actual || actual.is_empty();

        let default = match lit {
            Lit::Int(ref lit) if is_i32 && empty_or_is("i32", lit.suffix()) => {
                DefaultValue::I32(lit.base10_parse()?)
            }
            Lit::Int(ref lit) if is_i64 && empty_or_is("i64", lit.suffix()) => {
                DefaultValue::I64(lit.base10_parse()?)
            }
            Lit::Int(ref lit) if is_u32 && empty_or_is("u32", lit.suffix()) => {
                DefaultValue::U32(lit.base10_parse()?)
            }
            Lit::Int(ref lit) if is_u64 && empty_or_is("u64", lit.suffix()) => {
                DefaultValue::U64(lit.base10_parse()?)
            }

            Lit::Float(ref lit) if *ty == Ty::Float && empty_or_is("f32", lit.suffix()) => {
                DefaultValue::F32(lit.base10_parse()?)
            }
            Lit::Int(ref lit) if *ty == Ty::Float => DefaultValue::F32(lit.base10_parse()?),

            Lit::Float(ref lit) if *ty == Ty::Double && empty_or_is("f64", lit.suffix()) => {
                DefaultValue::F64(lit.base10_parse()?)
            }
            Lit::Int(ref lit) if *ty == Ty::Double => DefaultValue::F64(lit.base10_parse()?),

            Lit::Bool(ref lit) if *ty == Ty::Bool => DefaultValue::Bool(lit.value),
            Lit::Str(ref lit) if *ty == Ty::String => DefaultValue::String(lit.value()),
            Lit::ByteStr(ref lit)
                if *ty == Ty::Bytes(BytesTy::Bytes) || *ty == Ty::Bytes(BytesTy::Vec) =>
            {
                DefaultValue::Bytes(lit.value())
            }

            Lit::Str(ref lit) => {
                let value = lit.value();
                let value = value.trim();

                if let Ty::Enumeration(ref path) = *ty {
                    let variant = Ident::new(value, Span::call_site());
                    return Ok(DefaultValue::Enumeration(quote!(#path::#variant)));
                }

                // Parse special floating point values.
                if *ty == Ty::Float {
                    match value {
                        "inf" => {
                            return Ok(DefaultValue::Path(parse_str::<Path>(
                                "::core::f32::INFINITY",
                            )?));
                        }
                        "-inf" => {
                            return Ok(DefaultValue::Path(parse_str::<Path>(
                                "::core::f32::NEG_INFINITY",
                            )?));
                        }
                        "nan" => {
                            return Ok(DefaultValue::Path(parse_str::<Path>("::core::f32::NAN")?));
                        }
                        _ => (),
                    }
                }
                if *ty == Ty::Double {
                    match value {
                        "inf" => {
                            return Ok(DefaultValue::Path(parse_str::<Path>(
                                "::core::f64::INFINITY",
                            )?));
                        }
                        "-inf" => {
                            return Ok(DefaultValue::Path(parse_str::<Path>(
                                "::core::f64::NEG_INFINITY",
                            )?));
                        }
                        "nan" => {
                            return Ok(DefaultValue::Path(parse_str::<Path>("::core::f64::NAN")?));
                        }
                        _ => (),
                    }
                }

                // Rust doesn't have a negative literals, so they have to be parsed specially.
                if let Some(Ok(lit)) = value.strip_prefix('-').map(syn::parse_str::<Lit>) {
                    match lit {
                        Lit::Int(ref lit) if is_i32 && empty_or_is("i32", lit.suffix()) => {
                            // Initially parse into an i64, so that i32::MIN does not overflow.
                            let value: i64 = -lit.base10_parse()?;
                            return Ok(i32::try_from(value).map(DefaultValue::I32)?);
                        }
                        Lit::Int(ref lit) if is_i64 && empty_or_is("i64", lit.suffix()) => {
                            // Initially parse into an i128, so that i64::MIN does not overflow.
                            let value: i128 = -lit.base10_parse()?;
                            return Ok(i64::try_from(value).map(DefaultValue::I64)?);
                        }
                        Lit::Float(ref lit)
                            if *ty == Ty::Float && empty_or_is("f32", lit.suffix()) =>
                        {
                            return Ok(DefaultValue::F32(-lit.base10_parse()?));
                        }
                        Lit::Float(ref lit)
                            if *ty == Ty::Double && empty_or_is("f64", lit.suffix()) =>
                        {
                            return Ok(DefaultValue::F64(-lit.base10_parse()?));
                        }
                        Lit::Int(ref lit) if *ty == Ty::Float && lit.suffix().is_empty() => {
                            return Ok(DefaultValue::F32(-lit.base10_parse()?));
                        }
                        Lit::Int(ref lit) if *ty == Ty::Double && lit.suffix().is_empty() => {
                            return Ok(DefaultValue::F64(-lit.base10_parse()?));
                        }
                        _ => (),
                    }
                }
                match syn::parse_str::<Lit>(&value) {
                    Ok(Lit::Str(_)) => (),
                    Ok(lit) => return DefaultValue::from_lit(ty, lit),
                    _ => (),
                }
                bail!("invalid default value: {}", quote!(#value));
            }
            _ => bail!("invalid default value: {}", quote!(#lit)),
        };

        Ok(default)
    }

    pub fn new(ty: &Ty) -> DefaultValue {
        match *ty {
            Ty::Float => DefaultValue::F32(0.0),
            Ty::Double => DefaultValue::F64(0.0),
            Ty::Int32 | Ty::Sint32 | Ty::Sfixed32 => DefaultValue::I32(0),
            Ty::Int64 | Ty::Sint64 | Ty::Sfixed64 => DefaultValue::I64(0),
            Ty::Uint32 | Ty::Fixed32 => DefaultValue::U32(0),
            Ty::Uint64 | Ty::Fixed64 => DefaultValue::U64(0),

            Ty::Bool => DefaultValue::Bool(false),
            Ty::String => DefaultValue::String(String::new()),
            Ty::Bytes(..) => DefaultValue::Bytes(Vec::new()),
            Ty::Enumeration(ref path) => DefaultValue::Enumeration(quote!(#path::default())),
        }
    }

    pub fn owned(&self) -> TokenStream {
        match *self {
            DefaultValue::String(ref value) if value.is_empty() => {
                quote!(::prost::alloc::string::String::new())
            }
            DefaultValue::String(ref value) => quote!(#value.into()),
            DefaultValue::Bytes(ref value) if value.is_empty() => quote!(Default::default()),
            DefaultValue::Bytes(ref value) => {
                let lit = LitByteStr::new(value, Span::call_site());
                quote!(#lit.as_ref().into())
            }

            ref other => other.typed(),
        }
    }

    pub fn typed(&self) -> TokenStream {
        if let DefaultValue::Enumeration(_) = *self {
            quote!(#self as i32)
        } else {
            quote!(#self)
        }
    }
}

impl ToTokens for DefaultValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match *self {
            DefaultValue::F64(value) => value.to_tokens(tokens),
            DefaultValue::F32(value) => value.to_tokens(tokens),
            DefaultValue::I32(value) => value.to_tokens(tokens),
            DefaultValue::I64(value) => value.to_tokens(tokens),
            DefaultValue::U32(value) => value.to_tokens(tokens),
            DefaultValue::U64(value) => value.to_tokens(tokens),
            DefaultValue::Bool(value) => value.to_tokens(tokens),
            DefaultValue::String(ref value) => value.to_tokens(tokens),
            DefaultValue::Bytes(ref value) => {
                let byte_str = LitByteStr::new(value, Span::call_site());
                tokens.append_all(quote!(#byte_str as &[u8]));
            }
            DefaultValue::Enumeration(ref value) => value.to_tokens(tokens),
            DefaultValue::Path(ref value) => value.to_tokens(tokens),
        }
    }
}
