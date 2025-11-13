use crate::{objects::LoxString, value::Value};

pub struct Block {
    pub code: Vec<u8>,
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

    pub fn write(&mut self, byte: u8, line: u16) {
        self.code.push(byte);
        self.lines.push(line);
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    pub fn read_constant(&self, index: u8) -> &Value {
        &self.constants[index as usize]
    }

    pub fn read_string(&self, index: u8) -> &LoxString {
        if let Value::String(s) = self.read_constant(index) {
            s
        } else {
            panic!("Not a string");
        }
    }
}