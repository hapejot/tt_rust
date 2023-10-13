use std::fmt::Display;
use std::{collections::BTreeMap, thread::panicking};

use std::result::Result;

use serde::{ser, Serialize};
use tt_rust::data::{Query, Structure, Value, WhereCondition, WhereExpr, ValueWalker};

#[test]
fn scalar_string() {
    let x: Value = "Test".into();
    if let Value::ScalarValue(y) = x {
        assert_eq!(y.into_string(), "Test");
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

#[test]
fn walking_values() {
    let mut substruct_v = Structure::new();
    substruct_v.set("url", Value::from("http://www.google.de"));
    let mut struct_v = Structure::new();
    struct_v.set("id", "1".into());
    struct_v.set("img", Value::from(substruct_v));
    let vec_v = vec![struct_v];

    let v = Value::from(vec_v);
    println!("{:#?}", &v);

    let walker = ValueWalker::new(&v);
    for x in walker {
        println!("{:#?}",x);
    }
}
