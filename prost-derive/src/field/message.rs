use anyhow::{bail, ensure, Error};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Meta, Type};

use crate::field::{
    as_msg_attr, as_msgs_attr, from_msg_attr, merge_msg_attr, set_bool, set_option, tag_attr,
    to_msg_attr, to_msgs_attr, word_attr, Label,
};

#[derive(Clone)]
pub struct Field {
    pub field_ty: Type,
    pub label: Label,
    pub tag: u32,
    pub as_msg: Option<TokenStream>,
    pub as_msgs: Option<TokenStream>,
    pub to_msg: Option<TokenStream>,
    pub to_msgs: Option<TokenStream>,
    pub from_msg: Option<TokenStream>,
    pub merge_msg: Option<TokenStream>,
}

impl Field {
    pub fn new(
        field_ty: &Type,
        attrs: &[Meta],
        inferred_tag: Option<u32>,
    ) -> Result<Option<Field>, Error> {
        let mut message = false;
        let mut label = None;
        let mut tag = None;
        let mut as_msg = None;
        let mut as_msgs = None;
        let mut to_msg = None;
        let mut to_msgs = None;
        let mut from_msg = None;
        let mut merge_msg = None;
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
            } else if let Some(a) = as_msg_attr(attr)? {
                set_option(&mut as_msg, a, "duplicate as_msg attributes")?;
            } else if let Some(a) = as_msgs_attr(attr)? {
                set_option(&mut as_msgs, a, "duplicate as_msgs attributes")?;
            } else if let Some(t) = to_msg_attr(attr)? {
                set_option(&mut to_msg, t, "duplicate to_msg attributes")?;
            } else if let Some(t) = to_msgs_attr(attr)? {
                set_option(&mut to_msgs, t, "duplicate to_msgs attributes")?;
            } else if let Some(f) = from_msg_attr(attr)? {
                set_option(&mut from_msg, f, "duplicate from_msg attributes")?;
            } else if let Some(m) = merge_msg_attr(attr)? {
                set_option(&mut merge_msg, m, "duplicate merge_msg attributes")?;
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
            let converting = as_msg.is_some() || as_msgs.is_some()
                || to_msg.is_some() || to_msgs.is_some()
                || from_msg.is_some() || merge_msg.is_some();

            ensure!(
                !converting
                    || as_msg.is_some() || as_msgs.is_some()
                    || to_msg.is_some() || to_msgs.is_some(),
                "missing as_msg, as_msgs, to_msg, or to_msgs attribute",
            );

            ensure!(
                (as_msgs.is_none() && to_msgs.is_none()) || (as_msg.is_none() && to_msg.is_none()),
                "cannot use as_msg/to_msg and as_msgs/to_msgs at the same time",
            );

            ensure!(
                !converting || (as_msgs.is_none() && to_msgs.is_none()) || merge_msg.is_some(),
                "missing merge_msg attribute",
            );

            ensure!(
                !converting || from_msg.is_some() || merge_msg.is_some(),
                "missing from_msg, or merge_msg attribute",
            );
        } else {
            ensure!(
                as_msgs.is_none() && to_msgs.is_none(),
                "as_msgs and to_msgs attributes are only supported for repeated fields",
            );

            let converting = as_msg.is_some() || to_msg.is_some()
                || from_msg.is_some() || merge_msg.is_some();

            ensure!(
                !converting || as_msg.is_some() || to_msg.is_some(),
                "missing as_msg or to_msg attribute",
            );

            ensure!(
                !converting || from_msg.is_some() || merge_msg.is_some(),
                "missing from_msg or merge_msg attribute",
            );
        }

