use core::{fmt, marker::PhantomData};

use super::{DeserializeInto, DeserializerConfig, MaybeDeserializedValue};

pub struct OptionDeserializer<I>(PhantomData<I>);

impl<T, I> DeserializeInto<Option<T>> for OptionDeserializer<I>
where
    I: DeserializeInto<T>,
{
    #[inline]
    fn deserialize_into<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        config: &DeserializerConfig,
    ) -> Result<Option<T>, D::Error> {
        struct Visitor<'c, T, I>(&'c DeserializerConfig, PhantomData<(T, I)>);

        impl<'de, T, I> serde::de::Visitor<'de> for Visitor<'_, T, I>
        where
            I: DeserializeInto<T>,
        {
            type Value = Option<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "an option")
            }

            #[inline]
            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(None)
            }

            #[inline]
            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(None)
            }

            #[inline]
            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                I::deserialize_into(deserializer, self.0).map(Some)
            }
        }

        if I::can_deserialize_null() {
            Ok(Some(I::deserialize_into(deserializer, config)?))
        } else {
            deserializer.deserialize_option(Visitor::<T, I>(config, PhantomData))
        }
    }

    fn maybe_deserialize_into<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        config: &DeserializerConfig,
    ) -> Result<MaybeDeserializedValue<Option<T>>, D::Error> {
        struct Visitor<'c, T, I>(&'c DeserializerConfig, PhantomData<(T, I)>);

        impl<'de, T, I> serde::de::Visitor<'de> for Visitor<'_, T, I>
        where
            I: DeserializeInto<T>,
        {
            type Value = MaybeDeserializedValue<Option<T>>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "an option")
            }

            #[inline]
            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(MaybeDeserializedValue::Val(None))
            }

            #[inline]
            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(MaybeDeserializedValue::Val(None))
            }

            #[inline]
            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                Ok(I::maybe_deserialize_into(deserializer, self.0)?.map(Some))
            }
        }

        if I::can_deserialize_null() {
            Ok(I::maybe_deserialize_into(deserializer, config)?.map(Some))
        } else {
            deserializer.deserialize_option(Visitor::<T, I>(config, PhantomData))
        }
    }

    #[inline]
    fn can_deserialize_null() -> bool {
        true
    }
}

pub struct NullDeserializer;

impl DeserializeInto<()> for NullDeserializer {
    #[inline]
    fn deserialize_into<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        _config: &DeserializerConfig,
    ) -> Result<(), D::Error> {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a null value")
            }

            #[inline]
            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(())
            }

            #[inline]
            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(())
            }
        }

        deserializer.deserialize_option(Visitor)
    }

    #[inline]
    fn can_deserialize_null() -> bool {
        true
    }
}
