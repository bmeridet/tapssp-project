mod op;
mod debug;
mod value;
mod block;
mod vm;

use op::OpCode;
use block::Block;
use value::Value;
use vm::{VM, RunResult};

fn main() {
    let mut block = Block::new();

    let idx1 = block.add_constant(Value::Number(1.2));
    let idx2 = block.add_constant(Value::Number(3.4));
    let idx3 = block.add_constant(Value::Number(5.6));

    block.write(OpCode::Constant as u8, 1);
    block.write(idx1 as u8, 1);

    block.write(OpCode::Constant as u8, 1);
    block.write(idx2 as u8, 1);

    block.write(OpCode::Add as u8, 1);

    block.write(OpCode::Constant as u8, 1);
    block.write(idx3 as u8, 1);

    block.write(OpCode::Divide as u8, 1);
    block.write(OpCode::Negate as u8, 1);

    block.write(OpCode::Return as u8, 1);

    let mut vm = VM::new(block);
    match vm.run() {
        RunResult::Ok => println!("Program finished successfully."),
        RunResult::Error(msg) => println!("Runtime error: {}", msg),
    }
}
