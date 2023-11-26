use core::ffi::FromBytesUntilNulError;
use std::{
    collections::BTreeMap,
    fmt::Display,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::Mutex,
};

use once_cell::sync::Lazy;

use crate::{
    parse_script,
    parser::AST,
    runtime::{
        chr::CharReceiver, int::IntReceiver, sel::SelectorSet, str::StringReceiver, Receiver,
    },
    BlockContext, ContextRef,
};

struct MethodCache {
    cache: Mutex<BTreeMap<String, &'static CompiledMethod>>,
}

impl MethodCache {
    pub fn add(name: &str, meth: CompiledMethod) {
        let mut c = METHOD_CACHE.cache.try_lock().unwrap();

        c.insert(name.into(), Box::leak(Box::new(meth)));
    }

    pub fn get(name: &str) -> &'static CompiledMethod {
        let c = METHOD_CACHE.cache.try_lock().unwrap();
        c.get(name).unwrap()
    }
}

static METHOD_CACHE: Lazy<MethodCache> = Lazy::new(|| MethodCache {
    cache: Mutex::new(BTreeMap::new()),
});

#[derive(Debug)]
pub struct CompiledMethod {
    data: CompiledMethodData,
}

impl Deref for CompiledMethod {
    type Target = CompiledMethodData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for CompiledMethod {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

#[derive(Debug)]
pub struct CompiledMethodData {
    blocks: Vec<ByteCode>,
    stack: Vec<CodeAddress>,
    current_block: usize,
    names: Vec<(String, CodeAddress)>,
}

pub struct CompiledBlock {
    method: &'static CompiledMethod,
    ctx: ContextRef,
    block: usize,
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct CodeAddress(pub usize, pub usize);

#[derive(Debug)]
pub struct ByteCode {
    result: Option<CodeAddress>,
    opcode: Vec<Operation>,
}

#[derive(Debug)]
pub enum Operation {
    Int(isize),
    Str(String),
    Invoke(String, CodeAddress, Vec<CodeAddress>),
    Block(usize),
    Global(String),
    Char(char),
    String(String),
    Arg(usize),
    Return(CodeAddress),
    Param(usize),
    Myself,
    Move(CodeAddress, Option<CodeAddress>),
}

impl CompiledMethod {
    fn process_step(&'static self, ctx: ContextRef) {
        let CodeAddress(block, step) = ctx.ip();
        let op = &self.blocks[block].opcode[step];
        let v: Rc<dyn Receiver> = match op {
            Operation::Int(v) => Rc::new(IntReceiver::new(*v)),
            Operation::Invoke(selector, receiver, args) => {
                let receiver = ctx.get_value(receiver);
                let args = ctx.get_values(args.as_slice());
                let r = receiver.receive_message(SelectorSet::get(selector), args);
                r
            }
            Operation::Block(b) => {
                let bctx = BlockContext::new(ctx.clone());
                Rc::new(CompiledBlock::new(self, bctx, *b))
            }
            Operation::Char(v) => Rc::new(CharReceiver::new(*v)),
            Operation::String(v) => Rc::new(StringReceiver::new(v.clone())),
            Operation::Return(addr) => ctx.get_value(addr),
            Operation::Move(from, Some(to)) => {
                let v = ctx.get_value(from);
                ctx.set_value(to, v.clone());
                v
            }
            _ => todo!("{:?}", op),
        };
        {
            let ip = &ctx.ip();
            ctx.set_value(ip, v);
            ctx.next_ip();
        }
    }

    pub fn run(&'static self, ctx: ContextRef) -> Rc<dyn Receiver> {
        while !self.done(ctx.clone()) {
            self.process_step(ctx.clone());
        }
        let CodeAddress(block, _) = ctx.ip();
        ctx.get_value(&self.blocks[block].result.unwrap())
    }

    fn done(&self, ctx: ContextRef) -> bool {
        let CodeAddress(blk, stp) = ctx.ip();
        let b = &self.blocks[blk];
        b.opcode.len() <= stp
    }

    fn get_operation(&self, ip: CodeAddress) -> &Operation {
        let CodeAddress(blk, stp) = ip;
        let b = &self.blocks[blk];
        &b.opcode[stp]
    }
}

impl CompiledBlock {
    pub fn new(method: &'static CompiledMethod, ctx: ContextRef, block: usize) -> Self {
        Self { method, ctx, block }
    }
}

impl Receiver for CompiledBlock {
    fn receive_message(
        &self,
        selector: &'static str,
        args: Vec<Rc<dyn Receiver>>,
    ) -> Rc<dyn Receiver> {
        match selector {
            "value:" => {
                // let mut p = self.ctx.params.lock().unwrap();
                // for x in _args {
                //     p.push(x.clone())
                // }
                self.ctx.call(CodeAddress(self.block, 0));
                while let Operation::Arg(n) = self.method.get_operation(self.ctx.ip()) {
                    self.ctx.set_value(&self.ctx.ip(), args[*n].clone());
                    self.ctx.next_ip();
                }

                self.method.run(self.ctx.clone())
            }
            "value" => {
                // while let Operation::Arg(n) = self.method.get_operation(self.ctx.ip()) {
                //     self.ctx.set_value(&self.ctx.ip(), args[*n].clone());
                //     self.ctx.next_ip();
                // }
                self.ctx.call(CodeAddress(self.block, 0));
                self.method.run(self.ctx.clone())
            }
            s => todo!("doesn't understand {}", s),
        }
    }

    fn as_int(&self) -> Option<isize> {
        todo!()
    }

    fn as_str(&self) -> Option<&'static str> {
        todo!()
    }
}

impl Display for CodeAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "r{}-{}", self.0, self.1)
    }
}

