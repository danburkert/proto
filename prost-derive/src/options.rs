use anyhow::{bail, Error};
use syn::{Attribute, Lit, Meta, MetaNameValue};

use crate::field::{bool_attr, prost_attrs, set_option};

pub struct Options {
    pub debug: bool,
    pub default: bool,
    pub merge: bool,
    pub proto: ProtoVersion,
}

#[derive(Debug, PartialEq)]
pub enum ProtoVersion {
    Proto2,
    Proto3,
}

impl Options {
    pub fn new(attrs: Vec<Attribute>) -> Result<Self, Error> {
        let mut debug = None;
        let mut default = None;
        let mut merge = None;
        let mut proto = None;
        let mut unknown_attrs = Vec::new();

        for attr in prost_attrs(attrs) {
            if let Some(d) = bool_attr("debug", &attr)? {
                set_option(&mut debug, d, "duplicate debug attribute")?;
            } else if let Some(d) = bool_attr("default", &attr)? {
                set_option(&mut default, d, "duplicate default attribute")?;
            } else if let Some(m) = bool_attr("merge", &attr)? {
                set_option(&mut merge, m, "duplicate merge attribute")?;
            } else if let Some(p) = ProtoVersion::new(&attr)? {
                set_option(&mut proto, p, "duplicate proto attribute")?;
            } else {
                unknown_attrs.push(attr);
            }
        }

        match unknown_attrs.len() {
            0 => (),
            1 => bail!("unknown attribute: {:?}", unknown_attrs[0]),
            _ => bail!("unknown attributes: {:?}", unknown_attrs),
        }

        Ok(Options {
            debug: debug.unwrap_or(true),
            default: default.unwrap_or(true),
            merge: merge.unwrap_or(true),
            proto: proto.unwrap_or(ProtoVersion::Proto2),
        })
    }
}

impl ProtoVersion {
    pub fn new(attr: &Meta) -> Result<Option<Self>, Error> {
        if !attr.path().is_ident("proto") {
            return Ok(None);
        }

        match *attr {
            Meta::NameValue(MetaNameValue {
                lit: Lit::Str(ref lit),
                ..
            }) => match lit.value().as_ref() {
                "proto2" => Ok(Some(ProtoVersion::Proto2)),
                "proto3" => Ok(Some(ProtoVersion::Proto3)),
                _ => bail!("invalid proto attribute: {:?}", lit),
            },
            _ => bail!("invalid proto attribute: {:?}", attr),
        }
    }

    pub fn is_proto3(&self) -> bool {
        *self == ProtoVersion::Proto3
    }
}
