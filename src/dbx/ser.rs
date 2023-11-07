use super::{DBRow, SqlValue};
use crate::data::model::DataModel;
use crate::dbx::DBTable;
use element::SerElement;
use err::Error;
use serde::ser::{Impossible, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant};
use serde::{ser, Serialize};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::result::Result;
use tracing::info;

pub fn serialize_row<T>(model: Rc<DataModel>, v: T) -> Vec<crate::dbx::DBRow>
where
    T: serde::Serialize,
{
    let meta = model.meta();
    let s = RowSerializer::new(model.clone());
    match &v.serialize(&s) {
        Ok(x) => x.as_rows(None, &meta),
        Err(_) => todo!(),
    }
}

mod element {
    use std::fmt::Display;

    use rusqlite::types::Value;
    use tracing::{info, trace};

    use crate::{
        data::model::meta::{
            Meta,
            RelationKind::{Many, ManyMany, One},
        },
        dbx::{DBRow, SqlValue},
    };

    pub enum SerElement {
        Empty,
        Value(SqlValue),
        Sequence(Vec<SerElement>),
        Row(String, Vec<(String, SerElement)>),
    }

    impl SerElement {
        /// converts a SerElement structure to a list of rows ready to be posted to the database.
        ///
        pub fn as_rows(&self, context: Option<&str>, meta: &Meta) -> Vec<DBRow> {
            let mut result = vec![];
            match (self, context) {
                (SerElement::Empty, None) => todo!(),
                (SerElement::Empty, Some(_)) => todo!(),
                (SerElement::Value(_), None) => todo!(),
                (SerElement::Value(_), Some(_)) => todo!(),
                (SerElement::Sequence(s), context) => {
                    for x in s {
                        let mut ss = x.as_rows(context, meta);
                        result.append(&mut ss);
                    }
                }
                (SerElement::Row(n, r), None) => {
                    trace!("as rows Row {}", n);
                    let mut rr = DBRow::new(n.as_str());
                    for (k, v) in r {
                        match meta.get_relation(n.as_str(), k.as_str()) {
                            Some(r) => {
                                info!("found relation {} {}", n, k);
                                match &r.kind {
                                    One => {
                                        handle_one_relation(v, &mut rr, k, n, meta, &mut result);
                                        let sub_row = &result[0];
                                        for (f_fld, t_fld) in r.fields.iter() {
                                            info!("field map {} <- {}", f_fld, t_fld);
                                            rr.insert(
                                                f_fld.clone(),
                                                sub_row.get(t_fld).unwrap().clone(),
                                            );
                                        }
                                    }
                                    Many => {
                                        handle_many_relation(v, &mut rr, k, n, meta, &mut result);
                                        for sub_row in result.iter_mut() {
                                            for (f_fld, t_fld) in r.fields.iter() {
                                                info!("field map {} <- {}", t_fld, f_fld);
                                                sub_row.insert(
                                                    t_fld.clone(),
                                                    rr.get(f_fld).unwrap().clone(),
                                                );
                                            }
                                        }
                                    }
                                    ManyMany(rel_table) => {
                                        info!("many to many relation {}", rel_table);
                                        handle_many_many_relation(
                                            v,
                                            &mut rr,
                                            k,
                                            n,
                                            rel_table,
                                            meta,
                                            &mut result,
                                        );
                                    }
                                }
                            }
                            None => {
                                // info!("no relation {} {}", n, k);
                                handle_field(v, &mut rr, k, n, meta, &mut result);
                            }
                        };
                    }
                    result.insert(0, rr);
                }
                (SerElement::Row(_, r), Some(n)) => todo!(),
            }
            result
        }
    }

