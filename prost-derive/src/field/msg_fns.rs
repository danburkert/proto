use anyhow::{bail, ensure, Error};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Lit, Meta, MetaNameValue};

use crate::field::set_option;
use crate::options::Options;

#[derive(Clone, Default)]
pub struct MsgFns {
    pub as_msg: Option<TokenStream>,
    pub to_msg: Option<TokenStream>,
    pub from_msg: Option<TokenStream>,
    pub merge_msg: Option<TokenStream>,

    pub as_msgs: Option<TokenStream>,
    pub to_msgs: Option<TokenStream>,
}

impl MsgFns {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn attr(&mut self, attr: &Meta) -> Result<Option<()>, Error> {
        if let Some(a) = as_msg_attr(attr)? {
            set_option(&mut self.as_msg, a, "duplicate as_msg attributes")?;
        } else if let Some(t) = to_msg_attr(attr)? {
            set_option(&mut self.to_msg, t, "duplicate to_msg attributes")?;
        } else if let Some(f) = from_msg_attr(attr)? {
            set_option(&mut self.from_msg, f, "duplicate from_msg attributes")?;
        } else if let Some(m) = merge_msg_attr(attr)? {
            set_option(&mut self.merge_msg, m, "duplicate merge_msg attributes")?;
        } else if let Some(a) = as_msgs_attr(attr)? {
            set_option(&mut self.as_msgs, a, "duplicate as_msgs attributes")?;
        } else if let Some(t) = to_msgs_attr(attr)? {
            set_option(&mut self.to_msgs, t, "duplicate to_msgs attributes")?;
        } else {
            return Ok(None);
        }

        Ok(Some(()))
    }

    pub fn check(&self, repeated: bool, options: &Options) -> Result<(), Error> {
        if self.is_empty() {
            return Ok(());
        }

        let as_to_msg = self.as_to_msg();
        let from_merge_msg = self.from_merge_msg();
        let as_to_msgs = self.as_to_msgs();

        ensure!(
            !as_to_msg || !as_to_msgs,
            "cannot use as_msgs/to_msgs and as_msg/to_msg at the same time",
        );

        ensure!(
            !as_to_msg || !options.merge || from_merge_msg,
            "missing from_msg or merge_msg attribute",
        );

        if repeated {
            ensure!(
                !from_merge_msg || as_to_msg || as_to_msgs,
                "missing as_msg, to_msg, as_msgs or to_msgs attribute",
            );

            ensure!(
                !as_to_msgs || !options.merge || self.merge_msg.is_some(),
                "missing merge_msg attribute",
            );

            ensure!(
                !as_to_msgs || self.from_msg.is_none(),
                "cannot use from_msg when as_msgs or to_msgs is set",
            );
        } else {
            ensure!(
                !as_to_msgs,
                "cannot use as_msgs or to_msgs on a field that is not repeated or packed",
            );

            ensure!(
                !from_merge_msg || as_to_msg,
                "missing as_msg or to_msg attribute",
            );
        }

        Ok(())
    }

    pub fn as_to_msg(&self) -> bool {
        self.as_msg.is_some() || self.to_msg.is_some()
    }

    pub fn from_merge_msg(&self) -> bool {
        self.from_msg.is_some() || self.merge_msg.is_some()
    }

    pub fn as_to_msgs(&self) -> bool {
        self.as_msgs.is_some() || self.to_msgs.is_some()
    }

    pub fn is_empty(&self) -> bool {
        !self.as_to_msg() && !self.from_merge_msg() && !self.as_to_msgs()
    }

