use std::fmt;

use failure::Error;
use quote::{self, Tokens};
use syn::{
    self,
    FloatTy,
    Ident,
    IntTy,
    Lit,
    MetaItem,
    NestedMetaItem,
    StrStyle,
};

use field::{
    Label,
    bool_attr,
    set_option,
    tag_attr,
};

/// A scalar protobuf field.
#[derive(Clone)]
pub struct Field {
    pub ty: Ty,
    pub kind: Kind,
    pub tag: u32,
}

impl Field {

    pub fn new(attrs: &[MetaItem]) -> Result<Option<Field>, Error> {
        let mut ty = None;
        let mut label = None;
        let mut packed = None;
        let mut default = None;
        let mut tag = None;

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

        let tag = match tag {
            Some(tag) => tag,
            None => bail!("missing tag attribute"),
        };

        let has_default = default.is_some();
        let default = default.map_or_else(|| Ok(DefaultValue::new(&ty)),
                                          |lit| DefaultValue::from_lit(&ty, lit))?;

        let kind = match (label, packed, has_default) {
            (None, Some(true), _) |
            (Some(Label::Optional), Some(true), _) |
            (Some(Label::Required), Some(true), _) => {
                bail!("packed attribute may only be applied to repeated fields");
            },
            (Some(Label::Repeated), Some(true), _) if !ty.is_numeric() => {
                bail!("packed attribute may only be applied to numeric types");
            },
            (Some(Label::Repeated), _, true) => {
                bail!("repeated fields may not have a default value");
            },

            (None, _, _) => Kind::Plain(default),
            (Some(Label::Optional), _, _) => Kind::Optional(default),
            (Some(Label::Required), _, _) => Kind::Required(default),
            (Some(Label::Repeated), packed, false) if packed.unwrap_or(ty.is_numeric()) => Kind::Packed,
            (Some(Label::Repeated), _, false) => Kind::Repeated,
        };

        Ok(Some(Field {
            ty: ty,
            kind: kind,
            tag: tag,
        }))
    }

    pub fn new_oneof(attrs: &[MetaItem]) -> Result<Option<Field>, Error> {
        if let Some(mut field) = Field::new(attrs)? {
            match field.kind {
                Kind::Plain(default) => {
                    field.kind = Kind::Required(default);
                    Ok(Some(field))
                },
                Kind::Optional(..) => bail!("invalid optional attribute on oneof field"),
                Kind::Required(..) => bail!("invalid required attribute on oneof field"),
                Kind::Packed | Kind::Repeated => bail!("invalid repeated attribute on oneof field"),
            }
        } else {
            Ok(None)
        }
    }

