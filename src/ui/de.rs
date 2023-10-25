use serde::{de, forward_to_deserialize_any};
use serde::{
    de::MapAccess,
    Deserialize,
};

pub type Values = Vec<(String, String)>;

#[derive(Debug)]
pub enum Error {
    SomeError(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SomeError(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

impl de::Error for Error {
    #[doc = " Raised when there is general error when deserializing a type."]
    #[doc = ""]
    #[doc = " The message should not be capitalized and should not end with a period."]
    #[doc = ""]
    #[doc = " ```edition2021"]
    #[doc = " # use std::str::FromStr;"]
    #[doc = " #"]
    #[doc = " # struct IpAddr;"]
    #[doc = " #"]
    #[doc = " # impl FromStr for IpAddr {"]
    #[doc = " #     type Err = String;"]
    #[doc = " #"]
    #[doc = " #     fn from_str(_: &str) -> Result<Self, String> {"]
    #[doc = " #         unimplemented!()"]
    #[doc = " #     }"]
    #[doc = " # }"]
    #[doc = " #"]
    #[doc = " use serde::de::{self, Deserialize, Deserializer};"]
    #[doc = ""]
    #[doc = " impl<'de> Deserialize<'de> for IpAddr {"]
    #[doc = "     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>"]
    #[doc = "     where"]
    #[doc = "         D: Deserializer<'de>,"]
    #[doc = "     {"]
    #[doc = "         let s = String::deserialize(deserializer)?;"]
    #[doc = "         s.parse().map_err(de::Error::custom)"]
    #[doc = "     }"]
    #[doc = " }"]
    #[doc = " ```"]
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::SomeError(format!("{}", msg))
    }
}



// By convention, the public API of a Serde deserializer is one or more
// `from_xyz` methods such as `from_str`, `from_bytes`, or `from_reader`
// depending on what Rust types the deserializer is able to consume as input.
//
// This basic deserializer supports only `from_str`.
pub fn from_values<'a, T>(s: &'a Vec<(String, String)>) -> T
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_values(s);

    match T::deserialize(&mut deserializer) {
        Ok(t) => t,
        Err(e) => {
            panic!("Result: {:?}", e);
        }
    }
}

pub struct Deserializer<'de> {
    idx: usize,
    key: bool,
    input: &'de Values,
}

impl<'de> Deserializer<'de> {
    fn from_values(s: &'de Vec<(String, String)>) -> Self {
        Self {
            input: s,
            idx: 0,
            key: true,
        }
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.input.get(self.idx) {
            Some((key, val)) => {
                if self.key {
                    self.key = false;
                    visitor.visit_string(key.clone())
                } else {
                    self.key = true;
                    self.idx += 1;
                    visitor.visit_string(val.clone())
                }
            }
            None => todo!(),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple
        tuple_struct enum identifier ignored_any
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_map(MyMapAccess::new(self))
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de> {
        visitor.visit_some(self)
    }
}

struct MyMapAccess<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> MyMapAccess<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self { de }
    }
}

impl<'a, 'de> MapAccess<'de> for MyMapAccess<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.de.idx < self.de.input.len() {
            seed.deserialize(&mut *self.de).map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
    }
}
