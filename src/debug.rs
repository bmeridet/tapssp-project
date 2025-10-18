use crate::{block::Block, op::OpCode};

pub fn disassemble_chunk(chunk: &Block, name: &str) {
    println!("== {} ==", name);
    let mut offset = 0usize;
    while offset < chunk.code.len() {
        offset = disassemble_instruction(chunk, offset);
    }
}

pub fn disassemble_instruction(chunk: &Block, offset: usize) -> usize {
    print!("{:04} ", offset);
    
    if offset > 0 && chunk.lines[offset] == chunk.lines[offset - 1] {
        print!("   | ");
    } else {
        print!("{:4} ", chunk.lines[offset]);
    }

    let instruction = chunk.code[offset];
    let op = OpCode::from(instruction);
    
    match op {
        OpCode::Constant => {
            let index = chunk.code[offset + 1] as usize;
            println!("{:?} {:4} {:?}", op, index, chunk.constants[index]);
            offset + 2
        },
        _ => {
            println!("{:?}", op);
            offset + 1
        }
    }
}