    pub fn encode(&self, ident: &Ident) -> Tokens {
        let kind = match self.kind {
            Kind::Plain(..) | Kind::Optional(..) | Kind::Required(..) => "",
            Kind::Repeated => "_repeated",
            Kind::Packed => "_packed",
        };
        let encode_fn = Ident::new(format!("_prost::encoding::{}::encode{}",
                                           self.ty.encode_as(), kind));
        let tag = self.tag;

        match self.kind {
            Kind::Plain(ref default) => {
                let default = default.typed();
                quote! {
                    if #ident != #default {
                        #encode_fn(#tag, &#ident, buf);
                    }
                }
            },
            Kind::Optional(..) => quote! {
                if let ::std::option::Option::Some(ref value) = #ident {
                    #encode_fn(#tag, value, buf);
                }
            },
            Kind::Required(..) | Kind::Repeated | Kind::Packed => quote!{
                #encode_fn(#tag, &#ident, buf);
            },
        }
    }

    /// Returns an expression which evaluates to the result of merging a decoded
    /// scalar value into the field.
    pub fn merge(&self, ident: &Ident) -> Tokens {
        let kind = match self.kind {
            Kind::Plain(..) | Kind::Optional(..) | Kind::Required(..) => "",
            Kind::Repeated | Kind::Packed => "_repeated",
        };
        let merge_fn = Ident::new(format!("_prost::encoding::{}::merge{}",
                                          self.ty.encode_as(), kind));

        match self.kind {
            Kind::Plain(..) | Kind::Required(..) | Kind::Repeated | Kind::Packed => quote! {
                #merge_fn(wire_type, &mut #ident, buf)
            },
            Kind::Optional(..) => quote! {
                #merge_fn(wire_type,
                          #ident.get_or_insert_with(Default::default),
                          buf)
            },
        }
    }

    /// Returns an expression which evaluates to the encoded length of the field.
    pub fn encoded_len(&self, ident: &Ident) -> Tokens {
        let kind = match self.kind {
            Kind::Plain(..) | Kind::Optional(..) | Kind::Required(..) => "",
            Kind::Repeated => "_repeated",
            Kind::Packed => "_packed",
        };
        let encoded_len_fn = Ident::new(format!("_prost::encoding::{}::encoded_len{}",
                                                self.ty.encode_as(), kind));
        let tag = self.tag;

        match self.kind {
            Kind::Plain(ref default) => {
                let default = default.typed();
                quote! {
                    if #ident != #default {
                        #encoded_len_fn(#tag, &#ident)
                    } else {
                        0
                    }
                }
            },
            Kind::Optional(..) => quote! {
                #ident.as_ref().map_or(0, |value| #encoded_len_fn(#tag, value))
            },
            Kind::Required(..) | Kind::Repeated | Kind::Packed => quote!{
                #encoded_len_fn(#tag, &#ident)
            },
        }
    }

    pub fn clear(&self, ident: &Ident) -> Tokens {
        match self.kind {
            Kind::Plain(ref default) | Kind::Required(ref default) => {
                let default = default.typed();
                match self.ty {
                    Ty::String | Ty::Bytes => quote!(#ident.clear()),
                    _ => quote!(#ident = #default),
                }
            },
            Kind::Optional(_) => quote!(#ident = ::std::option::Option::None),
            Kind::Repeated | Kind::Packed => quote!(#ident.clear()),
        }
    }

    /// Returns an expression which evaluates to the default value of the field.
    pub fn default(&self) -> Tokens {
        match self.kind {
            Kind::Plain(ref value) | Kind::Required(ref value) => value.owned(),
            Kind::Optional(_) => quote!(::std::option::Option::None),
            Kind::Repeated | Kind::Packed => quote!(::std::vec::Vec::new()),
        }
    }

    /// An inner debug wrapper, around the base type.
    fn debug_inner(&self, wrap_name: &Ident) -> Tokens {
        if let Ty::Enumeration(ref ty) = self.ty {
            quote! {
                struct #wrap_name<'a>(&'a i32);
                impl<'a> ::std::fmt::Debug for #wrap_name<'a> {
                    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                        match super::#ty::from_i32(*self.0) {
                            None => ::std::fmt::Debug::fmt(&self.0, f),
                            Some(en) => ::std::fmt::Debug::fmt(&en, f),
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
    pub fn debug(&self, wrapper_name: &Ident) -> Tokens {
        let wrapper = self.debug_inner(&Ident::new("Inner"));
        let inner_ty = Ident::new(self.ty.rust_type());
        match self.kind {
            Kind::Plain(_) |
            Kind::Required(_) => self.debug_inner(wrapper_name),
            Kind::Optional(_) => quote! {
                struct #wrapper_name<'a>(&'a ::std::option::Option<#inner_ty>);
                impl<'a> ::std::fmt::Debug for #wrapper_name<'a> {
                    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                        #wrapper
                        ::std::fmt::Debug::fmt(&self.0.as_ref().map(Inner), f)
                    }
                }
            },
            Kind::Repeated |
            Kind::Packed => {
                quote! {
                    struct #wrapper_name<'a>(&'a ::std::vec::Vec<#inner_ty>);
                    impl<'a> ::std::fmt::Debug for #wrapper_name<'a> {
                        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                            let mut vec_builder = f.debug_list();
                            for v in self.0 {
                                #wrapper
                                vec_builder.entry(&Inner(v));
                            }
                            vec_builder.finish()
                        }
                    }
                }
            }
        }
    }

    /// Returns methods to embed in the message.
    pub fn methods(&self, ident: &Ident) -> Option<Tokens> {
        if let Ty::Enumeration(ref ty) = self.ty {
            let set = Ident::new(format!("set_{}", ident));
            let push = Ident::new(format!("push_{}", ident));
            Some(match self.kind {
                Kind::Plain(ref default) | Kind::Required(ref default) => {
                    quote! {
                        pub fn #ident(&self) -> super::#ty {
                            super::#ty::from_i32(self.#ident).unwrap_or(super::#default)
                        }

                        pub fn #set(&mut self, value: super::#ty) {
                            self.#ident = value as i32;
                        }
                    }
                },
                Kind::Optional(ref default) => {
                    quote! {
                        pub fn #ident(&self) -> super::#ty {
                            self.#ident.and_then(super::#ty::from_i32).unwrap_or(super::#default)
                        }

                        pub fn #set(&mut self, value: super::#ty) {
                            self.#ident = ::std::option::Option::Some(value as i32);
                        }
                    }
                },
                Kind::Repeated | Kind::Packed => {
                    quote! {
                        pub fn #ident(&self) -> ::std::iter::FilterMap<::std::iter::Cloned<::std::slice::Iter<i32>>,
                                                                       fn(i32) -> Option<super::#ty>> {
                            self.#ident.iter().cloned().filter_map(super::#ty::from_i32)
                        }
                        pub fn #push(&mut self, value: super::#ty) {
                            self.#ident.push(value as i32);
                        }
                    }
                },
            })
        } else if let Kind::Optional(ref default) = self.kind {
            let ty = Ident::new(self.ty.rust_ref_type());

            let match_some = if self.ty.is_numeric() {
                quote!(::std::option::Option::Some(val) => val,)
            } else {
                quote!(::std::option::Option::Some(ref val) => &val[..],)
            };

            Some(quote! {
                pub fn #ident(&self) -> #ty {
                    match self.#ident {
                        #match_some
                        ::std::option::Option::None => #default,
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
    Bytes,
    Enumeration(Ident),
}

impl Ty {

    pub fn from_attr(attr: &MetaItem) -> Result<Option<Ty>, Error> {
        let ty = match *attr {
            MetaItem::Word(ref name) if name == "float" => Ty::Float,
            MetaItem::Word(ref name) if name == "double" => Ty::Double,
            MetaItem::Word(ref name) if name == "int32" => Ty::Int32,
            MetaItem::Word(ref name) if name == "int64" => Ty::Int64,
            MetaItem::Word(ref name) if name == "uint32" => Ty::Uint32,
            MetaItem::Word(ref name) if name == "uint64" => Ty::Uint64,
            MetaItem::Word(ref name) if name == "sint32" => Ty::Sint32,
            MetaItem::Word(ref name) if name == "sint64" => Ty::Sint64,
            MetaItem::Word(ref name) if name == "fixed32" => Ty::Fixed32,
            MetaItem::Word(ref name) if name == "fixed64" => Ty::Fixed64,
            MetaItem::Word(ref name) if name == "sfixed32" => Ty::Sfixed32,
            MetaItem::Word(ref name) if name == "sfixed64" => Ty::Sfixed64,
            MetaItem::Word(ref name) if name == "bool" => Ty::Bool,
            MetaItem::Word(ref name) if name == "string" => Ty::String,
            MetaItem::Word(ref name) if name == "bytes" => Ty::Bytes,
            MetaItem::NameValue(ref name, Lit::Str(ref ident, _)) if name == "enumeration" => {
                Ty::Enumeration(Ident::new(ident.as_ref()))
            },
            MetaItem::List(ref name, ref items) if name == "enumeration" => {
                // TODO(rustlang/rust#23121): slice pattern matching would make this much nicer.
                if items.len() == 1 {
                    if let NestedMetaItem::MetaItem(MetaItem::Word(ref ident)) = items[0] {
                        Ty::Enumeration(ident.clone())
                    } else {
                        bail!("invalid enumeration attribute: item must be an identifier");
                    }
                } else {
                    bail!("invalid enumeration attribute: only a single identifier is supported");
                }
            },
            _ => return Ok(None),
        };
        Ok(Some(ty))
    }

    pub fn from_str(s: &str) -> Result<Ty, Error> {
        let enumeration_len = "enumeration".len();
        let error = Err(format_err!("invalid type: {}", s));
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
            "bytes" => Ty::Bytes,
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
                Ty::Enumeration(Ident::new(s[1..s.len() - 1].trim()))
            },
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
            Ty::Bytes => "bytes",
            Ty::Enumeration(..) => "enum",
        }
    }

    pub fn rust_type(&self) -> &'static str {
        match *self {
            Ty::String => "::std::string::String",
            Ty::Bytes => "::std::vec::Vec<u8>",
            _ => self.rust_ref_type(),
        }
    }

    pub fn rust_ref_type(&self) -> &'static str {
        match *self {
            Ty::Double => "f64",
            Ty::Float => "f32",
            Ty::Int32 => "i32",
            Ty::Int64 => "i64",
            Ty::Uint32 => "u32",
            Ty::Uint64 => "u64",
            Ty::Sint32 => "i32",
            Ty::Sint64 => "i64",
            Ty::Fixed32 => "u32",
            Ty::Fixed64 => "u64",
            Ty::Sfixed32 => "i32",
            Ty::Sfixed64 => "i64",
            Ty::Bool => "bool",
            Ty::String => "&str",
            Ty::Bytes => "&[u8]",
            Ty::Enumeration(..) => "i32",
        }
    }

    pub fn encode_as(&self) -> &'static str {
        match *self {
            Ty::Enumeration(..) => "int32",
            _ => self.as_str(),
        }
    }

    /// Returns true if the scalar type is length delimited (i.e., `string` or `bytes`).
    pub fn is_numeric(&self) -> bool {
        *self != Ty::String && *self != Ty::Bytes
    }
}

