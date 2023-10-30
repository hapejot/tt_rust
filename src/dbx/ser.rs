use std::{collections::BTreeMap, fmt::Display};

use std::result::Result;

use rusqlite::types::{Null, Value};
use rusqlite::ToSql;
use serde::ser::{Impossible, SerializeSeq, SerializeStruct, SerializeStructVariant};
use serde::{forward_to_deserialize_any, ser, Serialize};
use tracing::info;

use crate::data::model::Table;

use super::{DBRow, SqlValue};

enum SerElement {
    Value(SqlValue),
    Sequence(Vec<DBRow>),
    Row(DBRow),
}

pub fn serialize_row<T>(v: T) -> Vec<crate::dbx::DBRow>
where
    T: serde::Serialize,
{
    let s = RowSerializer::new();
    match v.serialize(&s) {
        Ok(x) => x,
        Err(_) => todo!(),
    }
}

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

#[derive(Debug)]
pub enum Ctx {
    Main {
        ty: String,
        rowidx: usize,
    },
    Single {
        rel: String,
        ty: String,
        rowidx: usize,
        parentrowidx: usize,
    },
    Multiple {
        rel: String,
        ty: String,
    },
}

pub struct Context {
    rel: Option<String>,
    stack: Vec<Ctx>,
    copy_rules: CopyRuleLib,
    operations: Vec<Operation>,
    rows: Vec<DBRow>,
}

impl Context {
    fn new(copy_rules: CopyRuleLib) -> Self {
        Self {
            rel: None,
            stack: vec![],
            copy_rules,
            operations: vec![],
            rows: vec![],
        }
    }

    fn enter(&mut self, ty: String) {
        let rel = self.rel.clone();
        match self.stack.last() {
            Some(x) => match x {
                Ctx::Main { .. } => self.push_context(&rel, ty),
                Ctx::Single { .. } => self.push_context(&rel, ty),
                Ctx::Multiple { rel, .. } => self.push_context(&Some(rel.clone()), ty),
            },
            None => self.push_context(&rel, ty),
        }
    }

    fn push_context(&mut self, rel: &Option<String>, ty: String) {
        let rowidx = self.rows.len();
        self.rows.push(DBRow::new());
        match rel {
            Some(rel) => self.stack.push(Ctx::Single {
                rel: rel.clone(),
                ty,
                rowidx,
                parentrowidx: 0,
            }),
            None => self.stack.push(Ctx::Main { ty, rowidx }),
        }
    }

    fn enter_vec(&mut self, ty: String) {
        match &self.rel {
            Some(rel) => self.stack.push(Ctx::Multiple {
                rel: rel.clone(),
                ty,
            }),
            None => todo!(),
        }
    }

    fn set_relation(&mut self, rel: String) {
        self.rel = Some(rel)
    }

    fn get_relationship(&self) -> Option<Relationship> {
        match self.stack.last() {
            Some(x) => match x {
                Ctx::Main { .. } => None,
                Ctx::Single { rel, .. } => Some(Relationship { name: rel.clone() }),
                Ctx::Multiple { rel, .. } => Some(Relationship { name: rel.clone() }),
            },
            None => todo!(),
        }
    }