    impl Display for SerElement {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                SerElement::Empty => write!(f, "Empty"),
                SerElement::Value(x) => write!(f, "Value({x})"),
                SerElement::Sequence(s) => {
                    write!(f, "Sequence(")?;
                    let mut sep = "";
                    for x in s {
                        write!(f, "{}{}", sep, x)?;
                        sep = ", ";
                    }
                    write!(f, ")")
                }
                SerElement::Row(name, values) => {
                    write!(f, "{}(", name)?;
                    let mut sep = "";
                    for (k, v) in values {
                        write!(f, "{}{}={}", sep, k, v)?;
                        sep = ", ";
                    }
                    write!(f, ")")
                }
            }
        }
    }

    /// v - input value to be processed
    /// rr - the direct result row. This will be enhanced by the new value
    /// k - name of the field
    /// n - name of the table this row belongs to
    /// meta - information about relations and their fields
    /// result - is the list of rows that are returned on the side while processing the value.
    fn handle_field(
        v: &SerElement,
        rr: &mut DBRow,
        k: &String,
        n: &String,
        meta: &Meta,
        result: &mut Vec<DBRow>,
    ) {
        match v {
            SerElement::Empty => rr.insert(k.clone(), SqlValue(rusqlite::types::Value::Null)),
            SerElement::Value(v) => rr.insert(k.clone(), v.clone()),
            SerElement::Sequence(v) => {
                info!("sub row sequence {}.{}", n, k);
                for x in v {
                    let mut sub_rows = x.as_rows(None, meta);
                    result.append(&mut sub_rows);
                }
            }
            SerElement::Row(n, _) => {
                info!("sub row {}.{}", n, k);
                let mut sub_rows = v.as_rows(None, meta);
                result.append(&mut sub_rows);
            }
        }
    }

    /// v - input value to be processed
    /// rr - the direct result row. This will be enhanced by the new value
    /// k - name of the field
    /// n - name of the table this row belongs to
    /// meta - information about relations and their fields
    /// result - is the list of rows that are returned on the side while processing the value.
    fn handle_one_relation(
        v: &SerElement,
        rr: &mut DBRow,
        k: &String,
        n: &String,
        meta: &Meta,
        result: &mut Vec<DBRow>,
    ) {
        match v {
            SerElement::Empty => todo!("implement empty relationship"),
            SerElement::Value(v) => panic!("relation cannot use atomic values"),
            SerElement::Sequence(v) => panic!("one relations cannot refer to vectors"),
            SerElement::Row(n, _) => {
                let mut rr = v.as_rows(None, meta);
                result.append(&mut rr);
            }
        }
    }

    /// v - input value to be processed
    /// rr - the direct result row. This will be enhanced by the new value
    /// k - name of the field
    /// n - name of the table this row belongs to
    /// meta - information about relations and their fields
    /// result - is the list of rows that are returned on the side while processing the value.
    fn handle_many_relation(
        v: &SerElement,
        rr: &mut DBRow,
        k: &String,
        n: &String,
        meta: &Meta,
        result: &mut Vec<DBRow>,
    ) {
        info!("handle many relation {} {}", k, n);
        match v {
            SerElement::Empty => todo!("implement empty relationship"),
            SerElement::Value(v) => panic!("relation cannot use atomic values"),
            SerElement::Sequence(v) => {
                for x in v {
                    info!("many: handle row {}", x);
                    let mut rr = x.as_rows(None, meta);
                    result.append(&mut rr);
                }
            }
            SerElement::Row(n, _) => {
                let mut rr = v.as_rows(None, meta);
                result.append(&mut rr);
            }
        }
    }
    /// v - input value to be processed
    /// rr - the direct result row. This will be enhanced by the new value
    /// k - name of the field
    /// n - name of the table this row belongs to
    /// meta - information about relations and their fields
    /// result - is the list of rows that are returned on the side while processing the value.
    fn handle_many_many_relation(
        v: &SerElement,
        rr: &mut DBRow,
        k: &String,
        n: &String,
        rel_table: &String,
        meta: &Meta,
        result: &mut Vec<DBRow>,
    ) {
        info!("handle many many relation {} {} {}", k, n, rel_table);
        match v {
            SerElement::Empty => todo!("implement empty relationship"),
            SerElement::Value(v) => panic!("relation cannot use atomic values"),
            SerElement::Sequence(v) => {
                for x in v {
                    let mut sub_rows = x.as_rows(None, meta);
                    let mut rel_row = DBRow::new(&rel_table);
                    let rel = meta.get_relation(n, k).unwrap();
                    let row1 = &sub_rows[0];
                    for (fld, target) in rel.fields.iter() {
                        let vtarget;
                        if let Some(v0) = rr.get(fld.as_str()) {
                            vtarget = v0.clone();
                        } else {
                            if let Some(v0) = row1.get(fld.as_str()) {
                                vtarget = v0.clone();
                            } else {
                                panic!("field {} could not be mapped.", fld);
                            }
                        }
                        rel_row.insert(target.clone(), vtarget);
                    }
                    result.append(&mut sub_rows);
                    result.push(rel_row);
                }
            }
            SerElement::Row(n, _) => todo!(),
        }
    }

    impl From<SqlValue> for SerElement {
        fn from(value: SqlValue) -> Self {
            SerElement::Value(value)
        }
    }

    impl From<&SerElement> for SqlValue {
        fn from(value: &SerElement) -> Self {
            match value {
                SerElement::Empty => todo!(),
                SerElement::Value(v) => v.clone(),
                SerElement::Sequence(_) => todo!(),
                SerElement::Row(_, _) => todo!(),
            }
        }
    }
}
pub fn serialize_row_with_default<T>(model: Rc<DataModel>, default: DBRow, v: T) -> Vec<DBRow>
where
    T: serde::Serialize,
{
    let cr = model.meta();
    let mut s = RowSerializer::new(model.clone());
    s.with_default(default);
    match &v.serialize(&s) {
        Ok(x) => x.as_rows(None, &cr),
        Err(_) => todo!(),
    }
}

