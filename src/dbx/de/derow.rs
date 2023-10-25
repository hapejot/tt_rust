use crate::error::Error;
use rusqlite::types::{FromSql, Value};
use serde::{
    de::{IntoDeserializer, MapAccess},
    forward_to_deserialize_any,
};

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
                .map_err(|err| Error::Message("sqlite get index".to_string()))
        } else {
            panic!("not in field state.");
        }
    }
}

impl<'de> serde::Deserializer<'de> for Deserializer<'de, '_, '_> {
    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf  unit unit_struct newtype_struct seq tuple
        tuple_struct   enum identifier ignored_any
    }

    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value() {
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
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de> {
            match self.value() {
                Ok(Value::Null) => visitor.visit_none(),
                Ok(_) => visitor.visit_some(self),
                Err(_) => todo!(),
            }
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

fn add_field_to_error(mut error: Error, _error_column: &str) -> Error {
    error
}