    fn leave(&mut self) {
        match self.stack.pop().unwrap() {
            Ctx::Main { ty, rowidx } => self.operations.insert(
                0,
                Operation {
                    index: rowidx,
                    table: ty,
                    depends: vec![],
                    done: false,
                },
            ),
            Ctx::Single {
                rel,
                ty,
                rowidx,
                parentrowidx,
            } => {
                let copy_rule = self.copy_rules.get(&rel);
                if let Some(cr) = copy_rule {
                    self.operations.insert(
                        0,
                        Operation {
                            index: rowidx,
                            table: ty,
                            done: false,
                            depends: vec![Dependency {
                                record_number: parentrowidx,
                                copy_rule: cr.clone(),
                            }],
                        },
                    );
                    if let Some(m2m) = &cr.many_to_many {
                        let many_to_many_idx = self.rows.len();
                        self.rows.push(DBRow::new());
                        self.operations.insert(
                            1,
                            Operation {
                                index: many_to_many_idx,
                                table: m2m.name.clone(),
                                done: false,
                                depends: vec![
                                    Dependency {
                                        record_number: parentrowidx,
                                        copy_rule: m2m.copy1.clone(),
                                    },
                                    Dependency {
                                        record_number: rowidx,
                                        copy_rule: m2m.copy2.clone(),
                                    },
                                ],
                            },
                        );
                    }
                } else {
                    self.operations.insert(
                        0,
                        Operation {
                            index: rowidx,
                            table: ty,
                            depends: vec![],
                            done: false,
                        },
                    )
                }
            }
            Ctx::Multiple { .. } => {}
        };
    }

    fn set_value<T: Into<SqlValue>>(&mut self, k: String, v: T) {
        match self.stack.last() {
            Some(curr) => match curr {
                Ctx::Main { ty: _r, rowidx } => {
                    let row = self.rows.get_mut(*rowidx).unwrap();
                    row.insert(k, v.into());
                }
                Ctx::Single { rowidx, .. } => {
                    let row = self.rows.get_mut(*rowidx).unwrap();
                    row.insert(k, v.into());
                }
                Ctx::Multiple { .. } => todo!(),
            },
            None => todo!(),
        }
    }

    pub(crate) fn get_row(&self, index: usize) -> &DBRow {
        self.rows.get(index).unwrap()
    }

    pub fn get_row_mut(&mut self, index: usize) -> &mut DBRow {
        self.rows.get_mut(index).unwrap()
    }

    fn get_operation(&self, n: usize) -> &Operation {
        if let Some(op) = self.operations.get(n) {
            op
        } else {
            panic!("no operation at index {}", n);
        }
    }

    /// loop through all the operations and apply the copy rules
    fn perform_copy_rules(&mut self) {
        let work_list = self.prepare_work_list_for_copy_rules();

        for (source, target, rule) in work_list {
            self.copy_rows(source, target, rule);
        }
    }

    fn prepare_work_list_for_copy_rules(&mut self) -> Vec<(usize, usize, CopyRule)> {
        let work_list = {
            let ops = self.get_operations();
            let mut work = vec![];
            for x in ops {
                let Operation { index, depends, .. } = x;
                for dep in depends {
                    work.push((dep.record_number, *index, dep.copy_rule.clone()));
                }
            }
            work
        };
        work_list
    }

    fn get_operations(&self) -> Vec<&Operation> {
        self.operations
            .iter()
            .map(|x| x)
            .collect::<Vec<&Operation>>()
    }

