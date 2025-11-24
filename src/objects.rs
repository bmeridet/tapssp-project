use crate::block::Block;
use core::fmt;
use std::fmt::Display;
use std::rc::Rc;

use crate::vm::VM;
use crate::value::Value;

pub enum ObjectType {
    LoxString,
    Function,
    Native,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LoxString {
    pub value: String,
    pub hash: usize,
}

impl Display for LoxString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl LoxString {
    pub fn new(value: &str) -> Rc<LoxString> {
        let hash = LoxString::hash(&value);
        let s = LoxString { 
            value: value.to_string(), 
            hash,
        };

        Rc::new(s)
    }

    pub fn from_string(s: &str) -> Rc<LoxString> {
        LoxString::new(s)
    }

    fn hash(s: &str) -> usize {
        let mut hash = 2166136261usize;
        for c in s.chars() {
            hash ^= c as usize;
            hash = hash.wrapping_mul(16777619);
        }
        hash
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: Rc<LoxString>,
    pub block: Block,
    pub arity: usize,
}

impl Function {
    pub fn new(function_name: Rc<LoxString>) -> Box<Function> {
        let f = Function {
            name: function_name,
            block: Block::new(),
            arity: 0,
        };

        Box::new(f)
    }
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.name.value == "script" {
            write!(f, "<script>")
        } else {
            write!(f, "<fn {}>", self.name)
        }
    }
}

#[derive(Clone, Copy)]
pub struct NativeFunction (
    pub fn(&VM, &[Value]) -> Value
);

impl fmt::Debug for NativeFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<native fn>")
    }
}

impl PartialEq for NativeFunction {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&self, &other)
    }
}