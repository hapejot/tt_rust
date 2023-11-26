use std::{fs::File, io::Read, path::Path};

use tt_rust::{
    code::{compile_script, CompiledMethod, Operation},
    parse_method,
    parser::AST::Method,
    ContextRef, MethodContext, TRACING,
};

#[test]
fn compile_to_7() {
    assert!(TRACING.clone());
    let code = compile_script(String::from(
        "
        1 + (2 * 3).",
    ))
    .unwrap();
    println!("{}", code);
    let f: ContextRef = MethodContext::new();
    let r = code.run(f);
    assert_eq!(Some(7), r.as_int());
}

#[test]
fn compile_to_points() {
    assert!(TRACING.clone());
    let code = compile_script(String::from(
        "
        a := 100 @ 200. b <- 300 @ 400. a + b.",
    ))
    .unwrap();
    println!("{}", code);
    let f = MethodContext::new();
    let r = format!("{}", code.run(f));
    assert_eq!("400@600", r);
}

#[test]
fn compile_block() {
    assert!(TRACING.clone());
    let code = compile_script(String::from(
        "
        '' species new: 10 streamContents: [ :result | result nextPut: $X ].
        ",
    ))
    .unwrap();
    println!("{}", code);
    let f = MethodContext::new();
    let r = format!("{}", code.run(f));
    assert_eq!("X", r);
}

#[test]
fn parse_binary() {
    assert!(TRACING.clone());
    let code = compile_script(String::from("^1 < 2")).unwrap();
    println!("{}", code);
    let f = MethodContext::new();
    let r = format!("{}", code.run(f));
    assert_eq!("True", r);
}

#[test]
fn compile_if_true() {
    assert!(TRACING.clone());
    let code = compile_script(String::from(
        "
        a := 1.
        a < 2 ifTrue: [a := 3].
        ^a
        ",
    ))
    .unwrap();
    println!("{}", code);
    let f = MethodContext::new();
    let r = code.run(f).as_int();
    assert_eq!(Some(3), r);
}

#[test]
fn compile_if_false() {
    assert!(TRACING.clone());
    let code = compile_script(String::from(
        "
        a := 1.
        a > 2 ifFalse: [a := 3].
        ^a
        ",
    ))
    .unwrap();
    println!("{}", code);
    let f = MethodContext::new();
    let r = code.run(f).as_int();
    assert_eq!(Some(3), r);
}

#[test]
fn compile_to_more_points() {
    assert!(TRACING.clone());
    let code = compile_script(String::from(
        "
        a := 100 @ 200. 
        b <- 300 @ 400. 
        Point x: a x + b y y: a y + b x.",
    ))
    .unwrap();
    println!("{}", code);
    // let f = MethodContext::new();
    // let r = format!("{}", code.run(f));
    // assert_eq!("True", r);
}

#[test]
fn compile_stored_method() {
    let selector = "format:";
    let p = format!("defs/string/{}", selector).replace(r":", "_");
    let p = Path::new(&p);
    if !p.exists() {
        panic!("unresolved method {}", selector);
    }

    let mut f = File::open(p).unwrap();
    let mut buf = String::new();
    f.read_to_string(&mut buf).unwrap();

    let t = parse_method(buf).unwrap()[0].clone();
    // trace!("tree: {}", t);
    let m = t.as_abstract_syntax_tree();
    match m {
        Method { body, params, .. } => {
            let mut code = CompiledMethod::new();
            let addr = code.push(Operation::Myself);
            code.define("self".into(), addr);
            for idx in 0..params.len() {
                let addr = code.push(Operation::Param(idx));
                code.define(params[idx].to_string(), addr);
            }
            let _idx = code.compile(&body);
            println!("{}", code);
        }
        _ => todo!("I only know how to deal with a method."),
    }
}
