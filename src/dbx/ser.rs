use std::{collections::BTreeMap, fmt::Display};

use std::result::Result;

use serde::{ser, Serialize};

use crate::data::Value;
use crate::tsort::TopSort;

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

type Row = BTreeMap<String, Value>;

#[derive(Debug)]
pub struct Context {
    rel: Option<String>,
    stack: Vec<Ctx>,
    copy_rules: CopyRuleLib,
    operations: Vec<Operation>,
    rows: Vec<Row>,
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
        self.rows.push(Row::new());
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
                    depends: None,
                },
            ),
            Ctx::Single {
                rel,
                ty,
                rowidx,
                parentrowidx,
            } => self.operations.insert(
                0,
                Operation {
                    index: rowidx,
                    table: ty,
                    depends: Some(Dependency {
                        record_number: parentrowidx,
                        copy_rule: self.copy_rules.get(&rel).clone(),
                    }),
                },
            ),
            Ctx::Multiple { .. } => {}
        };
    }

    fn set_value(&mut self, k: String, v: Value) {
        match self.stack.last() {
            Some(curr) => match curr {
                Ctx::Main { ty: r, rowidx } => {
                    let row = self.rows.get_mut(*rowidx).unwrap();
                    row.insert(k, v);
                }
                Ctx::Single { rel, ty, rowidx, .. } => {
                    let row = self.rows.get_mut(*rowidx).unwrap();
                    row.insert(k, v);
                },
                Ctx::Multiple { .. } => todo!(),
            },
            None => todo!(),
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

    fn get(&self, rel: &String) -> &CopyRule {
        self.rules.get(rel).unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct FieldMapping {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone)]
pub struct CopyRule {
    field_mappings: Vec<FieldMapping>,
}

impl CopyRule {
    pub fn new(field_mappings: Vec<FieldMapping>) -> Self {
        Self { field_mappings }
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
    table: String,
    depends: Option<Dependency>,
}

#[derive(Debug)]
pub struct SqlSerializer {
    pub counter: usize,
    pub tab_name: String,
    pub current_field: Option<String>,
    pub row: BTreeMap<String, Value>,
    pub operations: Vec<Operation>,
    pub stack: Vec<Operation>,
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
        self.post_value(v.into())
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
        if let Some(field) = &self.current_field {
            if let Some(op) = self.stack.last_mut() {
                self.context.set_value(field.clone(), Value::from(v));
            }
            self.current_field = None;
        }
        // else it is no atomic value, instead
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        todo!()
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
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
        // self.current_field = None;
        if let Some(name) = &self.current_field {
            self.context.set_relation(name.clone());
            self.context.enter_vec("vec".to_string());
        }

        Ok(self)
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
        variant_index: u32,
        variant: &'static str,
        len: usize,
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
            row: BTreeMap::new(),
            current_field: None,
            operations: vec![],
            stack: vec![],
            context: Context::new(copy_rules),
        }
    }

    fn enter(&mut self, name: &str) {
        let op = Operation {
            index: self.counter,
            table: String::from(name),
            depends: None,
        };
        self.counter += 1;
        self.stack.push(op);
    }

    fn exit(&mut self) {
        let op = self.stack.pop().unwrap();
        self.operations.push(op);
    }

    fn post_value(&self, v: Value) -> Result<(), Error> {
        Ok(())
    }

    pub(crate) fn get_operations(&self) -> Vec<&Operation> {
        let mut tsort = TopSort::new();
        for x in self.operations.iter() {
            if let Some(d) = &x.depends {
                tsort.add(d.record_number, x.index);
            }
        }

        let mut result = vec![];
        for n in tsort.sorted() {
            result.push(self.get_operation(n));
        }
        result
    }

    fn get_operation(&self, n: usize) -> &Operation {
        let r = self.operations.iter().find(|x| x.index == n).unwrap();
        r
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

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
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

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
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

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
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

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        todo!()
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
    println!("{:#?}", c.get_relationship());
    println!("{:#?}", c);
    c.leave();
    c.leave();
    c.leave();
    println!("{:#?}", c);
}
