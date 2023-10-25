use tt_rust::data::{Query, WhereCondition, WhereExpr};


#[test]
fn query() {
    let w = WhereCondition::new().and(WhereExpr::Equals("type".into(), "Null".into()));

    let q = Query::new("object", vec!["id", "type"], w);
    let sql = q.get_sql();
    assert_eq!(sql, "SELECT id,type FROM object WHERE type = ?");
    let p = q.get_params();
    // assert_eq!(p, vec!["Null"]);
}

