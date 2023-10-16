use serde::Serialize;
use serde_derive::Serialize;
use tt_rust::{
    data::{Query, WhereCondition, WhereExpr},
    dbx::{
        ser::{CopyRule, CopyRuleLib, FieldMapping},
        Database, DatabaseBuilder,
    },
};

#[test]
fn modify_from() {
    let mut builder = DatabaseBuilder::new();
    let db = builder
        .table(
            "object".into(),
            &["id".into(), "type".into(), "flags".into()],
            &["id".into()],
        )
        .table(
            "entry".into(),
            &["id".into(), "objid".into(), "name".into()],
            &["id".into(), "name".into()],
        )
        .build();
    db.connect();
    db.activate_structure();

    let mut s = db.new_structure();
    s.set("id", "1".into());
    s.set("type", "Null".into());
    // s.set(&String::from("id"), "1".into());
    // s.set(&String::from("type"), "Nul".into());
    assert_eq!(s.keys(), vec!["id", "type"]);
    db.modify_from("object".into(), s);
    assert!(db.is_connected());
}

#[test]
fn select() {
    let db = DatabaseBuilder::new().build();
    db.connect();

    let q = Query::new(
        "object",
        vec!["id", "type"],
        WhereCondition::new().and(WhereExpr::Equals("type".into(), "Null".into())),
    );

    let res = db.select(q);
}

#[derive(Debug, Serialize)]
#[serde(rename = "communication")]
pub enum Communication {
    #[serde(rename = "phone")]
    Phone {
        id: Option<usize>,
        personid: Option<usize>,
        number: String,
        role: String,
    },
    #[serde(rename = "email")]
    EMail {
        id: Option<usize>,
        personid: Option<usize>,
        address: String,
        role: String,
    },
}
#[derive(Debug, Serialize)]
#[serde(rename = "person")]
struct Person {
    id: Option<usize>,
    name1: String,
    name2: String,
    communications: Vec<Communication>,
    #[serde(rename = "adelstitle")]
    name3: Option<String>,
    name4: Option<String>,
}

#[test]

fn serialize() {
    let mut crs = CopyRuleLib::new();
    let copy_rule_1 = CopyRule::new(vec![FieldMapping {
        source: "id".to_string(),
        target: "personid".to_string(),
    }]);

    let p = Person {
        name1: "Peter".to_string(),
        name2: "Jaeckel".to_string(),
        name3: Some("Freiherr".to_string()),
        name4: None,
        communications: vec![
            Communication::Phone {
                number: "+4912345".to_string(),
                role: "fake".to_string(),
                id: None,
                personid: None,
            },
            Communication::EMail {
                address: "a@bc.de".to_string(),
                role: "dummy".to_string(),
                id: None,
                personid: None,
            },
        ],
        id: None,
    };

    let mut builder = DatabaseBuilder::new();
    let db = builder
        .table(
            "person".into(),
            &[
                "name1".to_string(),
                "name2".to_string(),
                "adelstitle".to_string(),
                "name4".to_string(),
            ],
            &["name1".to_string()],
        )
        .copy_rule("communications".to_string(), copy_rule_1)
        .build();
    db.connect();

    db.activate_structure();

    db.modify_from_ser(&p).unwrap();
    assert!(db.is_connected());
}