    pub fn get(&self, ident: &TokenStream) -> TokenStream {
        if let Some(ref as_msg) = self.as_msg {
            quote! {
                (#as_msg)(&#ident)
            }
        } else if let Some(ref to_msg) = self.to_msg {
            quote! {
                &(#to_msg)(&#ident)
            }
        } else {
            quote! {
                &#ident
            }
        }
    }

    pub fn get_slice(&self, ident: &TokenStream) -> TokenStream {
        if let Some(ref as_msgs) = self.as_msgs {
            quote! {
                (#as_msgs)(&#ident).as_ref()
            }
        } else if let Some(ref to_msgs) = self.to_msgs {
            quote! {
                (#to_msgs)(&#ident).as_ref()
            }
        } else if let Some(ref to_msg) = self.to_msg {
            quote! {
                #ident
                    .iter()
                    .map(#to_msg)
                    .collect::<::prost::alloc::vec::Vec<_>>()
                    .as_ref()
            }
        } else if let Some(ref as_msg) = self.as_msg {
            quote! {
                #ident
                    .iter()
                    .map(#as_msg)
                    .cloned()
                    .collect::<::prost::alloc::vec::Vec<_>>()
                    .as_ref()
            }
        } else {
            quote! {
                #ident.as_ref()
            }
        }
    }

    pub fn map(&self, ident: &TokenStream, map: TokenStream) -> TokenStream {
        let get = self.get(ident);
        quote! {{
            let value = #get;
            #map
        }}
    }

    pub fn map_as_ref(&self, ident: &TokenStream, map: TokenStream) -> TokenStream {
        if let Some(ref as_msg) = self.as_msg {
            quote! {{
                let value = (#as_msg)(&#ident);
                #map
            }}
        } else if let Some(ref to_msg) = self.to_msg {
            quote! {{
                let value = (#to_msg)(&#ident);
                let value = value.as_ref();
                #map
            }}
        } else {
            quote! {{
                let value = #ident.as_ref();
                #map
            }}
        }
    }

    pub fn iter_map(&self, ident: &TokenStream, map: TokenStream) -> TokenStream {
        if let Some(ref as_msgs) = self.as_msgs {
            quote! {
                (#as_msgs)(&#ident).into_iter().map(|value| {
                    #map
                })
            }
        } else if let Some(ref to_msgs) = self.to_msgs {
            quote! {
                (#to_msgs)(&#ident).into_iter().map(|value| {
                    let value = &value;
                    #map
                })
            }
        } else if let Some(ref as_msg) = self.as_msg {
            quote! {
                #ident.iter().map(#as_msg).map(|value| {
                    #map
                })
            }
        } else if let Some(ref to_msg) = self.to_msg {
            quote! {
                #ident.iter().map(#to_msg).map(|value| {
                    let value = &value;
                    #map
                })
            }
        } else {
            quote! {
                #ident.iter().map(|value| {
                    #map
                })
            }
        }
    }

    pub fn for_each(&self, ident: &TokenStream, for_each: TokenStream) -> TokenStream {
        if let Some(ref as_msgs) = self.as_msgs {
            quote! {
                (#as_msgs)(&#ident).into_iter().for_each(|value| {
                    #for_each
                });
            }
        } else if let Some(ref to_msgs) = self.to_msgs {
            quote! {
                (#to_msgs)(&#ident).into_iter().for_each(|value| {
                    let value = &value;
                    #for_each
                });
            }
        } else if let Some(ref as_msg) = self.as_msg {
            quote! {
                #ident.iter().map(#as_msg).for_each(|value| {
                    #for_each
                });
            }
        } else if let Some(ref to_msg) = self.to_msg {
            quote! {
                #ident.iter().map(#to_msg).for_each(|value| {
                    let value = &value;
                    #for_each
                });
            }
        } else {
            quote! {
                #ident.iter().for_each(|value| {
                    #for_each
                });
            }
        }
    }

    pub fn set(&self, ident: &TokenStream, value: TokenStream) -> Option<TokenStream> {
        if let Some(ref merge_msg) = self.merge_msg {
            Some(quote! {
                (#merge_msg)(#ident, #value)
            })
        } else if let Some(ref from_msg) = self.from_msg {
            Some(quote! {
                *#ident = (#from_msg)(#value)
            })
        } else {
            None
        }
    }

    pub fn from(&self, value: TokenStream) -> TokenStream {
        if let Some(ref from_msg) = self.from_msg {
            quote! {
                (#from_msg)(#value)
            }
        } else if let Some(ref merge_msg) = self.merge_msg {
            quote! {{
                let mut val = Default::default();
                (#merge_msg)(&mut val, #value);
                val
            }}
        } else {
            value
        }
    }

    pub fn push(&self, ident: &TokenStream, value: TokenStream) -> Option<TokenStream> {
        if self.as_msgs.is_some() || self.to_msgs.is_some() {
            if let Some(ref merge_msg) = self.merge_msg {
                Some(quote! {
                    (#merge_msg)(#ident, #value)
                })
            } else {
                None
            }
        } else {
            if let Some(ref from_msg) = self.from_msg {
                Some(quote! {
                    #ident.push((#from_msg)(#value))
                })
            } else if let Some(ref merge_msg) = self.merge_msg {
                Some(quote! {{
                    let mut val = Default::default();
                    (#merge_msg)(&mut val, #value);
                    #ident.push(val);
                }})
            } else {
                None
            }
        }
    }
}

macro_rules! msg_fn_attr {
    ($fn:ident, $attr:literal) => {
        fn $fn(attr: &Meta) -> Result<Option<TokenStream>, Error> {
            if !attr.path().is_ident($attr) {
                return Ok(None);
            }

            match *attr {
                Meta::NameValue(MetaNameValue {
                    lit: Lit::Str(ref lit),
                    ..
                }) => Ok(Some(lit.parse()?)),
                _ => bail!("invalid {} attribute: {:?}", $attr, attr),
            }
        }
    };
}

msg_fn_attr!(as_msg_attr, "as_msg");
msg_fn_attr!(to_msg_attr, "to_msg");
msg_fn_attr!(from_msg_attr, "from_msg");
msg_fn_attr!(merge_msg_attr, "merge_msg");

msg_fn_attr!(as_msgs_attr, "as_msgs");
msg_fn_attr!(to_msgs_attr, "to_msgs");