pub mod err {
    use std::fmt::Display;

    use serde::ser;

    #[derive(Debug)]
    pub enum Error {
        Generic(String),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Generic(msg) => write!(f, "sql ser error: {}", msg),
            }
        }
    }

    impl ser::StdError for Error {
        fn source(&self) -> Option<&(dyn ser::StdError + 'static)> {
            None
        }

        fn description(&self) -> &str {
            "description() is deprecated; use Display"
        }

        fn cause(&self) -> Option<&dyn ser::StdError> {
            self.source()
        }
    }

    impl serde::ser::Error for Error {
        fn custom<T>(msg: T) -> Self
        where
            T: Display,
        {
            Error::Generic(format!("{}", msg))
        }
    }
}
#[derive(Debug, Clone)]
pub struct CopyRuleLib {
    rules: BTreeMap<String, CopyRule>,
}

impl CopyRuleLib {
    pub fn new() -> Self {
        Self {
            rules: BTreeMap::new(),
        }
    }
    pub fn add(&mut self, name: &str, rule: CopyRule) {
        self.rules.insert(name.into(), rule);
    }

    fn get(&self, rel: &String) -> Option<&CopyRule> {
        if let Some(rules) = self.rules.get(rel) {
            Some(rules)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct FieldCopyRule {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone)]
pub struct ManyToMany {
    name: String,
    table1: String,
    copy1: CopyRule,
    table2: String,
    copy2: CopyRule,
}

#[derive(Debug, Clone)]
pub struct CopyRule {
    field_mappings: Vec<FieldCopyRule>,
    many_to_many: Option<Box<ManyToMany>>,
}

impl CopyRule {
    pub fn new(field_mappings: Vec<FieldCopyRule>) -> Self {
        Self {
            field_mappings,
            many_to_many: None,
        }
    }
    pub fn many_to_many(
        mut self,
        name: &str,
        table1: &str,
        copy1: CopyRule,
        table2: &str,
        copy2: CopyRule,
    ) -> Self {
        self.many_to_many = Some(Box::new(ManyToMany {
            name: name.to_string(),
            table1: table1.to_string(),
            copy1: copy1,
            table2: table2.to_string(),
            copy2: copy2,
        }));
        self
    }
}

#[derive(Debug, Clone)]
struct Dependency {
    record_number: usize,
    copy_rule: CopyRule,
}

pub struct NameSerializer {}

impl<'de> ser::Serializer for &NameSerializer {
    type Ok = String;
    type Error = Error;
    type SerializeSeq = Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeMap = Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(v.to_string())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        todo!()
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        todo!()
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        todo!()
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        todo!()
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        todo!()
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        todo!()
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        todo!()
    }
}

struct RowSerializer {
    model: Rc<DataModel>,
    default: Option<DBRow>,
}
struct DBRowSerializer {
    model: Rc<DataModel>,
    result: Vec<(String, SerElement)>,
    name: Option<String>,
}

impl DBRowSerializer {
    pub fn new(model: Rc<DataModel>, name: &str) -> Self {
        Self {
            model,
            result: vec![],
            name: Some(name.into()),
        }
    }

