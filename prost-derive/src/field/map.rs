use anyhow::{bail, ensure, Error};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Ident, Lit, Meta, MetaNameValue, NestedMeta, Type};

use crate::field::{scalar, set_option, tag_attr, MsgFns};
use crate::options::Options;

#[derive(Clone, Debug)]
pub enum MapTy {
    HashMap,
    BTreeMap,
}

impl MapTy {
    fn from_str(s: &str) -> Option<MapTy> {
        match s {
            "map" | "hash_map" => Some(MapTy::HashMap),
            "btree_map" => Some(MapTy::BTreeMap),
            _ => None,
        }
    }

    fn module(&self) -> Ident {
        match *self {
            MapTy::HashMap => Ident::new("hash_map", Span::call_site()),
            MapTy::BTreeMap => Ident::new("btree_map", Span::call_site()),
        }
    }
}

fn fake_scalar(ty: scalar::Ty) -> scalar::Field {
    let kind = scalar::Kind::Plain(scalar::DefaultValue::new(&ty));
    scalar::Field {
        field_ty: Type::Verbatim(ty.rust_type()),
        ty,
        kind,
        tag: 0, // Not used here
        clear: true,
        msg_fns: MsgFns::default(),
    }
}

#[derive(Clone)]
pub struct Field {
    pub field_ty: Type,
    pub map_ty: MapTy,
    pub key_ty: scalar::Ty,
    pub value_ty: ValueTy,
    pub tag: u32,
    pub msg_fns: MsgFns,
}

impl Field {
    pub fn new(
        field_ty: &Type,
        attrs: &[Meta],
        inferred_tag: Option<u32>,
        options: &Options,
    ) -> Result<Option<Field>, Error> {
        let mut types = None;
        let mut tag = None;
        let mut msg_fns = MsgFns::new();

        for attr in attrs {
            if let Some(t) = tag_attr(attr)? {
                set_option(&mut tag, t, "duplicate tag attributes")?;
            } else if msg_fns.attr(attr)?.is_some() {
                continue;
            } else if let Some(map_ty) = attr
                .path()
                .get_ident()
                .and_then(|i| MapTy::from_str(&i.to_string()))
            {
                let (k, v): (String, String) = match &*attr {
                    Meta::NameValue(MetaNameValue {
                        lit: Lit::Str(lit), ..
                    }) => {
                        let items = lit.value();
                        let mut items = items.split(',').map(ToString::to_string);
                        let k = items.next().unwrap();
                        let v = match items.next() {
                            Some(k) => k,
                            None => bail!("invalid map attribute: must have key and value types"),
                        };
                        if items.next().is_some() {
                            bail!("invalid map attribute: {:?}", attr);
                        }
                        (k, v)
                    }
                    Meta::List(meta_list) => {
                        // TODO(rustlang/rust#23121): slice pattern matching would make this much nicer.
                        if meta_list.nested.len() != 2 {
                            bail!("invalid map attribute: must contain key and value types");
                        }
                        let k = match &meta_list.nested[0] {
                            NestedMeta::Meta(Meta::Path(k)) if k.get_ident().is_some() => {
                                k.get_ident().unwrap().to_string()
                            }
                            _ => bail!("invalid map attribute: key must be an identifier"),
                        };
                        let v = match &meta_list.nested[1] {
                            NestedMeta::Meta(Meta::Path(v)) if v.get_ident().is_some() => {
                                v.get_ident().unwrap().to_string()
                            }
                            _ => bail!("invalid map attribute: value must be an identifier"),
                        };
                        (k, v)
                    }
                    _ => return Ok(None),
                };
                set_option(
                    &mut types,
                    (map_ty, key_ty_from_str(&k)?, ValueTy::from_str(&v)?),
                    "duplicate map type attribute",
                )?;
            } else {
                return Ok(None);
            }
        }

        msg_fns.check(false, options)?;

        Ok(match (types, tag.or(inferred_tag)) {
            (Some((map_ty, key_ty, value_ty)), Some(tag)) => {
                if let ValueTy::Scalar(scalar::Ty::Enumeration(..)) = value_ty {
                    ensure!(
                        msg_fns.is_empty(),
                        "map fields with enumerations as values cannot have as_msg, to_msg, from_msg, merge_msg, as_msgs or to_msgs attributes",
                    );
                }

                Some(Field {
                    field_ty: field_ty.clone(),
                    map_ty,
                    key_ty,
                    value_ty,
                    tag,
                    msg_fns,
                })
            }
            _ => None,
        })
    }

