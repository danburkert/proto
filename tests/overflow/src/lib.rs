#![cfg_attr(not(feature = "std"), no_std)]

#[cfg_attr(test, macro_use)]
extern crate cfg_if;

#[cfg(test)]
mod encoded_len;