    pub fn get_default_values() -> Vec<(String, SqlValue)> {
        vec![]
    }
}
impl SerializeStruct for DBRowSerializer {
    type Ok = SerElement;

    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        // info!("serialize {}", key);
        let s = SQLValueSerializer {
            model: self.model.clone(),
            name: Some(key.into()),
        };
        self.result.push((key.into(), value.serialize(s).unwrap()));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Row(self.name.unwrap(), self.result))
    }
}

impl SerializeStructVariant for DBRowSerializer {
    type Ok = SerElement;
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let s = SQLValueSerializer {
            model: self.model.clone(),
            name: Some(key.into()),
        };
        self.result.push((key.into(), value.serialize(s).unwrap()));
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Row(self.name.unwrap(), self.result))
    }
}

impl RowSerializer {
    pub fn new(model: Rc<DataModel>) -> Self {
        Self {
            model,
            default: None,
        }
    }

    fn with_default(&mut self, default: DBRow) {
        self.default = Some(default);
    }
}

impl ser::SerializeMap for DBRowSerializer {
    type Ok = SerElement;
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let s = NameSerializer {};
        let n = key.serialize(&s).unwrap();
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

struct SQLValueSerializer {
    model: Rc<DataModel>,
    name: Option<String>,
}

impl ser::Serializer for SQLValueSerializer {
    type Ok = SerElement;
    type Error = Error;
    type SerializeSeq = TableSerializer;
    type SerializeTuple = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeMap = StructSerializer;
    type SerializeStruct = StructSerializer;
    type SerializeStructVariant = StructSerializer;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Value(SqlValue::new(v)))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Value(SqlValue::new(v)))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Value(SqlValue::new(v)))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Value(SqlValue::new(v)))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Value(SqlValue::new(v)))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Value(SqlValue::new(v)))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Value(SqlValue::new(v)))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Value(SqlValue::new(v)))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Value(SqlValue::new(v)))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Value(SqlValue::new(v)))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Value(SqlValue::new(v)))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Value(SqlValue::new(v as u16)))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Value(SqlValue::new(v)))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Value(SqlValue::new(v)))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Value(SqlValue::new(
            rusqlite::types::Value::Null,
        )))
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Value(SqlValue::new(variant)))
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(TableSerializer {
            model: self.model,
            rows: vec![],
            name: self.name.clone(),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        todo!()
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        todo!()
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        todo!()
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(StructSerializer::new(self.model.clone(), None))
    }

    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        // let r = serialize_row_with_default(self.model.clone(), default, value);
        // for x in r {
        //     self.rows.push(x);
        // }

        Ok(StructSerializer::new(self.model.clone(), Some(name.into())))
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(StructSerializer::new(
            self.model.clone(),
            Some(format!("{}.{}", name, variant)),
        ))
    }
}

impl<'de> ser::Serializer for &RowSerializer {
    type Ok = SerElement;
    type Error = Error;
    type SerializeSeq = TableSerializer;
    type SerializeTuple = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;
    type SerializeMap = DBRowSerializer;
    type SerializeStruct = DBRowSerializer;
    type SerializeStructVariant = DBRowSerializer;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        info!("serialize sequence");
        let s = TableSerializer {
            rows: vec![],
            model: self.model.clone(),
            name: None,
        };
        Ok(s)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        todo!()
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        todo!()
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        todo!()
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        todo!();
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        info!("serialize struct");
        let s = DBRowSerializer::new(self.model.clone(), name);
        Ok(s)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        todo!();
    }
}

struct StructSerializer {
    parent: DBRowSerializer,
    key: Option<String>,
}

impl StructSerializer {
    fn new(model: Rc<DataModel>, name: Option<String>) -> Self {
        Self {
            key: None,
            parent: DBRowSerializer {
                model,
                result: vec![],
                name,
            },
        }
    }

    fn serialize_field_impl<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        SerializeStruct::serialize_field(&mut self.parent, key, value)
    }
}

impl SerializeMap for StructSerializer {
    type Ok = SerElement;
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let s = NameSerializer {};
        self.key = Some(key.serialize(&s).unwrap());
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let k = &self.key;
        if let Some(key) = k {
            let s = SQLValueSerializer {
                model: self.parent.model.clone(),
                name: Some(key.into()),
            };
            self.parent
                .result
                .push((key.into(), value.serialize(s).unwrap()));
        };
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Row("map".to_string(), self.parent.result))
    }
}