impl Copy for CodeAddress {}

impl ByteCode {
    pub fn new() -> Self {
        Self {
            result: None,
            opcode: vec![],
        }
    }

    fn push(&mut self, op: Operation) -> usize {
        self.opcode.push(op);
        self.opcode.len() - 1
    }

    fn set_result(&mut self, addr: CodeAddress) {
        self.result = Some(addr);
    }

    fn result(&self) -> Option<CodeAddress> {
        self.result
    }

    fn len(&self) -> usize {
        self.opcode.len()
    }
}

impl Display for CompiledMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:15}", "Compiled Method:")?;
        for block_idx in 0..self.blocks.len() {
            writeln!(f, "---")?;
            let b = &self.blocks[block_idx];
            match b.result() {
                Some(addr) => writeln!(f, "{:15} {}", format!("Block {}", block_idx), addr)?,
                None => writeln!(f, "{:15}", format!("Block {}", block_idx))?,
            }
            for idx in 0..b.opcode.len() {
                let addr = CodeAddress(block_idx, idx);
                let name = {
                    match self.names.iter().find(|x| x.1 == addr) {
                        Some((k, _)) => k.as_str(),
                        None => "",
                    }
                };
                writeln!(f, "{:15} {} = {:?}", name, addr, b.opcode[idx])?
            }
        }
        Ok(())
    }
}

impl CompiledMethod {
    pub fn new() -> Self {
        let data = CompiledMethodData {
            current_block: 0,
            blocks: vec![ByteCode::new()],
            stack: vec![],
            names: vec![],
        };
        Self { data }
    }

    pub fn define(&mut self, name: String, idx: CodeAddress) {
        self.names.push((name, idx));
    }

    fn idx_for(&self, name: &str) -> Option<CodeAddress> {
        let r = self.names.iter().find(|x| x.0 == name);
        match r {
            Some((_, v)) => Some(*v),
            None => None,
        }
    }

    pub fn compile(&mut self, ast: &AST) -> CodeAddress {
        match ast {
            AST::Int(v) => self.push(Operation::Int(*v)),
            AST::Char(v) => self.push(Operation::Char(*v)),
            AST::String(v) => self.push(Operation::String(v.to_string())),
            AST::Block { params, body, .. } => {
                let old_block = self.current_block;
                self.current_block = self.blocks.len();
                self.blocks.push(ByteCode::new());

                for param_idx in 0..params.len() {
                    let idx = self.push(Operation::Arg(param_idx));
                    self.define(params[param_idx].to_string(), idx);
                }
                self.compile(body);
                let block = Operation::Block(self.current_block);
                {
                    let block_num = self.current_block;
                    let pos = self.len();
                    if let Some(addr) = self.stack.pop() {
                        let block = &mut self.blocks[block_num];
                        block.set_result(addr);
                    } else {
                        let addr = CodeAddress(block_num, pos - 1);
                        let block = &mut self.blocks[block_num];
                        block.set_result(addr);
                    }
                }
                self.current_block = old_block;
                let result_idx = self.push(block);
                result_idx
            }
            AST::Return(x) => {
                let idx = self.compile(x);
                self.push(Operation::Return(idx))
            }
            AST::Statements(s) => {
                let mut n = CodeAddress(0, 0);
                for stmt in s {
                    n = self.compile(stmt);
                }
                n
            }
            AST::InvokeSequence(a, b) => {
                let mut n = self.compile(a);
                self.stack.push(n);
                for x in b {
                    n = self.compile(x);
                    self.stack.push(n);
                }
                let block_num = self.current_block;
                self.blocks[block_num].result = Some(n);
                n
            }
            AST::Message { name, args } => {
                if let Some(receiver_idx) = self.stack.pop() {
                    let mut argv = vec![];

                    for x in args {
                        let n = self.compile(x);
                        argv.push(n);
                    }
                    let c = Operation::Invoke(name.to_string(), receiver_idx, argv);
                    self.push(c)
                } else {
                    panic!()
                }
            }
            AST::Variable(name) => match self.idx_for(*name) {
                Some(idx) => idx,
                None => {
                    let idx = self.push(Operation::Global(name.to_string()));
                    self.define(name.to_string(), idx);
                    idx
                }
            },
            AST::Assign(namet, v) => {
                if let AST::Name(name) = **namet {
                    let idx = self.compile(v);
                    if idx.0 > 0 {
                        self.push(Operation::Move(idx, self.idx_for(name)));
                    } else {
                        self.define(name.to_string(), idx);
                    }
                    idx
                } else {
                    panic!()
                }
            }
            _ => todo!(),
        }
    }

    pub fn push(&mut self, op: Operation) -> CodeAddress {
        let block_num = self.current_block;
        let b = &mut self.blocks[block_num];
        CodeAddress(block_num, b.push(op))
    }

    pub fn len(&self) -> usize {
        self.blocks[self.current_block].len()
    }
}

impl Default for CompiledMethod {
    fn default() -> Self {
        Self::new()
    }
}

pub fn compile_script(
    input_string: String,
) -> Result<&'static CompiledMethod, Box<dyn std::error::Error>> {
    let parse_trees = parse_script(input_string)?;
    let mut o = CompiledMethod::new();
    for t in parse_trees {
        let ast = t.as_abstract_syntax_tree();
        println!("-> {}", t.to_string());
        println!("-> {:#?}", &ast);
        o.compile(&ast);
    }
    let l0 = Box::new(o);
    let l = Box::leak(l0);
    Ok(l)
}
