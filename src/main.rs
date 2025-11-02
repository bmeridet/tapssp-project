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
use std::io::{stdin, stdout, Write};

fn repl() {
    let mut line = String::new();

    loop {
        print!("> ");
        stdout().flush().unwrap();

        line.clear();
        let bytes = stdin().read_line(&mut line).unwrap();
        if bytes == 0 {
            break;
        }

        let input = line.trim();
        if input.eq_ignore_ascii_case("quit") || input.eq_ignore_ascii_case("exit") {
            break;
        }

        match VM::interpret(input) {
            Err(e) => println!("{:?}", e),
            Ok(value) => println!("{:?}", value),
        }
    }

    println!("Exiting.");
}

fn main() {
    repl();
}
