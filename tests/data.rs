use tt_rust::data::{
    model::{DataModel, Table},
    Query, WhereCondition, WhereExpr,
};

use std::slice::Iter;

#[test]
fn query() {
    let w = WhereCondition::new().and(WhereExpr::Equals("type".into(), "Null".into()));

    let q = Query::new("object", vec!["id", "type"], w);
    let sql = q.get_sql();
    assert_eq!(sql, "SELECT id,type FROM object WHERE type = ?");
    let p = q.get_params();
    // assert_eq!(p, vec!["Null"]);
}

#[test]
fn data_model() {
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


        assert_eq!("person", model.tables().next().unwrap().name());
}
