use tt_rust::{evaluate_script, runtime::sel::SelectorSet, TRACING};

#[test]
fn string_format() {
    assert!(TRACING.clone());
    let o = evaluate_script(String::from("
    'Five is {1}.' format: 1 + 4.
    ")).unwrap();
    SelectorSet::stats();
    assert_eq!(o.as_str(), Some("Five is 5."));
}


#[test]
fn block_1() {
    assert!(TRACING.clone());
    // [:x| x + 2] value: 1.
    // [:x :y| x + y] value:1 value:2.
    let o = evaluate_script(String::from("
    [:x :y| x + y] value:1 value:2.
    ")).unwrap();
    SelectorSet::stats();
    assert_eq!(o.as_int(), Some(3));
}


#[test]
fn points_2() {
    assert!(TRACING.clone());
    let o = evaluate_script(String::from("
    a := 100 @ 200. 
    b <- 300 @ 400. 
    Point x: a x + b y y: a y + b x.")).unwrap();
    SelectorSet::stats();
    assert_eq!(o.receive_message("x", &[]).as_int(), Some(500));
    assert_eq!(o.receive_message("y", &[]).as_int(), Some(500));
}


#[test]
fn points_1() {
    assert!(TRACING.clone());
    let o = evaluate_script(String::from("a := 100 @ 200. b <- 300 @ 400. a + b.")).unwrap();
    SelectorSet::stats();
    assert_eq!(o.receive_message("x", &[]).as_int(), Some(400));
    assert_eq!(o.receive_message("y", &[]).as_int(), Some(600));
}


#[test]
fn eval_vars() {
    assert!(TRACING.clone());
    let o = evaluate_script(String::from("a := 1. b <- 2. a + b.")).unwrap();
    SelectorSet::stats();
    assert_eq!(o.as_int(), Some(3));
}

#[test]
fn eval() {
    assert!(TRACING.clone());
    let o = evaluate_script(String::from("1 + 2 * 3.")).unwrap();
    SelectorSet::stats();
    assert_eq!(o.as_int(), Some(9));
}

#[test]
fn eval_to_7() {
    assert!(TRACING.clone());
    let o = evaluate_script(String::from("1 + (2 * 3).")).unwrap();
    SelectorSet::stats();
    assert_eq!(o.as_int(), Some(7));
}
