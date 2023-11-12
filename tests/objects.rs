use tt_rust::{evaluate_script, runtime::sel::SelectorSet, TRACING};

#[test]
fn eval() {
    assert!(TRACING.clone());
    let o = evaluate_script(String::from("1 + 2 * 3.")).unwrap();

    SelectorSet::stats();

    assert_eq!(o.as_int(), Some(9));
}
