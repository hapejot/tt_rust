use std::{fs::File, io::Read, path::Path, rc::Rc};

use tt_rust::{
    compile_script, parse_method, parser::AST::Method, CompiledMethod, Frame, Operation, TRACING,
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
    let mut f = Frame::new(Rc::new(code));
    let r = f.run();
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
    let mut f = Frame::new(Rc::new(code));
    let r = format!("{}", f.run());
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
    let mut f = Frame::new(Rc::new(code));
    let r = format!("{}", f.run());
    assert_eq!("X", r);
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
