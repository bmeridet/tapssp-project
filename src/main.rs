mod op;
mod value;
mod block;
mod vm;
mod token;
mod scanner;
mod compiler;
mod error;
mod table;
mod objects;

use vm::{VM};
use std::io::{stdin, stdout, Write};
use std::fs;

fn repl() {
    let mut line = String::new();

    let mut vm = VM::new();

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

        match vm.interpret(input) {
            Err(e) => println!("{:?}", e),
            Ok(value) => println!("{:?}", value),
        }
    }

    println!("Exiting.");
}

fn run_file(filename: &str) {
    let source = fs::read_to_string(filename)
        .expect("Could not read file");

    let mut vm = VM::new();

    match vm.interpret(&source) {
        Err(e) => println!("{:?}", e),
        Ok(value) => println!("{:?}", value),
    }
}

fn main() {
    if let Some(arg) = std::env::args().nth(1) {
        run_file(&arg);
    } else {
        repl();
    }
}