    pub fn new_oneof(attrs: &[Meta], options: &Options) -> Result<Option<Field>, Error> {
        if let Some(field) = Field::new(&Type::Verbatim(quote!()), attrs, None, options)? {
            ensure!(
                field.msg_fns.is_empty(),
                "oneof messages cannot use as_msg, to_msg, from_msg, merge_msg, as_msgs or to_msgs",
            );

            Ok(Some(field))
        } else {
            Ok(None)
        }
    }

    /// Returns a statement which encodes the map field.
    pub fn encode(&self, ident: TokenStream) -> TokenStream {
        let tag = self.tag;
        let key_mod = self.key_ty.module();
        let ke = quote!(::prost::encoding::#key_mod::encode);
        let kl = quote!(::prost::encoding::#key_mod::encoded_len);
        let module = self.map_ty.module();

        let encoding_mod = match &self.value_ty {
            ValueTy::Scalar(scalar::Ty::Enumeration(ty)) => {
                let default = quote!(#ty::default() as i32);
                return quote! {
                    ::prost::encoding::#module::encode_with_default(
                        #ke,
                        #kl,
                        ::prost::encoding::int32::encode,
                        ::prost::encoding::int32::encoded_len,
                        &(#default),
                        #tag,
                        &#ident,
                        buf,
                    );
                };
            }
            ValueTy::Scalar(value_ty) => {
                let val_mod = value_ty.module();
                quote!(::prost::encoding::#val_mod)
            }
            ValueTy::Message => quote!(::prost::encoding::message),
        };

        if self.msg_fns.is_empty() {
            quote! {
                ::prost::encoding::#module::encode(
                    #ke,
                    #kl,
                    #encoding_mod::encode,
                    #encoding_mod::encoded_len,
                    #tag,
                    &#ident,
                    buf,
                );
            }
        } else {
            let get = self.msg_fns.get(&quote!(value));
            quote! {
                ::prost::encoding::#module::encode(
                    #ke,
                    #kl,
                    |tag, value, buf| #encoding_mod::encode(tag, #get, buf),
                    |tag, value| #encoding_mod::encoded_len(tag, #get),
                    #tag,
                    &#ident,
                    buf,
                );
            }
        }
    }

    /// Returns an expression which evaluates to the result of merging a decoded key value pair
    /// into the map.
    pub fn merge(&self, ident: TokenStream) -> TokenStream {
        let key_mod = self.key_ty.module();
        let km = quote!(::prost::encoding::#key_mod::merge);
        let module = self.map_ty.module();

        let encoding_mod = match &self.value_ty {
            ValueTy::Scalar(scalar::Ty::Enumeration(ty)) => {
                let default = quote!(#ty::default() as i32);
                return quote! {
                    ::prost::encoding::#module::merge_with_default(
                        #km,
                        ::prost::encoding::int32::merge,
                        #default,
                        &mut #ident,
                        buf,
                        ctx,
                    )
                };
            }
            ValueTy::Scalar(value_ty) => {
                let val_mod = value_ty.module();
                quote!(::prost::encoding::#val_mod)
            }
            ValueTy::Message => quote!(::prost::encoding::message),
        };

        let set = self.msg_fns.set(&quote!(value), quote!(msg));
        if let Some(set) = set {
            quote! {
                ::prost::encoding::#module::merge(
                    #km,
                    |wire_type, value, buf, ctx| {
                        let mut msg = Default::default();
                        #encoding_mod::merge(wire_type, &mut msg, buf, ctx).map(|_| #set)
                    },
                    &mut #ident,
                    buf,
                    ctx,
                )
            }
        } else {
            quote! {
                ::prost::encoding::#module::merge(
                    #km,
                    #encoding_mod::merge,
                    &mut #ident,
                    buf,
                    ctx,
                )
            }
        }
    }

    /// Returns an expression which evaluates to the encoded length of the map.
    pub fn encoded_len(&self, ident: TokenStream) -> TokenStream {
        let tag = self.tag;
        let key_mod = self.key_ty.module();
        let kl = quote!(::prost::encoding::#key_mod::encoded_len);
        let module = self.map_ty.module();
        let encoding_mod = match &self.value_ty {
            ValueTy::Scalar(scalar::Ty::Enumeration(ty)) => {
                let default = quote!(#ty::default() as i32);
                return quote! {
                    ::prost::encoding::#module::encoded_len_with_default(
                        #kl,
                        ::prost::encoding::int32::encoded_len,
                        &(#default),
                        #tag,
                        &#ident,
                    )
                };
            }
            ValueTy::Scalar(value_ty) => {
                let val_mod = value_ty.module();
                quote!(::prost::encoding::#val_mod)
            }
            ValueTy::Message => quote!(::prost::encoding::message),
        };

        if self.msg_fns.is_empty() {
            quote! {
                ::prost::encoding::#module::encoded_len(
                    #kl,
                    #encoding_mod::encoded_len,
                    #tag,
                    &#ident,
                )
            }
        } else {
            let get = self.msg_fns.get(&quote!(value));
            quote! {
                ::prost::encoding::#module::encoded_len(
                    #kl,
                    |tag, value| #encoding_mod::encoded_len(tag, #get),
                    #tag,
                    &#ident,
                )
            }
        }
    }

    pub fn clear(&self, ident: TokenStream) -> TokenStream {
        quote!(#ident.clear())
    }

    /// Returns methods to embed in the message.
    pub fn methods(&self, ident: &Ident) -> Option<TokenStream> {
        if let ValueTy::Scalar(scalar::Ty::Enumeration(ty)) = &self.value_ty {
            let key_ty = self.key_ty.rust_type();
            let key_ref_ty = self.key_ty.rust_ref_type();

            let get = Ident::new(&format!("get_{}", ident), Span::call_site());
            let insert = Ident::new(&format!("insert_{}", ident), Span::call_site());
            let take_ref = if self.key_ty.is_numeric() {
                quote!(&)
            } else {
                quote!()
            };

            let get_doc = format!(
                "Returns the enum value for the corresponding key in `{}`, \
                 or `None` if the entry does not exist or it is not a valid enum value.",
                ident,
            );
            let insert_doc = format!("Inserts a key value pair into `{}`.", ident);
            Some(quote! {
                #[doc=#get_doc]
                pub fn #get(&self, key: #key_ref_ty) -> ::core::option::Option<#ty> {
                    self.#ident.get(#take_ref key).cloned().and_then(#ty::from_i32)
                }
                #[doc=#insert_doc]
                pub fn #insert(&mut self, key: #key_ty, value: #ty) -> ::core::option::Option<#ty> {
                    self.#ident.insert(key, value as i32).and_then(#ty::from_i32)
                }
            })
        } else {
            None
        }
    }

    /// Returns a newtype wrapper around the map, implementing nicer Debug
    ///
    /// The Debug tries to convert any enumerations met into the variants if possible, instead of
    /// outputting the raw numbers.
    pub fn debug(&self, wrapper_name: TokenStream) -> TokenStream {
        // A fake field for generating the debug wrapper
        let key_wrapper = fake_scalar(self.key_ty.clone()).debug(quote!(KeyWrapper));
        let field_ty = &self.field_ty;

        match &self.value_ty {
            ValueTy::Scalar(ty) => {
                let value_wrapper = fake_scalar(ty.clone()).debug(quote!(ValueWrapper));
                let get = if self.msg_fns.as_to_msg() {
                    self.msg_fns.get(&quote!(v))
                } else {
                    quote!(v)
                };

                quote! {
                    struct #wrapper_name<'a>(&'a #field_ty);
                    impl<'a> ::core::fmt::Debug for #wrapper_name<'a> {
                        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                            #key_wrapper
                            #value_wrapper
                            let mut builder = f.debug_map();
                            for (k, v) in self.0 {
                                builder.entry(&KeyWrapper(k), &ValueWrapper(#get));
                            }
                            builder.finish()
                        }
                    }
                }
            }
            ValueTy::Message => {
                let get = self.msg_fns.get(&quote!(v));
                quote! {
                    struct #wrapper_name<'a>(&'a #field_ty);
                    impl<'a> ::core::fmt::Debug for #wrapper_name<'a> {
                        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                            #key_wrapper
                            let mut builder = f.debug_map();
                            for (k, v) in self.0 {
                                builder.entry(&KeyWrapper(k), #get);
                            }
                            builder.finish()
                        }
                    }
                }
            }
        }
    }
}

fn key_ty_from_str(s: &str) -> Result<scalar::Ty, Error> {
    let ty = scalar::Ty::from_str(s)?;
    match ty {
        scalar::Ty::Int32
        | scalar::Ty::Int64
        | scalar::Ty::Uint32
        | scalar::Ty::Uint64
        | scalar::Ty::Sint32
        | scalar::Ty::Sint64
        | scalar::Ty::Fixed32
        | scalar::Ty::Fixed64
        | scalar::Ty::Sfixed32
        | scalar::Ty::Sfixed64
        | scalar::Ty::Bool
        | scalar::Ty::String => Ok(ty),
        _ => bail!("invalid map key type: {}", s),
    }
}

/// A map value type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueTy {
    Scalar(scalar::Ty),
    Message,
}

impl ValueTy {
    fn from_str(s: &str) -> Result<ValueTy, Error> {
        if let Ok(ty) = scalar::Ty::from_str(s) {
            Ok(ValueTy::Scalar(ty))
        } else if s.trim() == "message" {
            Ok(ValueTy::Message)
        } else {
            bail!("invalid map value type: {}", s);
        }
    }
}
