use crate::encoding::{DecodeContext, WireType};
use crate::{DecodeError, Message, UnknownEnumValue};

use bytes::{Buf, BufMut};

use core::fmt::Debug;

/// Represents the value of an open enum field.
///
/// The [Protocol Buffers guide][proto-guide] specifies that unknown values
/// of fields with open enum types should be stored directly in the field
/// when decoding messages. This type provides an ergonomic way to represent
/// such values in Rust.
///
/// [proto-guide]: https://protobuf.dev/programming-guides/enum/#definitions
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OpenEnum<T> {
    /// A known value of the generated enum type.
    Known(T),
    /// An unknown value as decoded from the message.
    Unknown(i32),
}

impl<T> Default for OpenEnum<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::Known(T::default())
    }
}

impl<T> From<T> for OpenEnum<T> {
    fn from(value: T) -> Self {
        Self::Known(value)
    }
}

impl<T> OpenEnum<T> {
    /// Converts a raw integer value into an open enum value.
    ///
    /// This method is used to decode field values from the wire format.
    pub fn from_raw(value: i32) -> Self
    where
        i32: TryInto<T>,
    {
        match value.try_into() {
            Ok(v) => Self::Known(v),
            Err(_) => Self::Unknown(value),
        }
    }

    /// Converts an open enum value into its raw integer representation.
    pub fn into_raw(self) -> i32
    where
        T: Into<i32>,
    {
        match self {
            Self::Known(v) => v.into(),
            Self::Unknown(v) => v,
        }
    }

    /// Converts an open enum value into its raw integer representation.
    ///
    /// This is a convenience method for borrowed values.
    pub fn to_raw(&self) -> i32
    where
        T: Clone + Into<i32>,
    {
        match self {
            Self::Known(v) => v.clone().into(),
            Self::Unknown(v) => *v,
        }
    }
}

impl<T> OpenEnum<T> {
    /// Returns the known value of the open enum.
    ///
    /// # Panics
    ///
    /// Panics if the value is in fact unknown.
    pub fn unwrap(self) -> T {
        match self {
            Self::Known(v) => v,
            Self::Unknown(v) => panic!("unknown field value {}", v),
        }
    }

    /// Returns the known value of the open enum, or, if the value is unknown,
    /// returns the provided default value.
    ///
    /// Arguments passed to `unwrap_or` are eagerly evaluated; if you are passing
    /// the result of a function call, it is recommended to use
    /// [`unwrap_or_else`] and pass a lazily evaluated closure to it.
    ///
    /// [`unwrap_or_else`]: #method.unwrap_or_else
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Self::Known(v) => v,
            Self::Unknown(_) => default,
        }
    }

    /// Returns the known value of the open enum, or, if the value is unknown,
    /// returns the value computed from the field's raw integer value by the
    /// provided closure.
    pub fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce(i32) -> T,
    {
        match self {
            Self::Known(v) => v,
            Self::Unknown(v) => f(v),
        }
    }

    /// Returns the known value of the open enum, or, if the value is unknown,
    /// returns the default value of the enum type.
    pub fn unwrap_or_default(self) -> T
    where
        T: Default,
    {
        match self {
            Self::Known(v) => v,
            Self::Unknown(_) => T::default(),
        }
    }

    /// If the value of the open enum is known, returns it in `Ok`, otherwise
    /// returns an `Err` with the unknown value.
    pub fn get(&self) -> Result<T, UnknownEnumValue>
    where
        T: Clone,
    {
        match self {
            Self::Known(v) => Ok(v.clone()),
            Self::Unknown(r) => Err(UnknownEnumValue(*r)),
        }
    }

    /// Sets the value of receiver to the provided known value.
    pub fn set(&mut self, value: T) {
        *self = Self::Known(value);
    }

    /// If the value of the open enum is known, returns it in `Some`, otherwise
    /// returns `None`.
    pub fn known(self) -> Option<T> {
        match self {
            Self::Known(v) => Some(v),
            Self::Unknown(_) => None,
        }
    }

    /// If the value of the open enum is known, returns it in `Ok`, otherwise
    /// returns the provided error value in `Err`.
    ///
    /// Arguments passed to `known_or` are eagerly evaluated; if you are passing
    /// the result of a function call, it is recommended to use
    /// [`known_or_else`] and pass a lazily evaluated closure to it.
    ///
    /// [`known_or_else`]: #method.known_or_else
    pub fn known_or<E>(self, err: E) -> Result<T, E> {
        match self {
            Self::Known(v) => Ok(v),
            Self::Unknown(_) => Err(err),
        }
    }

    /// If the value of the open enum is known, returns it in `Ok`, otherwise
    /// returns `Err` with the value computed from the field's raw integer value
    /// by the provided closure.
    pub fn known_or_else<E, F>(self, err: F) -> Result<T, E>
    where
        F: FnOnce(i32) -> E,
    {
        match self {
            Self::Known(v) => Ok(v),
            Self::Unknown(v) => Err(err(v)),
        }
    }
}

impl<T> Message for OpenEnum<T>
where
    T: Clone + Into<i32> + Debug + Send + Sync,
    i32: TryInto<T>,
{
    fn encoded_len(&self) -> usize {
        self.to_raw().encoded_len()
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        self.to_raw().encode_raw(buf)
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        let mut raw = 0;
        <i32 as Message>::merge_field(&mut raw, tag, wire_type, buf, ctx)?;
        *self = OpenEnum::from_raw(raw);
        Ok(())
    }

    fn clear(&mut self) {
        *self = OpenEnum::from_raw(0);
    }
}
