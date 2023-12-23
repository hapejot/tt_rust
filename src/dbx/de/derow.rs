use crate::error::Error;
use rusqlite::types::{FromSql, Value};
use serde::{
    de::{value::U32Deserializer, EnumAccess, IntoDeserializer, MapAccess, VariantAccess},
    forward_to_deserialize_any,
};
use tracing::trace;

pub struct Deserializer<'row, 'stmt, 'cols> {
    idx: Option<usize>,
    row: &'row rusqlite::Row<'stmt>,
    columns: &'cols [String],
}

impl<'row, 'stmt, 'cols> Deserializer<'row, 'stmt, 'cols> {
    pub(crate) fn from_row(row: &'row rusqlite::Row<'stmt>, columns: &'cols Vec<String>) -> Self {
        Deserializer {
            idx: None,
            row,
            columns,
        }
    }

    fn value<T: FromSql>(&self) -> Result<T, Error> {
        if let Some(idx) = self.idx {
            self.row
                .get(idx)
                .map_err(|_err| Error::Message("sqlite get index".to_string()))
        } else {
            panic!("not in field state.");
        }
    }
}

impl<'de> serde::Deserializer<'de> for Deserializer<'de, '_, '_> {
    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf    newtype_struct seq tuple
        tuple_struct   identifier ignored_any

        unit unit_struct
    }

    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let v = self.value();
        trace!("deserialize any {:?}", &v);
        match v {
            Ok(Value::Null) => visitor.visit_none(),
            Ok(Value::Integer(v)) => visitor.visit_i64(v),
            Ok(Value::Real(v)) => visitor.visit_f64(v),
            Ok(Value::Text(v)) => visitor.visit_string(v),
            Ok(Value::Blob(v)) => visitor.visit_seq(v.into_deserializer()),
            Err(_) => todo!(),
        }
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_map(RowMapAccess { idx: 0, de: self })
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value() {
            Ok(Value::Null) => visitor.visit_none(),
            Ok(_) => visitor.visit_some(self),
            Err(_) => todo!(),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let v = self.value();
        trace!("deserialize enum {:?}", &v);
        if let Ok(Value::Integer(n)) = v {
            visitor.visit_enum(EnumValue(n))
        } else {
            panic!("result value is not integer... {:?}", v)
        }
    }
}

struct EnumValue(i64);

impl<'de> VariantAccess<'de> for EnumValue {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        todo!()
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        todo!()
    }
}

impl<'de> EnumAccess<'de> for EnumValue {
    type Error = Error;

    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let variant_code_deserializer: U32Deserializer<Self::Error> =
            U32Deserializer::new(self.0 as u32);

        let value = seed
            .deserialize(variant_code_deserializer)?;

        Ok((value, self))
    }
}

struct RowMapAccess<'row, 'stmt, 'cols> {
    idx: usize,
    de: Deserializer<'row, 'stmt, 'cols>,
}

impl<'de> MapAccess<'de> for RowMapAccess<'de, '_, '_> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        if self.idx >= self.de.columns.len() {
            Ok(None)
        } else {
            let column = self.de.columns[self.idx].as_str();
            seed.deserialize(column.into_deserializer())
                .map(Some)
                .map_err(|e| add_field_to_error(e, column))
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let out = seed
            .deserialize(Deserializer {
                idx: Some(self.idx),
                row: self.de.row,
                columns: self.de.columns,
            })
            .map_err(|e| add_field_to_error(e, &self.de.columns[self.idx]));
        self.idx += 1;
        out
    }
}

#[allow(unused_mut)]
fn add_field_to_error(mut error: Error, _error_column: &str) -> Error {
    error
}