impl SerializeStruct for StructSerializer {
    type Ok = SerElement;

    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_field_impl(key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Row(
            self.parent.name.unwrap(),
            self.parent.result,
        ))
    }
}

impl SerializeStructVariant for StructSerializer {
    type Ok = SerElement;

    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.serialize_field_impl(key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Row(
            self.parent.name.unwrap(),
            self.parent.result,
        ))
    }
}

struct TableSerializer {
    rows: Vec<SerElement>,
    model: Rc<DataModel>,
    name: Option<String>,
}

impl SerializeSeq for TableSerializer {
    type Ok = SerElement;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let s = SQLValueSerializer {
            model: self.model.clone(),
            name: self.name.clone(),
        };

        // let r = serialize_row_with_default(self.model.clone(), default, value);
        self.rows.push(value.serialize(s).unwrap());
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Sequence(self.rows))
    }
}

#[cfg(test)]
mod testing {
    use std::{collections::BTreeMap, rc::Rc};

    use crate::{
        data::model::{
            meta::{
                Meta,
                RelationKind::{Many, One},
            },
            DataModel,
        },
        dbx::{ser::CopyRule, SqlValue},
        TRACING,
    };

    use super::RowSerializer;
    use rusqlite::ToSql;
    use serde_derive::Serialize;
    use tracing::info;

    #[derive(Serialize)]
    enum Gender {
        #[serde(rename = "m")]
        Male,
        Female,
        Other,
    }

    #[derive(Serialize)]
    struct Person {
        personid: String,
        name: String,
        gender: Gender,
        age: u8,
        communications: Vec<Communication>,
        identification: BTreeMap<String, String>,
    }

    #[derive(Serialize)]
    enum Communication {
        EMail { personid: String, address: String },
        Phone { personid: String, number: String },
    }

    #[test]
    fn row_serializer() {
        let p = Person {
            personid: "#1".into(),
            name: "Peter Jaeckel".into(),
            gender: Gender::Male,
            age: 53,
            identification: BTreeMap::from([("A".to_string(), "B".to_string())]),
            communications: vec![
                Communication::EMail {
                    personid: String::new(),
                    address: "a@bc.de".into(),
                },
                Communication::Phone {
                    personid: String::new(),
                    number: "1234".into(),
                },
            ],
        };
        let model = DataModel::new("person");

        let rs = crate::dbx::ser::serialize_row(Rc::new(model), p);
        assert_eq!(3, rs.len());
        let r = &rs[0];
        assert!(String::from(r.get("name").unwrap().clone()) == String::from("Peter Jaeckel"));
        assert!(String::from(r.get("gender").unwrap().clone()) == String::from("m"));
        assert!(r.get("age").unwrap() == &SqlValue::from(53));
        assert_eq!(r.table(), "Person");
    }

    #[derive(Serialize)]
    struct Order {
        number: String,
        sold_to: Person,
    }

    #[test]
    fn order_serializer() {
        assert!(TRACING.clone());
        let o = Order {
            number: "#100".to_string(),
            sold_to: Person {
                personid: "#2".to_string(),
                name: "Lizzy".to_string(),
                gender: Gender::Female,
                age: 21,
                identification: BTreeMap::from([("X".to_string(), "Y".to_string())]),
                communications: vec![Communication::EMail {
                    personid: String::new(),
                    address: "ab@c.de".into(),
                }],
            },
        };
        let mut model = DataModel::new("order");
        let rule = CopyRule::new(vec![]);

        let mut meta = Meta::new();

        let rel = meta.define_relation(One, "Order", "sold_to", "Person");
        meta.map_field(rel.as_str(), "sold_to_id", "personid");

        let rel = meta.define_relation(Many, "Person", "communications", "EMail");
        meta.map_field(rel.as_str(), "personid", "personid");

        model.set_meta(meta);

        let rs = crate::dbx::ser::serialize_row(Rc::new(model), o);
        assert_eq!(4, rs.len());
        for r in rs.iter() {
            match r.table() {
                "Order" => {
                    info!("result row: {}", r);
                    assert!(r.get("sold_to_id") == Some(&SqlValue::from("#2")));
                }
                "Person" => info!("result row: {}", r),
                "Communication.EMail" => info!("result row: {}", r),
                "map" => info!("result row: {}", r),
                _ => panic!("unknown row type {}", r.table()),
            }
        }
    }
}
