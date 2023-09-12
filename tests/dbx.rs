use tt_rust::dbx::DatabaseBuilder;

#[test]
fn first() {
    let mut builder = DatabaseBuilder::new();
    let db = builder
        .table(
            "object".into(),
            &["id".into(), "type".into()],
            &["id".into()],
        )
        .table(
            "entry".into(),
            &["id".into(), "objid".into(), "name".into()],
            &["id".into(), "name".into()],
        )
        .build();
    db.connect();
    assert!(db.is_connected());
}
