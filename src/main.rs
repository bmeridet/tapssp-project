mod op;
mod debug;
mod value;
mod block;
mod vm;
mod token;
mod scanner;
mod compiler;
mod error;

use vm::{VM};

fn main() {
    let src = "1 + 2";

    match VM::interpret(src) {
        Err(e) => println!("{:?}", e),
        Ok(value) => println!("{:?}", value),
    }
}
