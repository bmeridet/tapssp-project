use crate::{objects::LoxString, value::Value, op::OpCode};
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub code: Vec<OpCode>,
    pub constants: Vec<Value>,
    pub lines: Vec<u16>,
}

impl Block {
    pub fn new() -> Block {
        Block {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
        }
    }

    pub fn write(&mut self, byte: OpCode, line: u16) -> usize{
        self.code.push(byte);
        self.lines.push(line);
        self.code.len() - 1
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    pub fn read_constant(&self, index: u8) -> &Value {
        &self.constants[index as usize]
    }

    pub fn read_string(&self, index: u8) -> Rc<LoxString> {
        if let Value::String(s) = self.read_constant(index) {
            s.clone()
        } else {
            panic!("Not a string");
        }
    }
}