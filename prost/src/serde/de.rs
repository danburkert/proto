use alloc::{boxed::Box, format};
use core::marker::PhantomData;
use serde::{de::DeserializeSeed, Deserializer};

use super::DeserializerConfig;

mod bytes;
mod default;
mod r#enum;
mod forward;
mod map;
mod message;
mod oneof;
mod option;
mod scalar;
mod vec;

/// This is an extended and cut-down version of serde's [serde::Deserialize].
///
/// The main changes are:
///   - the addition of an additional argument `config` ([DeserializerConfig]). Deserializers can
///     use that to change their deserialization behavior.
///   - the `can_deserialize_null` method.
///
pub trait CustomDeserialize<'de>: Sized {
    /// Deserialize `Self` from the given `deserializer` and `config`.
    fn deserialize<D>(deserializer: D, config: &DeserializerConfig) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>;

    /// By default this impl doesn't support deserializing from `null` values.
    #[inline]
    fn can_deserialize_null() -> bool {
        false
    }
}

impl<'de, T> CustomDeserialize<'de> for Box<T>
where
    T: CustomDeserialize<'de>,
{
    #[inline]
    fn deserialize<D>(deserializer: D, config: &DeserializerConfig) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let val = <T as CustomDeserialize>::deserialize(deserializer, config)?;
        Ok(Box::new(val))
    }
}

// FIXME: Make `T` contravariant, not covariant, by changing the `T` in `PhantomData` to
// `fn() -> T`.
pub struct DesWithConfig<'c, T>(pub &'c DeserializerConfig, PhantomData<fn() -> T>);

impl<'c, T> DesWithConfig<'c, T> {
    #[inline]
    pub fn new(config: &'c DeserializerConfig) -> Self {
        Self(config, PhantomData)
    }
}

impl<'de, T> serde::de::DeserializeSeed<'de> for DesWithConfig<'_, T>
where
    T: CustomDeserialize<'de>,
{
    type Value = T;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <T as CustomDeserialize<'de>>::deserialize(deserializer, self.0)
    }
}

#[derive(Debug)]
pub enum MaybeDeserializedValue<T> {
    Val(T),
    UnknownEnumValue,
}

impl<T> MaybeDeserializedValue<T> {
    #[inline]
    pub fn map<R>(self, f: impl FnOnce(T) -> R) -> MaybeDeserializedValue<R> {
        match self {
            Self::Val(val) => MaybeDeserializedValue::Val(f(val)),
            Self::UnknownEnumValue => MaybeDeserializedValue::UnknownEnumValue,
        }
    }

    #[inline]
    pub fn unwrap_for_field<E>(
        self,
        config: &DeserializerConfig,
        field_name: &'static str,
    ) -> Result<T, E>
    where
        E: serde::de::Error,
        T: Default,
    {
        match self {
            Self::Val(val) => Ok(val),
            Self::UnknownEnumValue if config.ignore_unknown_enum_string_values => Ok(T::default()),
            Self::UnknownEnumValue => Err(E::custom(format!(
                "found an unknown enum value at field `{field_name}`"
            ))),
        }
    }

    #[inline]
    pub fn unwrap_for_omittable<E>(
        self,
        config: &DeserializerConfig,
        location: &'static str,
    ) -> Result<Option<T>, E>
    where
        E: serde::de::Error,
    {
        match self {
            Self::Val(val) => Ok(Some(val)),
            Self::UnknownEnumValue if config.ignore_unknown_enum_string_values => Ok(None),
            Self::UnknownEnumValue => Err(E::custom(format!(
                "found an unknown enum value `{location}`"
            ))),
        }
    }
}

impl<T> From<T> for MaybeDeserializedValue<T> {
    #[inline]
    fn from(val: T) -> Self {
        Self::Val(val)
    }
}

pub trait DeserializeInto<T> {
    fn deserialize_into<'de, D: Deserializer<'de>>(
        deserializer: D,
        config: &DeserializerConfig,
    ) -> Result<T, D::Error>;

    fn maybe_deserialize_into<'de, D: Deserializer<'de>>(
        deserializer: D,
        config: &DeserializerConfig,
    ) -> Result<MaybeDeserializedValue<T>, D::Error> {
        Self::deserialize_into(deserializer, config).map(MaybeDeserializedValue::Val)
    }

    #[inline]
    fn can_deserialize_null() -> bool {
        false
    }
}

pub struct DesIntoWithConfig<'c, W, T>(pub &'c DeserializerConfig, PhantomData<(W, T)>);

impl<'c, W, T> DesIntoWithConfig<'c, W, T> {
    #[inline]
    pub fn new(config: &'c DeserializerConfig) -> Self {
        Self(config, PhantomData)
    }
}

impl<'de, W, T> DeserializeSeed<'de> for DesIntoWithConfig<'_, W, T>
where
    W: DeserializeInto<T>,
{
    type Value = T;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        <W as DeserializeInto<T>>::deserialize_into(deserializer, self.0)
    }
}

pub struct MaybeDesIntoWithConfig<'c, W, T>(pub &'c DeserializerConfig, PhantomData<(W, T)>);

impl<'c, W, T> MaybeDesIntoWithConfig<'c, W, T> {
    #[inline]
    pub fn new(config: &'c DeserializerConfig) -> Self {
        Self(config, PhantomData)
    }
}

impl<'de, W, T> DeserializeSeed<'de> for MaybeDesIntoWithConfig<'_, W, T>
where
    W: DeserializeInto<T>,
{
    type Value = MaybeDeserializedValue<T>;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        <W as DeserializeInto<T>>::maybe_deserialize_into(deserializer, self.0)
    }
}

// Re-export all deserializers.
// FIXME: Remove the `self::` when we've bumped the MSRV to 1.72.
pub use self::bytes::BytesDeserializer;
pub use default::DefaultDeserializer;
pub use forward::ForwardDeserializer;
pub use map::MapDeserializer;
pub use message::MessageDeserializer;
pub use oneof::{DeserializeOneOf, OneOfDeserializer};
pub use option::{NullDeserializer, OptionDeserializer};
pub use r#enum::{DeserializeEnum, EnumDeserializer};
pub use scalar::{BoolDeserializer, FloatDeserializer, IntDeserializer};
pub use vec::VecDeserializer;

mod size_hint {
    use core::{cmp, mem};

    #[inline]
    pub fn cautious<Element>(hint: Option<usize>) -> usize {
        const MAX_PREALLOC_BYTES: usize = 1024 * 1024;

        if mem::size_of::<Element>() == 0 {
            0
        } else {
            cmp::min(
                hint.unwrap_or(0),
                MAX_PREALLOC_BYTES / mem::size_of::<Element>(),
            )
        }
    }
}