        Ok(Some(Field {
            field_ty: field_ty.clone(),
            label: label.unwrap_or(Label::Optional),
            tag,
            as_msg,
            as_msgs,
            to_msg,
            to_msgs,
            from_msg,
            merge_msg,
        }))
    }

    pub fn new_oneof(attrs: &[Meta]) -> Result<Option<Field>, Error> {
        if let Some(mut field) = Field::new(&Type::Verbatim(quote!()), attrs, None)? {
            ensure!(
                field.as_msg.is_none()
                    && field.to_msg.is_none()
                    && field.from_msg.is_none()
                    && field.merge_msg.is_none(),
                "oneof messages cannot have as_msg, to_msg, from_msg, or merge_msg attributes",
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
            Label::Optional => {
                let msg = match (&self.as_msg, &self.to_msg) {
                    (Some(as_msg), _) => quote!(#as_msg(&#ident)),
                    (None, Some(to_msg)) => quote!(#to_msg(&#ident).as_ref()),
                    (None, None) => quote!(#ident.as_ref()),
                };

                quote! {
                    if let ::core::option::Option::Some(value) = #msg {
                        ::prost::encoding::message::encode(#tag, value, buf);
                    }
                }
            }
            Label::Required => {
                let msg = match (&self.as_msg, &self.to_msg) {
                    (Some(as_msg), _) => quote!(#as_msg(&#ident)),
                    (None, Some(to_msg)) => quote!(&#to_msg(&#ident)),
                    (None, None) => quote!(&#ident),
                };

                quote! {
                    ::prost::encoding::message::encode(#tag, #msg, buf);
                }
            }
            Label::Repeated => match (&self.as_msgs, &self.to_msgs) {
                (Some(msgs_fn), _) | (None, Some(msgs_fn)) => quote! {
                    #msgs_fn(&#ident).iter().for_each(|value| {
                        ::prost::encoding::message::encode(#tag, value, buf)
                    });
                },
                (None, None) => {
                    let msg = match (&self.as_msg, &self.to_msg) {
                        (Some(as_msg), _) => quote!(#as_msg(value)),
                        (None, Some(to_msg)) => quote!(&#to_msg(value)),
                        (None, None) => quote!(value),
                    };

                    quote! {
                        #ident.iter().for_each(|value| {
                            ::prost::encoding::message::encode(#tag, #msg, buf);
                        });
                    }
                }
            },
        }
    }

    pub fn merge(&self, ident: TokenStream) -> TokenStream {
        match self.label {
            Label::Optional => match (&self.from_msg, &self.merge_msg) {
                (_, Some(merge_msg)) => quote! {{
                    let mut msg = Default::default();
                    ::prost::encoding::message::merge(wire_type, &mut msg, buf, ctx)
                        .map(|_| #merge_msg(#ident, Some(msg)))
                }},
                (Some(from_msg), None) => quote! {{
                    let mut msg = Default::default();
                    ::prost::encoding::message::merge(wire_type, &mut msg, buf, ctx)
                        .map(|_| *#ident = #from_msg(Some(msg)))
                }},
                (None, None) => quote! {
                    ::prost::encoding::message::merge(
                        wire_type,
                        #ident.get_or_insert_with(Default::default),
                        buf,
                        ctx,
                    )
                },
            },
            Label::Required => match (&self.from_msg, &self.merge_msg) {
                (_, Some(merge_msg)) => quote! {{
                    let mut msg = Default::default();
                    ::prost::encoding::message::merge(wire_type, &mut msg, buf, ctx)
                        .map(|_| #merge_msg(#ident, msg))
                }},
                (Some(from_msg), None) => quote! {{
                    let mut msg = Default::default();
                    ::prost::encoding::message::merge(wire_type, &mut msg, buf, ctx)
                        .map(|_| *#ident = #from_msg(msg))
                }},
                (None, None) => quote! {
                    ::prost::encoding::message::merge(wire_type, #ident, buf, ctx)
                },
            },
            Label::Repeated => match (&self.from_msg, &self.merge_msg) {
                (_, Some(merge_msg)) if (
                    self.as_msgs.is_some() || self.to_msgs.is_some()
                ) => quote! {{
                    let mut msg = Default::default();
                    ::prost::encoding::message::merge(wire_type, &mut msg, buf, ctx).map(|_| {
                        #merge_msg(#ident, msg);
                    })
                }},
                (Some(from_msg), _) => quote! {{
                    let mut msg = Default::default();
                    ::prost::encoding::message::merge(wire_type, &mut msg, buf, ctx).map(|_| {
                        #ident.push(#from_msg(msg))
                    })
                }},
                (None, Some(merge_msg)) => quote! {{
                    let mut msg = Default::default();
                    ::prost::encoding::message::merge(wire_type, &mut msg, buf, ctx).map(|_| {
                        let mut val = Default::default();
                        #merge_msg(&mut val, msg);
                        #ident.push(val);
                    })
                }},
                (None, None) => quote! {{
                    ::prost::encoding::message::merge_repeated(wire_type, #ident, buf, ctx)
                }}
            },
        }
    }

    pub fn encoded_len(&self, ident: TokenStream) -> TokenStream {
        let tag = self.tag;

        match self.label {
            Label::Optional => {
                let msg = match (&self.as_msg, &self.to_msg) {
                    (Some(as_msg), _) => quote!(#as_msg(&#ident)),
                    (None, Some(to_msg)) => quote!(#to_msg(&#ident).as_ref()),
                    (None, None) => quote!(#ident.as_ref()),
                };

                quote! {
                    #msg.map_or(0, |value| ::prost::encoding::message::encoded_len(#tag, value))
                }
            }
            Label::Required => {
                let msg = match (&self.as_msg, &self.to_msg) {
                    (Some(as_msg), _) => quote!(#as_msg(&#ident)),
                    (None, Some(to_msg)) => quote!(&#to_msg(&#ident)),
                    (None, None) => quote!(&#ident),
                };

                quote! {
                    ::prost::encoding::message::encoded_len(#tag, #msg)
                }
            }
            Label::Repeated => match (&self.as_msgs, &self.to_msgs) {
                (Some(msgs_fn), _) | (None, Some(msgs_fn)) => quote! {
                    #msgs_fn(&#ident).iter().map(|value| {
                        ::prost::encoding::message::encoded_len(#tag, value)
                    }).sum::<usize>()
                },
                (None, None) => {
                    let msg = match (&self.as_msg, &self.to_msg) {
                        (Some(as_msg), _) => quote!(#as_msg(value)),
                        (None, Some(to_msg)) => quote!(&#to_msg(value)),
                        (None, None) => quote!(value),
                    };

                    quote! {
                        #ident.iter().map(|value| {
                            ::prost::encoding::message::encoded_len(#tag, #msg)
                        }).sum::<usize>()
                    }
                }
            },
        }
    }

    pub fn clear(&self, ident: TokenStream) -> TokenStream {
        match self.label {
            Label::Optional => match (&self.from_msg, &self.merge_msg) {
                (_, Some(merge_msg)) => quote! {
                    #merge_msg(&mut #ident, ::core::option::Option::None)
                },
                (Some(from_msg), None) => quote! {
                    #ident = #from_msg(::core::option::Option::None)
                },
                (None, None) => quote! {
                    #ident = ::core::option::Option::None
                },
            },
            Label::Required => match (&self.from_msg, &self.merge_msg) {
                (_, Some(merge_msg)) => quote!(#merge_msg(&mut #ident, Default::default())),
                (Some(from_msg), None) => quote!(#ident = #from_msg(Default::default())),
                (None, None) => quote!(#ident.clear()),
            },
            Label::Repeated if self.as_msgs.is_some() || self.to_msgs.is_some() => quote! {
                #ident = Default::default()
            },
            Label::Repeated => quote!(#ident.clear()),
        }
    }

    pub fn debug(&self, ident: TokenStream) -> TokenStream {
        match self.label {
            Label::Optional | Label::Required => match (&self.as_msg, &self.to_msg) {
                (Some(msg_fn), _) | (None, Some(msg_fn)) => quote!(&#msg_fn(&#ident)),
                (None, None) => quote!(&#ident),
            }
            Label::Repeated => match (&self.as_msgs, &self.to_msgs, &self.as_msg, &self.to_msg) {
                (Some(msgs_fn), _, _, _) | (None, Some(msgs_fn), _, _) => quote!(#msgs_fn(&#ident)),
                (None, None, Some(msg_fn), _) | (None, None, None, Some(msg_fn)) => {
                    let field_ty = &self.field_ty;
                    quote! {{
                        struct RepeatedWrapper<'a>(&'a #field_ty);
                        impl<'a> ::core::fmt::Debug for RepeatedWrapper<'a> {
                            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                                let mut vec_builder = f.debug_list();
                                for v in self.0 {
                                    vec_builder.entry(&#msg_fn(v));
                                }
                                vec_builder.finish()
                            }
                        }
                        RepeatedWrapper(&#ident)
                    }}
                }
                (None, None, None, None) => quote!(&#ident),
            }
        }
    }
}