    fn copy_rows(&mut self, from: usize, to: usize, copy_rule: CopyRule) {
        info!("copy row from:{} to:{}", from, to);
        let source_row = self.get_row(from).clone();
        let target_row = self.get_row_mut(to);
        let fmap = &copy_rule.field_mappings;
        if fmap.len() == 0 {
        } else {
            for FieldCopyRule { source, target } in copy_rule.field_mappings.iter() {
                info!("field {} to {}", source, target);
                if let Some(v) = source_row.get(source) {
                    target_row.insert(target.clone(), v.clone());
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Relationship {
    name: String,
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
    pub fn add(&mut self, name: String, rule: CopyRule) {
        self.rules.insert(name, rule);
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

#[derive(Debug)]
pub struct Operation {
    index: usize,
    done: bool,
    table: String,
    depends: Vec<Dependency>,
}
impl Operation {
    pub(crate) fn get_index(&self) -> usize {
        self.index
    }

    pub(crate) fn get_table(&self) -> String {
        self.table.clone()
    }
}

pub struct SqlSerializer {
    pub counter: usize,
    pub tab_name: String,
    pub current_field: Option<String>,
    pub row: DBRow,
    // pub operations: Vec<Operation>,
    // pub stack: Vec<Operation>,
    pub context: Context,
}

impl<'a> ser::Serializer for &'a mut SqlSerializer {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.post_value(v)
    }

    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.post_value(v)
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        if let Some(field) = &self.current_field {
            self.context.set_value(field.clone(), v);
        }
        // else it is no atomic value, instead
        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        if let Some(field) = &self.current_field {
            self.context.set_value(field.clone(), Value::Null);
            self.current_field = None;
        }
        Ok(())
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

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        // self.current_field = None;
        if let Some(name) = &self.current_field {
            self.context.set_relation(name.clone());
            self.context.enter_vec("vec".to_string());
        }

        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        todo!()
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        todo!()
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        todo!()
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        todo!()
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.tab_name = String::from(name);
        self.enter(name);
        if let Some(name) = &self.current_field {
            self.context.set_relation(name.clone());
        }
        self.context.enter(name.to_string());
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.enter(name);
        if let Some(name) = &self.current_field {
            self.context.set_relation(name.clone());
        }
        self.context.enter(variant.to_string());
        Ok(self)
    }
}

impl SqlSerializer {
    pub fn new(copy_rules: CopyRuleLib) -> Self {
        Self {
            counter: 1,
            tab_name: String::new(),
            row: DBRow::new(),
            current_field: None,
            // operations: vec![],
            // stack: vec![],
            context: Context::new(copy_rules),
        }
    }

    fn enter(&mut self, _name: &str) {
        // let op = Operation {
        //     index: self.counter,
        //     table: String::from(name),
        //     depends: None,
        // };
        self.counter += 1;
        // self.stack.push(op);
    }

    fn exit(&mut self) {
        // let op = self.stack.pop().unwrap();
        // self.operations.push(op);
    }

    fn post_value<T: Into<SqlValue>>(&mut self, v: T) -> Result<(), Error> {
        if let Some(field) = &self.current_field {
            self.context.set_value(field.clone(), v);
            self.current_field = None;
        }
        Ok(())
    }

    pub(crate) fn get_operations(&self) -> Vec<&Operation> {
        let result = self.context.operations.iter().collect();
        result
    }

    fn get_operation(&self, n: usize) -> &Operation {
        let r = self.context.get_operation(n);
        r
    }

    pub(crate) fn perform_copy_rules(&mut self) {
        info!("handle dependencies");
        self.context.perform_copy_rules();
    }
}

impl<'a> ser::SerializeSeq for &'a mut SqlSerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.context.leave();
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for &'a mut SqlSerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}
impl<'a> ser::SerializeTupleStruct for &'a mut SqlSerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut SqlSerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}
impl<'a> ser::SerializeMap for &'a mut SqlSerializer {
    type Ok = ();

    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, _key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_value<T: ?Sized>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!()
    }
}

impl<'a> ser::SerializeStruct for &'a mut SqlSerializer {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.current_field = Some(String::from(key));
        value.serialize(&mut **self).unwrap();
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        println!("done with whatever");
        self.exit();
        self.context.leave();
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for &'a mut SqlSerializer {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.current_field = Some(String::from(key));
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.exit();
        self.context.leave();
        Ok(())
    }
}

#[test]
fn context() {
    let mut c = Context::new(CopyRuleLib::new());
    c.enter("S1".to_string());
    c.set_relation("r1".to_string());
    c.enter("S3".to_string());
    c.set_relation("r3".to_string());
    c.enter_vec("V".to_string());
    c.set_relation("r2".to_string());
    c.enter("S2".to_string());
    c.leave();
    c.leave();
    c.leave();
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

pub struct RowSerializer {}
pub struct DBRowSerializer {
    rows: Vec<DBRow>,
    name: Option<String>,
}

impl DBRowSerializer {
    pub fn new(table: &str) -> Self {
        Self {
            rows: vec![DBRow {
                table: Some(String::from(table)),
                values: vec![],
            }],
            name: None,
        }
    }
}

impl RowSerializer {
    pub fn new() -> Self {
        Self {}
    }
}

impl ser::SerializeMap for DBRowSerializer {
    type Ok = Vec<DBRow>;
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let s = NameSerializer {};
        let n = key.serialize(&s).unwrap();
        self.name = Some(n);
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

struct SQLValueSerializer {}

impl ser::Serializer for SQLValueSerializer {
    type Ok = SerElement;
    type Error = Error;
    type SerializeSeq = TableSerializer;
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
        Ok(SerElement::Value(SqlValue::new(v)))
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
        Ok(SerElement::Value(SqlValue::new(v)))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Value(SqlValue::new::<Option<String>>(None)))
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
        Ok(TableSerializer { rows: vec![] })
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

impl SerializeStruct for DBRowSerializer {
    type Ok = Vec<DBRow>;

    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let s = SQLValueSerializer {};
        match value.serialize(s).unwrap() {
            SerElement::Value(v) => self.rows[0].insert(key.to_string(), v),
            SerElement::Sequence(seq) => {
                for x in seq {
                    self.rows.push(x);
                }
            }
            SerElement::Row(_) => todo!(),
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.rows)
    }
}

impl SerializeStructVariant for DBRowSerializer {
    type Ok = Vec<DBRow>;
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let s = SQLValueSerializer {};
        match value.serialize(s).unwrap() {
            SerElement::Value(v) => self.rows[0].insert(key.to_string(), v),
            SerElement::Sequence(seq) => {
                todo!("sequence b");
                for x in seq {
                    self.rows.push(x);
                }
            }
            SerElement::Row(_) => todo!(),
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.rows)
    }
}

impl<'de> ser::Serializer for &RowSerializer {
    type Ok = Vec<DBRow>;
    type Error = Error;
    type SerializeSeq = Impossible<Self::Ok, Self::Error>;
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
        let s = DBRowSerializer::new("row");
        Ok(s)
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        let s = DBRowSerializer::new(name);
        Ok(s)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        let s = DBRowSerializer::new(variant);
        Ok(s)
    }
}

