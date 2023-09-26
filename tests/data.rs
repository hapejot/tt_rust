use std::fmt::Display;
use std::{collections::BTreeMap, thread::panicking};

use std::result::Result;

use serde::{ser, Serialize};
use tt_rust::data::{Query, Structure, Value, WhereCondition, WhereExpr};

#[derive(Debug)]
struct ValueMap(BTreeMap<String, Value>);


impl Structure for ValueMap {
    fn get(&self, name: &str) -> &Value {
        self.0.get(name).unwrap()
    }

    fn set(&mut self, name: &str, value: Value) {
        self.0.insert(String::from(name), value);
    }

    fn keys(&self) -> Vec<String> {
        todo!()
    }
}

impl From<ValueMap> for Value {
    fn from(value: ValueMap) -> Self {
        Value::Structure(Box::new(value))
    }
}

#[test]
fn scalar_string() {
    let x: Value = "Test".into();
    if let Value::Scalar(y) = x {
        assert_eq!(y.into_string(), "Test");
    } else {
        panic!()
    }
}

#[test]
fn structure() {
    let mut raw_value = BTreeMap::<String, Value>::new();
    raw_value.insert(String::from("Author"), "Peter".into());
    let x: Value = ValueMap(raw_value).into();
    if let Value::Structure(y) = x {
        if let Value::Scalar(z) = y.get(&String::from("Author")) {
            assert_eq!(z.into_string(), "Peter")
        } else {
            panic!()
        }
    } else {
        panic!()
    }
}

#[test]
fn query() {
    let w = WhereCondition::new().and(WhereExpr::Equals("type".into(), "Null".into()));

    let q = Query::new("object", vec!["id", "type"], w);
    let sql = q.get_sql();
    assert_eq!(sql, "SELECT id,type FROM object WHERE type = ?");
    let p = q.get_params();
    assert_eq!(p, vec!["Null"]);
}

