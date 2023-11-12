use tt_rust::evaluate_script;

#[test]
fn eval() {
    evaluate_script(String::from("1+2.")).unwrap();
}