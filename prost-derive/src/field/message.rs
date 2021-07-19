use anyhow::{bail, ensure, Error};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Meta, Type};

use crate::field::{bool_attr, set_bool, set_option, tag_attr, word_attr, Label, MsgFns};
use crate::options::Options;

#[derive(Clone)]
pub struct Field {
    pub field_ty: Type,
    pub label: Label,
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
        let mut message = false;
        let mut label = None;
        let mut tag = None;
        let mut clear = None;
        let mut msg_fns = MsgFns::new();
        let mut boxed = false;

        let mut unknown_attrs = Vec::new();

        for attr in attrs {
            if word_attr("message", attr) {
                set_bool(&mut message, "duplicate message attribute")?;
            } else if word_attr("boxed", attr) {
                set_bool(&mut boxed, "duplicate boxed attribute")?;
            } else if let Some(t) = tag_attr(attr)? {
                set_option(&mut tag, t, "duplicate tag attributes")?;
            } else if let Some(l) = Label::from_attr(attr) {
                set_option(&mut label, l, "duplicate label attributes")?;
            } else if let Some(c) = bool_attr("clear", &attr)? {
                set_option(&mut clear, c, "duplicate clear attributes")?;
            } else if msg_fns.attr(attr)?.is_some() {
                continue;
            } else {
                unknown_attrs.push(attr);
            }
        }

        if !message {
            return Ok(None);
        }

        match unknown_attrs.len() {
            0 => (),
            1 => bail!(
                "unknown attribute for message field: {:?}",
                unknown_attrs[0]
            ),
            _ => bail!("unknown attributes for message field: {:?}", unknown_attrs),
        }

        let tag = match tag.or(inferred_tag) {
            Some(tag) => tag,
            None => bail!("message field is missing a tag attribute"),
        };

        if let Some(Label::Repeated) = label {
            msg_fns.check(true, options)?;
        } else {
            msg_fns.check(false, options)?;
        }

        let field_ty = field_ty.clone();
        let label = if options.proto.is_proto3() {
            label.unwrap_or(Label::Required)
        } else {
            label.unwrap_or(Label::Optional)
        };

        Ok(Some(Field {
            field_ty,
            label,
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

            if let Some(attr) = attrs.iter().find(|attr| Label::from_attr(attr).is_some()) {
                bail!(
                    "invalid attribute for oneof field: {}",
                    attr.path().into_token_stream()
                );
            }
            field.label = Label::Required;
            Ok(Some(field))
        } else {
            Ok(None)
        }
    }

    pub fn encode(&self, ident: TokenStream) -> TokenStream {
        let tag = self.tag;

        match self.label {
            Label::Optional => self.msg_fns.map_as_ref(
                &ident,
                quote! {
                    if let ::core::option::Option::Some(value) = value {
                        ::prost::encoding::message::encode(#tag, value, buf);
                    }
                },
            ),
            Label::Required => self.msg_fns.map(
                &ident,
                quote! {
                    ::prost::encoding::message::encode(#tag, value, buf);
                },
            ),
            Label::Repeated => self.msg_fns.for_each(
                &ident,
                quote! {
                    ::prost::encoding::message::encode(#tag, value, buf);
                },
            ),
        }
    }

    pub fn merge(&self, ident: TokenStream) -> TokenStream {
        match self.label {
            Label::Optional => {
                let set = self.msg_fns.set(&ident, quote!(Some(msg)));
                if let Some(set) = set {
                    quote! {{
                        let mut msg = Default::default();
                        ::prost::encoding::message::merge(wire_type, &mut msg, buf, ctx)
                            .map(|_| #set)
                    }}
                } else {
                    quote! {
                        ::prost::encoding::message::merge(
                            wire_type,
                            #ident.get_or_insert_with(Default::default),
                            buf,
                            ctx,
                        )
                    }
                }
            }
            Label::Required => {
                let set = self.msg_fns.set(&ident, quote!(msg));
                if let Some(set) = set {
                    quote! {{
                        let mut msg = Default::default();
                        ::prost::encoding::message::merge(wire_type, &mut msg, buf, ctx)
                            .map(|_| #set)
                    }}
                } else {
                    quote! {
                        ::prost::encoding::message::merge(wire_type, #ident, buf, ctx)
                    }
                }
            }
            Label::Repeated => {
                let push = self.msg_fns.push(&ident, quote!(msg));
                if let Some(push) = push {
                    quote! {{
                        let mut msg = Default::default();
                        ::prost::encoding::message::merge(wire_type, &mut msg, buf, ctx)
                            .map(|_| #push)
                    }}
                } else {
                    quote! {
                        ::prost::encoding::message::merge_repeated(wire_type, #ident, buf, ctx)
                    }
                }
            }
        }
    }

    pub fn encoded_len(&self, ident: TokenStream) -> TokenStream {
        let tag = self.tag;

        match self.label {
            Label::Optional => self.msg_fns.map_as_ref(
                &ident,
                quote! {
                    value.map_or(0, |value| ::prost::encoding::message::encoded_len(#tag, value))
                },
            ),
            Label::Required => self.msg_fns.map(
                &ident,
                quote! {
                    ::prost::encoding::message::encoded_len(#tag, value)
                },
            ),
            Label::Repeated => {
                let iter_map = self.msg_fns.iter_map(
                    &ident,
                    quote! {
                        ::prost::encoding::message::encoded_len(#tag, value)
                    },
                );

                quote! {
                    #iter_map.sum::<usize>()
                }
            }
        }
    }

    pub fn clear(&self, ident: TokenStream) -> TokenStream {
        if !self.clear {
            return quote!();
        }

        match self.label {
            Label::Optional => self
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
            Label::Required => self
                .msg_fns
                .set(
                    &quote!(&mut #ident),
                    quote! {
                        Default::default()
                    },
                )
                .unwrap_or_else(|| {
                    quote! {
                        #ident = Default::default()
                    }
                }),
            Label::Repeated if self.msg_fns.as_to_msgs() => self
                .msg_fns
                .set(
                    &quote!(&mut #ident),
                    quote! {
                        Default::default()
                    },
                )
                .unwrap_or_else(|| {
                    quote! {
                        #ident = Default::default()
                    }
                }),
            Label::Repeated => quote! {
                #ident.clear()
            },
        }
    }

    pub fn debug(&self, ident: TokenStream) -> TokenStream {
        match self.label {
            Label::Optional | Label::Required => self.msg_fns.get(&ident),
            Label::Repeated => {
                let field_ty = &self.field_ty;
                let for_each = self.msg_fns.for_each(
                    &quote!(self.0),
                    quote! {
                        vec_builder.entry(value);
                    },
                );

                quote! {{
                    struct RepeatedWrapper<'a>(&'a #field_ty);
                    impl<'a> ::core::fmt::Debug for RepeatedWrapper<'a> {
                        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                            let mut vec_builder = f.debug_list();
                            #for_each
                            vec_builder.finish()
                        }
                    }
                }}
            }
        }
    }
}