impl fmt::Debug for Ty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
    Enumeration(String),
    Identifier(String),
}

impl DefaultValue {

    pub fn from_attr(attr: &MetaItem) -> Result<Option<Lit>, Error> {
        if attr.name() != "default" {
            return Ok(None);
        } else if let MetaItem::NameValue(_, ref lit) = *attr {
            Ok(Some(lit.clone()))
        } else {
            bail!("invalid default value attribute: {:?}", attr)
        }
    }

    pub fn from_lit(ty: &Ty, lit: Lit) -> Result<DefaultValue, Error> {
        let is_i32 = *ty == Ty::Int32 || *ty == Ty::Sint32 || *ty == Ty::Sfixed32;
        let is_i64 = *ty == Ty::Int64 || *ty == Ty::Sint64 || *ty == Ty::Sfixed64;

        let is_u32 = *ty == Ty::Uint32 || *ty == Ty::Fixed32;
        let is_u64 = *ty == Ty::Uint64 || *ty == Ty::Fixed64;

        let default = match lit {
            Lit::Int(value, IntTy::I32) | Lit::Int(value, IntTy::Unsuffixed) if is_i32 => DefaultValue::I32(value as _),
            Lit::Int(value, IntTy::I64) | Lit::Int(value, IntTy::Unsuffixed) if is_i64 => DefaultValue::I64(value as _),
            Lit::Int(value, IntTy::U32) | Lit::Int(value, IntTy::Unsuffixed) if is_u32 => DefaultValue::U32(value as _),
            Lit::Int(value, IntTy::U64) | Lit::Int(value, IntTy::Unsuffixed) if is_u64 => DefaultValue::U64(value),

            Lit::Float(ref value, FloatTy::F32) | Lit::Float(ref value, FloatTy::Unsuffixed) if *ty == Ty::Float => DefaultValue::F32(value.parse()?),
            Lit::Int(value, IntTy::Unsuffixed) if *ty == Ty::Float => DefaultValue::F32(value as _),

            Lit::Float(ref value, FloatTy::F64) | Lit::Float(ref value, FloatTy::Unsuffixed) if *ty == Ty::Double => DefaultValue::F64(value.parse()?),
            Lit::Int(value, IntTy::Unsuffixed) if *ty == Ty::Double => DefaultValue::F64(value as _),

            Lit::Bool(value) if *ty == Ty::Bool => DefaultValue::Bool(value),
            Lit::Str(ref value, _) if *ty == Ty::String => DefaultValue::String(value.clone()),
            Lit::ByteStr(ref value, _) if *ty == Ty::Bytes => DefaultValue::Bytes(value.clone()),

            Lit::Str(s, StrStyle::Cooked) => {
                let s = s.trim();
                if let Ty::Enumeration(ref ty) = *ty {
                    return Ok(DefaultValue::Enumeration(format!("{}::{}", ty, s)));
                }

                // Parse special floating point values.
                if *ty == Ty::Float {
                    match s {
                        "inf" => return Ok(DefaultValue::Identifier("::std::f32::INFINITY".to_owned())),
                        "-inf" => return Ok(DefaultValue::Identifier("::std::f32::NEG_INFINITY".to_owned())),
                        "nan" => return Ok(DefaultValue::Identifier("::std::f32::NAN".to_owned())),
                        _ => (),
                    }
                }
                if *ty == Ty::Double {
                    match s {
                        "inf" => return Ok(DefaultValue::Identifier("::std::f64::INFINITY".to_owned())),
                        "-inf" => return Ok(DefaultValue::Identifier("::std::f64::NEG_INFINITY".to_owned())),
                        "nan" => return Ok(DefaultValue::Identifier("::std::f64::NAN".to_owned())),
                        _ => (),
                    }
                }

                // Rust doesn't have a negative literals, so they have to be parsed specially.
                if s.chars().next() == Some('-') {
                    match syn::parse::lit(&s[1..]) {
                        syn::parse::IResult::Done(rest, _) if !rest.is_empty() => (),

                        syn::parse::IResult::Done(_, Lit::Int(value, IntTy::I32))
                        | syn::parse::IResult::Done(_, Lit::Int(value, IntTy::Unsuffixed)) if is_i32 => {
                            return Ok(DefaultValue::I32((!value + 1) as i32));
                        },

                        syn::parse::IResult::Done(_, Lit::Int(value, IntTy::I64))
                        | syn::parse::IResult::Done(_, Lit::Int(value, IntTy::Unsuffixed)) if is_i64 => {
                            return Ok(DefaultValue::I64((!value + 1) as i64));
                        },

                        syn::parse::IResult::Done(_, Lit::Float(ref value, FloatTy::F32))
                        | syn::parse::IResult::Done(_, Lit::Float(ref value, FloatTy::Unsuffixed)) if *ty == Ty::Float => {
                            return Ok(DefaultValue::F32(-value.parse()?));
                        },

                        syn::parse::IResult::Done(_, Lit::Float(ref value, FloatTy::F64))
                        | syn::parse::IResult::Done(_, Lit::Float(ref value, FloatTy::Unsuffixed)) if *ty == Ty::Double => {
                            return Ok(DefaultValue::F64(-value.parse()?));
                        },

                        syn::parse::IResult::Done(_, Lit::Int(value, IntTy::Unsuffixed)) if *ty == Ty::Float => {
                            return Ok(DefaultValue::F32(-(value as f32)));
                        },

                        syn::parse::IResult::Done(_, Lit::Int(value, IntTy::Unsuffixed)) if *ty == Ty::Double => {
                            return Ok(DefaultValue::F64(-(value as f64)));
                        },

                        _ => (),
                    }
                }
                match syn::parse::lit(&s) {
                    syn::parse::IResult::Done(rest, _) if !rest.is_empty() => (),
                    syn::parse::IResult::Done(_, Lit::Str(..)) => (),
                    syn::parse::IResult::Done(_, lit) => return DefaultValue::from_lit(ty, lit),
                    _ => (),
                }
                bail!("invalid default value: {}", quote!(#s));
            },
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
            Ty::Bytes => DefaultValue::Bytes(Vec::new()),
            Ty::Enumeration(ref ty) => return DefaultValue::Enumeration(format!("{}::default()", ty)),
        }
    }

    pub fn owned(&self) -> Tokens {
        match *self {
            DefaultValue::String(ref value) if value.is_empty() => quote!(::std::string::String::new()),
            DefaultValue::String(ref value) => quote!(#value.to_owned()),
            DefaultValue::Bytes(ref value) if value.is_empty() => quote!(::std::vec::Vec::new()),
            DefaultValue::Bytes(ref value) => {
                let lit = Lit::ByteStr(value.clone(), StrStyle::Cooked);
                quote!(#lit.to_owned())
            },

            ref other => other.typed(),
        }
    }

    pub fn typed(&self) -> Tokens {
        if let DefaultValue::Enumeration(_) = *self {
            quote!(super::#self as i32)
        } else {
            quote!(#self)
        }
    }
}

impl quote::ToTokens for DefaultValue {
    fn to_tokens(&self, tokens: &mut Tokens) {
        match *self {
            DefaultValue::F64(value) => value.to_tokens(tokens),
            DefaultValue::F32(value) => value.to_tokens(tokens),
            DefaultValue::I32(value) => value.to_tokens(tokens),
            DefaultValue::I64(value) => value.to_tokens(tokens),
            DefaultValue::U32(value) => value.to_tokens(tokens),
            DefaultValue::U64(value) => value.to_tokens(tokens),
            DefaultValue::Bool(value) => value.to_tokens(tokens),
            DefaultValue::String(ref value) => value.to_tokens(tokens),
            DefaultValue::Bytes(ref value) => Lit::ByteStr(value.clone(), StrStyle::Cooked).to_tokens(tokens),
            DefaultValue::Enumeration(ref value) => Ident::new(value.as_str()).to_tokens(tokens),
            DefaultValue::Identifier(ref value) => Ident::new(value.as_str()).to_tokens(tokens),
        }
    }
}
