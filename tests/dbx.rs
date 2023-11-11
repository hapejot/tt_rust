use std::collections::BTreeMap;

use serde_derive::Serialize;
use tracing::*;
use tt_rust::{
    data::{
        model::{
            meta::{Meta, RelationKind::One},
            DataModel, Table,
        },
        Query, WhereCondition, WhereExpr,
    },
    dbx::{
        ser::{CopyRule, CopyRuleLib, FieldCopyRule},
        DBRow, Database, DatabaseBuilder,
    },
    TRACING,
};

#[test]
fn modify_from() {
    let db = prepare_database_object();

    let mut s = DBRow::new("object");
    s.set("id", "1".into());
    s.set("type", "Null".into());
    assert_eq!(s.keys(), vec!["id", "type"]);
    db.modify_from("object".into(), &s);
    assert!(db.is_connected());
}

fn prepare_database_object() -> tt_rust::dbx::Database {
    let model = DataModel::new("object").table(
        Table::new("object")
            .field("id", true, "string")
            .field("type", false, "string")
            .field("flags", false, "string"),
    );

    let builder = DatabaseBuilder::new();
    let db = builder.build();
    db.connect(None);
    db.activate_structure(model);
    db
}

#[test]
fn select() {
    let db = prepare_database_object();

    let q = Query::new(
        "object",
        vec!["id", "type"],
        WhereCondition::new().and(WhereExpr::Equals("type".into(), "Null".into())),
    );

    let res: Vec<BTreeMap<String, String>> = db.select(q);
    trace!("result: {:?}", res);
}

#[derive(Debug, Serialize)]
#[serde(rename = "communication")]
pub enum Communication {
    #[serde(rename = "phone")]
    Phone {
        id: Option<String>,
        number: String,
        role: String,
    },
    #[serde(rename = "email")]
    EMail {
        id: Option<String>,
        address: String,
        role: String,
    },
}

pub fn new_guid() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Debug, Serialize)]
#[serde(rename = "person")]
struct Person {
    id: String,
    name1: String,
    name2: String,
    communications: Vec<Communication>,
    name3: Option<String>,
    name4: Option<String>,
}

#[test]
fn serialize() {
    assert!(TRACING.clone());

    let p = Person {
        name1: "Peter".to_string(),
        name2: "Jaeckel".to_string(),
        name3: Some("Freiherr".to_string()),
        name4: None,
        communications: vec![
            Communication::Phone {
                number: "+4912345".to_string(),
                role: "fake".to_string(),
                id: Some(new_guid()),
            },
            Communication::EMail {
                address: "a@bc.de".to_string(),
                role: "dummy".to_string(),
                id: Some(new_guid()),
            },
        ],
        id: new_guid(),
    };

    let db = prepare_person_db();
    assert!(db.is_connected());

    db.modify_from_ser(&p).unwrap();
    let res = db.execute_query("select * from person");
    assert_eq!(1, res.len());
    for x in res {
        info!("person: {}", x);
    }

    let res = db.execute_query("select * from email");
    for x in res.iter() {
        info!("email: {}", x);
    }
    assert_eq!(1, res.len());

    let res = db.execute_query("select * from phone");
    assert_eq!(1, res.len());
    for x in res {
        info!("phone: {}", x);
    }
}

fn prepare_person_db() -> tt_rust::dbx::Database {
    let mut builder = DatabaseBuilder::new();
    let copy_rule_1 = CopyRule::new(vec![FieldCopyRule {
        source: "id".to_string(),
        target: "personid".to_string(),
    }]);
    let model = make_person_model();
    let db = builder.build();
    db.connect(None);

    db.activate_structure(model);
    db
}

fn make_person_model() -> DataModel {
    let mut model = DataModel::new("Person");
    let mut tab = Table::new("person")
        .field("id", true, "string")
        .field("name1", false, "string")
        .field("name2", false, "string")
        .field("name3", false, "string")
        .field("name4", false, "string");
    model = model
        .table(tab)
        .table(
            Table::new("email")
                .field("id", true, "string")
                .field("personid", false, "string")
                .field("role", false, "string")
                .field("address", false, "string"),
        )
        .table(
            Table::new("phone")
                .field("id", true, "string")
                .field("personid", false, "string")
                .field("role", false, "string")
                .field("number", false, "string"),
        );
    let mut meta = Meta::new();
    meta.define_relation(One, "person", "communication.email", "email");
    meta.define_relation(One, "person", "communication.phone", "phone");
    model.set_meta(meta);
    model
}

#[test]
fn test_new_model() {
    assert!(TRACING.clone());
    let model = make_person_model();
    let db = Database::new();
    db.connect(None);
    db.activate_structure(model.clone());
    for t in db.tables() {
        info!("table: {}", t);
    }
    // check if we can activate the same model again without errors
    db.activate_structure(model);
}
