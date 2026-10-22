//! Preserve Serde's typed decoding while requiring named structs and symbolic tags.
//! Ordinary sequences/maps (including numeric property keys) retain their meaning.
use serde::{
    de::{self, DeserializeSeed, Visitor},
    Deserialize, Deserializer,
};

pub(super) struct Named<D>(pub D);
struct Wrapped<V> {
    inner: V,
    fields: bool,
}
struct Seed<S>(S);
struct NameSeed<S>(S);

impl<'de, S: DeserializeSeed<'de>> DeserializeSeed<'de> for Seed<S> {
    type Value = S::Value;
    fn deserialize<D: Deserializer<'de>>(self, de: D) -> Result<Self::Value, D::Error> {
        self.0.deserialize(Named(de))
    }
}
impl<'de, S: DeserializeSeed<'de>> DeserializeSeed<'de> for NameSeed<S> {
    type Value = S::Value;
    fn deserialize<D: Deserializer<'de>>(self, de: D) -> Result<Self::Value, D::Error> {
        let name = String::deserialize(de)?;
        self.0
            .deserialize(de::value::StringDeserializer::<D::Error>::new(name))
    }
}

macro_rules! read {
    ($method:ident $(,$arg:ident:$ty:ty)*)=> {
        fn $method<V:Visitor<'de>>(self,$($arg:$ty,)* visitor:V)->Result<V::Value,Self::Error> {
            self.0.$method($($arg,)* Wrapped { inner:visitor,fields:false })
        }
    }
}
impl<'de, D: Deserializer<'de>> Deserializer<'de> for Named<D> {
    type Error = D::Error;
    read!(deserialize_any);
    read!(deserialize_bool);
    read!(deserialize_i8);
    read!(deserialize_i16);
    read!(deserialize_i32);
    read!(deserialize_i64);
    read!(deserialize_i128);
    read!(deserialize_u8);
    read!(deserialize_u16);
    read!(deserialize_u32);
    read!(deserialize_u64);
    read!(deserialize_u128);
    read!(deserialize_f32);
    read!(deserialize_f64);
    read!(deserialize_char);
    read!(deserialize_str);
    read!(deserialize_string);
    read!(deserialize_bytes);
    read!(deserialize_byte_buf);
    read!(deserialize_option);
    read!(deserialize_unit);
    read!(deserialize_unit_struct,name:&'static str);
    read!(deserialize_newtype_struct,name:&'static str);
    read!(deserialize_seq);
    read!(deserialize_tuple,len:usize);
    read!(deserialize_tuple_struct,name:&'static str,len:usize);
    read!(deserialize_map);
    read!(deserialize_identifier);
    read!(deserialize_ignored_any);
    read!(deserialize_enum,name:&'static str,variants:&'static [&'static str]);
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.0.deserialize_struct(
            name,
            fields,
            Wrapped {
                inner: visitor,
                fields: true,
            },
        )
    }
    fn is_human_readable(&self) -> bool {
        self.0.is_human_readable()
    }
}

macro_rules! visit {
    ($method:ident,$ty:ty) => {
        fn $method<E: de::Error>(self, value: $ty) -> Result<Self::Value, E> {
            self.inner.$method(value)
        }
    };
}
impl<'de, V: Visitor<'de>> Visitor<'de> for Wrapped<V> {
    type Value = V::Value;
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.expecting(f)
    }
    visit!(visit_bool, bool);
    visit!(visit_i8, i8);
    visit!(visit_i16, i16);
    visit!(visit_i32, i32);
    visit!(visit_i64, i64);
    visit!(visit_i128, i128);
    visit!(visit_u8, u8);
    visit!(visit_u16, u16);
    visit!(visit_u32, u32);
    visit!(visit_u64, u64);
    visit!(visit_u128, u128);
    visit!(visit_f32, f32);
    visit!(visit_f64, f64);
    visit!(visit_char, char);
    visit!(visit_str, &str);
    visit!(visit_borrowed_str, &'de str);
    visit!(visit_string, String);
    visit!(visit_bytes, &[u8]);
    visit!(visit_borrowed_bytes, &'de [u8]);
    visit!(visit_byte_buf, Vec<u8>);
    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        self.inner.visit_unit()
    }
    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        self.inner.visit_none()
    }
    fn visit_some<D: Deserializer<'de>>(self, de: D) -> Result<Self::Value, D::Error> {
        self.inner.visit_some(Named(de))
    }
    fn visit_newtype_struct<D: Deserializer<'de>>(self, de: D) -> Result<Self::Value, D::Error> {
        self.inner.visit_newtype_struct(Named(de))
    }
    fn visit_seq<A: de::SeqAccess<'de>>(self, access: A) -> Result<Self::Value, A::Error> {
        if self.fields {
            return Err(de::Error::custom(
                "expected a named-field map, not a positional struct",
            ));
        }
        self.inner.visit_seq(Sequence(access))
    }
    fn visit_map<A: de::MapAccess<'de>>(self, access: A) -> Result<Self::Value, A::Error> {
        self.inner.visit_map(Map {
            inner: access,
            fields: self.fields,
        })
    }
    fn visit_enum<A: de::EnumAccess<'de>>(self, access: A) -> Result<Self::Value, A::Error> {
        self.inner.visit_enum(Enum(access))
    }
}

struct Sequence<A>(A);
impl<'de, A: de::SeqAccess<'de>> de::SeqAccess<'de> for Sequence<A> {
    type Error = A::Error;
    fn next_element_seed<S: DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<Option<S::Value>, Self::Error> {
        self.0.next_element_seed(Seed(seed))
    }
    fn size_hint(&self) -> Option<usize> {
        self.0.size_hint()
    }
}
struct Map<A> {
    inner: A,
    fields: bool,
}
impl<'de, A: de::MapAccess<'de>> de::MapAccess<'de> for Map<A> {
    type Error = A::Error;
    fn next_key_seed<S: DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<Option<S::Value>, Self::Error> {
        if self.fields {
            self.inner.next_key_seed(NameSeed(seed))
        } else {
            self.inner.next_key_seed(Seed(seed))
        }
    }
    fn next_value_seed<S: DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<S::Value, Self::Error> {
        self.inner.next_value_seed(Seed(seed))
    }
    fn size_hint(&self) -> Option<usize> {
        self.inner.size_hint()
    }
}
struct Enum<A>(A);
impl<'de, A: de::EnumAccess<'de>> de::EnumAccess<'de> for Enum<A> {
    type Error = A::Error;
    type Variant = Variant<A::Variant>;
    fn variant_seed<S: DeserializeSeed<'de>>(
        self,
        seed: S,
    ) -> Result<(S::Value, Self::Variant), Self::Error> {
        let (value, variant) = self.0.variant_seed(NameSeed(seed))?;
        Ok((value, Variant(variant)))
    }
}
struct Variant<A>(A);
impl<'de, A: de::VariantAccess<'de>> de::VariantAccess<'de> for Variant<A> {
    type Error = A::Error;
    fn unit_variant(self) -> Result<(), Self::Error> {
        self.0.unit_variant()
    }
    fn newtype_variant_seed<S: DeserializeSeed<'de>>(
        self,
        seed: S,
    ) -> Result<S::Value, Self::Error> {
        self.0.newtype_variant_seed(Seed(seed))
    }
    fn tuple_variant<V: Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.0.tuple_variant(
            len,
            Wrapped {
                inner: visitor,
                fields: false,
            },
        )
    }
    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.0.struct_variant(
            fields,
            Wrapped {
                inner: visitor,
                fields: true,
            },
        )
    }
}