struct TableSerializer {
    rows: Vec<DBRow>,
}

impl SerializeSeq for TableSerializer {
    type Ok = SerElement;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let r = serialize_row(value);
        for x in r {
            self.rows.push(x);
        }
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(SerElement::Sequence(self.rows))
    }
}

#[cfg(test)]
mod testing {
    use crate::dbx::SqlValue;

    use super::RowSerializer;
    use rusqlite::ToSql;
    use serde_derive::Serialize;

    #[derive(Serialize)]
    enum Gender {
        #[serde(rename = "m")]
        Male,
        Femal,
        Other,
    }

    #[derive(Serialize)]
    struct Person {
        name: String,
        gender: Gender,
        age: u8,
        communications: Vec<Communication>,
    }

    #[derive(Serialize)]
    struct Communication {
        role: String,
        address: String,
    }

    #[test]
    fn row_serializer() {
        let p = Person {
            name: "Peter Jaeckel".into(),
            gender: Gender::Male,
            age: 53,
            communications: vec![
                Communication{ role: "email".into(), address: "a@bc.de".into() },
                Communication{ role: "phone".into(), address: "123456".into() },
            ],
        };

        let rs = crate::dbx::ser::serialize_row(p);
        assert_eq!(3, rs.len());
        let r = &rs[0];
        assert!(String::from(r.get("name").unwrap().clone()) == String::from("Peter Jaeckel"));
        assert!(String::from(r.get("gender").unwrap().clone()) == String::from("m"));
        assert!(r.get("age").unwrap() == &SqlValue::from(53));
        assert_eq!(r.table(), "Person");
    }
}
