use crate::{block::Block, value::Value, op::OpCode, debug::disassemble_instruction};

#[derive(Debug)]
pub enum RunResult {
    Ok(Value),
    Error(String),
}

pub struct VM {
    pub block: Block,
    pub ip: usize,
    pub stack: Vec<Value>,
}

impl VM {
    pub fn new(block: Block) -> VM {
        VM {
            block,
            ip: 0,
            stack: Vec::with_capacity(256),
        }
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().expect("stack underflow")
    }

    fn binary_op<F>(&mut self, op: F) -> Result<(), String> where F: FnOnce(f64, f64) -> f64 {
        let b = self.pop();
        let a = self.pop();
        
        match (a.as_number(), b.as_number()) {
            (Some(aa), Some(bb)) => {
                self.push(Value::Number(op(aa, bb)));
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

    pub fn run(&mut self) -> RunResult {
        loop {
            if self.ip >= self.block.code.len() {
                return RunResult::Error("Reached end of code".to_string());
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
                OpCode::Add => {
                    if let Err(msg) = self.binary_op(|a, b| a + b) {
                        return RunResult::Error(format!("Add:{}", msg));
                    }
                },
                OpCode::Subtract => {
                    if let Err(msg) = self.binary_op(|a, b| a - b) {
                        return RunResult::Error(format!("Subtract:{}", msg));
                    }
                },
                OpCode::Multiply => {
                    if let Err(msg) = self.binary_op(|a, b| a * b) {
                        return RunResult::Error(format!("Multiply:{}", msg));
                    }
                },
                OpCode::Divide => {
                    if let Err(msg) = self.binary_op(|a, b| a / b) {
                        return RunResult::Error(format!("Divide:{}", msg));
                    }
                },
                OpCode::Negate => {
                    match self.pop().as_number() {
                        Some(num) => self.push(Value::Number(-num)),
                        None => return RunResult::Error("Negate:Operand must be a number".to_string()),
                    }
                },
                OpCode::Return => {
                    let value = self.pop();
                    println!("===> {:?}", value);
                    return RunResult::Ok(value);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_block() -> Block {
        let mut block = Block::new();

        let idx1 = block.add_constant(Value::Number(3.0));
        let idx2 = block.add_constant(Value::Number(4.0));

        block.write(OpCode::Constant as u8, 1);
        block.write(idx1 as u8, 1);

        block.write(OpCode::Constant as u8, 1);
        block.write(idx2 as u8, 1);

        block
    }

    #[test]
    fn test_vm_arithmetic() {
        let mut block = load_block();

        block.write(OpCode::Add as u8, 1);
        block.write(OpCode::Return as u8, 1);

        let mut vm = VM::new(block);

        match vm.run() {
            RunResult::Error(msg) => panic!("VM run failed: {}", msg),
            RunResult::Ok(value) => assert_eq!(value.as_number(), Some(7.0)),
        }
    }

    #[test]
    fn test_vm_negate() {
        let mut block = load_block();

        block.write(OpCode::Negate as u8, 1);
        block.write(OpCode::Return as u8, 1);

        let mut vm = VM::new(block);

        match vm.run() {
            RunResult::Error(msg) => panic!("VM run failed: {}", msg),
            RunResult::Ok(value) => assert_eq!(value.as_number(), Some(-4.0)),
        }
    }

    #[test]
    fn test_vm_subtract() {
        let mut block = load_block();

        block.write(OpCode::Subtract as u8, 1);
        block.write(OpCode::Return as u8, 1);

        let mut vm = VM::new(block);

        match vm.run() {
            RunResult::Error(msg) => panic!("VM run failed: {}", msg),
            RunResult::Ok(value) => assert_eq!(value.as_number(), Some(-1.0)),
        }
    }

    #[test]
    fn test_vm_multiply() {
        let mut block = load_block();

        block.write(OpCode::Multiply as u8, 1);
        block.write(OpCode::Return as u8, 1);

        let mut vm = VM::new(block);

        match vm.run() {
            RunResult::Error(msg) => panic!("VM run failed: {}", msg),
            RunResult::Ok(value) => assert_eq!(value.as_number(), Some(12.0)),
        }
    }

    #[test]
    fn test_vm_divide() {
        let mut block = load_block();

        block.write(OpCode::Divide as u8, 1);
        block.write(OpCode::Return as u8, 1);

        let mut vm = VM::new(block);

        match vm.run() {
            RunResult::Error(msg) => panic!("VM run failed: {}", msg),
            RunResult::Ok(value) => assert_eq!(value.as_number(), Some(0.75)),
        }
    }
}