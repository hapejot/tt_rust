use tt_rust::{TRACING, compile_script};


#[test]
fn compile_to_7() {
    assert!(TRACING.clone());
    let code = compile_script(String::from("
        1 + (2 * 3).")).unwrap();
    println!("{}", code);
}

#[test]
fn compile_to_points() {
    assert!(TRACING.clone());
    let code = compile_script(String::from("
        a := 100 @ 200. b <- 300 @ 400. a + b.")).unwrap();
    println!("{}", code);
}


#[test]
fn compile_block() {
    assert!(TRACING.clone());
    let code = compile_script(String::from("
    '' species new: 10 streamContents: [ :result | result nextPut: $X ].
    ")).unwrap();
    println!("{}", code);
}


#[test]
fn compile_to_more_points() {
    assert!(TRACING.clone());
    let code = compile_script(String::from("
        a := 100 @ 200. 
        b <- 300 @ 400. 
        Point x: a x + b y y: a y + b x.")).unwrap();
    println!("{}", code);
}
