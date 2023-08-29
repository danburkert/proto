use anyhow::{bail, Error};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Meta;

use crate::field::{set_bool, word_attr};

pub fn encode_unknown(_ident: TokenStream) -> TokenStream {
    // TODO(jason): handle differently named `unknown_fields` structs (since users may want to name them something different)
    // actually we need to write this more hygienically
    quote! {
        self.unknown_fields.encode_raw(buf)
    }
}

pub fn merge_unknown(_ident: TokenStream) -> TokenStream {
    quote! {
        self.unknown_fields.merge_next_field(wire_type, tag, buf)
    }
}

pub fn encoding_len_unknown(_ident: TokenStream) -> TokenStream {
    quote! {
        self.unknown_fields.encoded_len()
    }
}

pub fn clear_unknown(ident: TokenStream) -> TokenStream {
    quote! {
        #ident = ::prost::unknown::UnknownFields::default();
    }
}

pub fn matches_attrs(attrs: &[Meta]) -> Result<Option<()>, Error> {
    let mut is_unknown_fields = false;

    let mut unknown_attrs = Vec::new();

    for attr in attrs {
        if word_attr("unknown", attr) {
            set_bool(&mut is_unknown_fields, "duplicate group attributes")?;
        } else {
            unknown_attrs.push(attr);
        }
    }

    match unknown_attrs.len() {
        0 => (),
        1 => bail!("unknown attribute for group field: {:?}", unknown_attrs[0]),
        _ => bail!("unknown attributes for group field: {:?}", unknown_attrs),
    }

    if is_unknown_fields {
        Ok(Some(()))
    } else {
        Ok(None)
    }
}
