use std::{clone, ptr::null};

use crate::{
    block::Block, compiler::compile, debug::disassemble_instruction, error::LoxError, op::OpCode, value::Value, objects::LoxString, table::Table
};

pub struct VM {
    pub block: Block,
    pub ip: usize,
    pub stack: Vec<Value>,
    pub strings: Table,
    pub globals: Table,
}

impl VM {
    pub fn new() -> VM {
        VM {
            block: Block::new(),
            ip: 0,
            stack: Vec::with_capacity(256),
            strings: Table::new(),
            globals: Table::new(),
        }
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().expect("stack underflow")
    }

    fn peek(&self, n: usize) -> Value {
        self.stack.get(self.stack.len() - n - 1).expect("stack underflow").clone()
    }

    fn binary_op<T>(&mut self, op: fn(f64, f64) -> T, f: fn(T) -> Value) -> Result<(), String> {
        let b = self.pop();
        let a = self.pop();
        
        match (a.as_number(), b.as_number()) {
            (Some(aa), Some(bb)) => {
                self.push(f(op(aa, bb)));
                Ok(())
            }
            _ => Err("Operands must be numbers".to_string()),
        }
    }

    fn read_byte(&mut self) -> u8 {
        let byte = self.block.code[self.ip];
        self.ip += 1;
        byte
    }

    pub fn interpret(&mut self, source: &str) -> Result<(), LoxError> {
        self.ip = 0;
        self.block = compile(source)?;
        self.run()
    }

    fn run(&mut self) -> Result<(), LoxError> {
        loop {
            if self.ip >= self.block.code.len() {
                return Err(LoxError::RuntimeError("Hit end of bytecode".to_string()));
            }

            let op = OpCode::from(self.read_byte());

            #[cfg(debug_assertions)]
            {
                print!("stack -> ");
                for value in &self.stack {
                    print!("[ {:?} ] ", value);
                }
                println!();
                disassemble_instruction(&self.block, self.ip - 1);
            }

            match op {
                OpCode::Constant => {
                    let index = self.read_byte() as usize;
                    let value = self.block.constants[index].clone();
                    self.push(value);
                },
                OpCode::Nil => self.push(Value::Nil),
                OpCode::True => self.push(Value::Bool(true)),
                OpCode::False => self.push(Value::Bool(false)),
                OpCode::Pop => {
                    self.pop();
                },
                OpCode::GetGlobal => {
                    let index = self.read_byte();
                    let s = self.block.read_string(index).clone();
                    if let Some(v) = self.globals.get(&s) {
                        self.push(v);
                    } else {
                        return Err(LoxError::RuntimeError(format!("Undefined variable '{}'", s.value)));
                    }
                },
                OpCode::DefGlobal => {
                    let index = self.read_byte();
                    let s = self.block.read_string(index).clone();
                    let value = self.pop();
                    self.globals.set(s, value);
                },
                OpCode::SetGlobal => {
                    let index = self.read_byte();
                    let s = self.block.read_string(index);

                    if self.globals.set(s.clone(), self.peek(0)) {
                        self.globals.delete(s);
                        return Err(LoxError::RuntimeError(format!("Undefined variable '{}'", s.value)));
                    }
                },
                OpCode::Equal => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push(Value::Bool(a == b));
                },
                OpCode::Greater => {
                    if let Err(msg) = self.binary_op(|a, b| a > b, Value::Bool) {
                        return Err(LoxError::RuntimeError(msg));
                    }
                },
                OpCode::Less => {
                    if let Err(msg) = self.binary_op(|a, b| a < b, Value::Bool) {
                        return Err(LoxError::RuntimeError(msg));
                    }
                }
                OpCode::Add => {
                    let (b, a) = (self.pop(), self.pop());

                    match (&a, &b) {
                        (Value::Number(a), Value::Number(b)) => self.push(Value::Number(a + b)),
                        (Value::String(a), Value::String(b)) => {
                            let result = format!("{}{}", a.value, b.value);
                            self.push(Value::String(LoxString::new(&result)))
                        }
                        _ => return Err(LoxError::RuntimeError("Operands must be two numbers or two strings".to_string())),
                    }
                },
                OpCode::Subtract => {
                    if let Err(msg) = self.binary_op(|a, b| a - b, Value::Number) {
                        return Err(LoxError::RuntimeError(msg));
                    }
                },
                OpCode::Multiply => {
                    if let Err(msg) = self.binary_op(|a, b| a * b, Value::Number) {
                        return Err(LoxError::RuntimeError(msg));
                    }
                },
                OpCode::Divide => {
                    if let Err(msg) = self.binary_op(|a, b| a / b, Value::Number) {
                        return Err(LoxError::RuntimeError(msg));
                    }
                },
                OpCode::Not => {
                    let val = self.pop();
                    self.push(Value::Bool(val.is_falsey()));
                }
                OpCode::Negate => {
                    match self.pop().as_number() {
                        Some(num) => self.push(Value::Number(-num)),
                        None => return Err(LoxError::RuntimeError("Operand must be a number".to_string())),
                    }
                },
                OpCode::Print => {
                    println!("{}", self.pop());
                }
                OpCode::Return => {
                    return Ok(());
                }
            }
        }
    }
}