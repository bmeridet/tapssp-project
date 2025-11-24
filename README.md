# TAPSSP Project — A Lox Interpreter in Rust

This project implements a **Lox interpreter** in Rust using a **bytecode virtual machine**, a **hand-written scanner**, and a **recursive descent parser/compiler**, inspired by *Crafting Interpreters*.  
It compiles Lox source code into bytecode, handling variables, functions, control flow, and expressions.

---

## Overview

The interpreter consists of:

- **Scanner** — Tokenizes raw source text into `Token`s.  
- **Parser/Compiler** — Parses tokens according to Lox grammar, resolves variables, and emits bytecode instructions into a `Block`.  
- **Block** — Stores bytecode (`OpCode`s), constants, and line numbers.  
- **Objects** — Runtime entities like `Function` and `LoxString`.  
- **VM** — Executes bytecode on a stack-based architecture with call frames.

---

## Value System

Runtime values (`Value`) used by the VM include:

```rust
pub enum Value {
    Number(f64),
    Bool(bool),
    String(Rc<LoxString>),
    Function(Rc<Function>),
    NativeFunction(NativeFunction),
    Nil,
}

- Stored on the VM stack and in constants in `Block`.  
- Includes numbers, booleans, strings, functions, and nil.  
- Functions are heap-allocated via `Rc<Function>` to allow multiple references.

## Tokens and Scanner

`Token` represents individual lexical units:

- `Identifier`, `Number`, `String`  
- Symbols: `+`, `-`, `*`, `/`, `=`, `==`, `!=`  
- Keywords: `var`, `fun`, `if`, `else`, `while`, `for`, `print`, `return`, `true`, `false`, `nil`, `and`, `or`  
- Special tokens: `Eof` and `Error`  

The `Scanner`:

- Iterates over the source string, producing `Token`s  
- Supports multi-character operators  
- Tracks line numbers  
- Returns error tokens for invalid input

---

## Parser and Compiler

The parser is integrated with the compiler to produce bytecode in **one pass**.

### Parser

- Drives compilation by consuming tokens from the `Scanner`.  
- Implements **recursive descent parsing** with operator precedence (`Precedence` enum).  
- Handles statements (`print`, `var`, `fun`, `return`, `if`, `while`, `for`) and expressions (`+`, `-`, `*`, `/`, `==`, `!=`, `<`, `>`, etc.).  
- Emits instructions via the compiler into the current function’s `Block`.  
- Supports **local variables** and **nested scopes**, resolving them to bytecode via `OpCode::GetLocal` and `OpCode::SetLocal`.  
- Emits `OpCode::GetGlobal` / `OpCode::SetGlobal` for global variables.

### Compiler

- Maintains the current function being compiled (`Function`), a stack of `Local` variables, and the scope depth.  
- Supports nested functions (`compiler_push` / `compiler_pop`) for proper closure compilation.  
- Adds constants to the current function’s `Block` via `add_constant`.  
- Emits control flow instructions: `Jump`, `JumpIfFalse`, `Loop` for loops and conditionals.  
- Ensures assignment and initializer rules: local variables cannot be read before initialization.  
- Supports functions with parameter limits (max 255 parameters).  

---

## Block (Bytecode Representation)

`Block` represents a **chunk of bytecode**:

```rust
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

- Holds bytecode instructions, constants, and source line numbers.  
- `write` appends an instruction and tracks its line number.  
- `add_constant` appends values to the constant table for instructions to reference.  

---

## Objects

### LoxString

- Used for all string literals and variable names.  
- Stored via `Rc<LoxString>` for shared ownership.  

### Function

- Contains a `Block` of bytecode, function name, arity, and scope info.  
- Each `Function` is reference-counted (`Rc`) so multiple closures can share the same function object.  

### NativeFunction

- Wraps Rust closures callable from Lox code.  
- Used for implementing built-in functions (e.g., `clock`, `print`).  

---

## Virtual Machine

The VM executes instructions from a `Block`:

- Maintains a **stack of `Value`s**  
- Supports **call frames** for function calls  
- Implements all `OpCode`s defined in the `op` module, including:

  - Stack operations: `Pop`, `Constant`  
  - Arithmetic: `Add`, `Subtract`, `Multiply`, `Divide`, `Negate`  
  - Comparison: `Equal`, `Greater`, `Less`  
  - Boolean: `True`, `False`, `Not`  
  - Control flow: `Jump`, `JumpIfFalse`, `Loop`  
  - Function calls: `Call`, `Return`  
  - Variable access: `GetLocal`, `SetLocal`, `GetGlobal`, `SetGlobal`, `DefGlobal`

Execution model:

1. Fetch instruction from `Block.code`  
2. Decode operands  
3. Manipulate stack as instructed  
4. Jump or call functions when needed  

---

## Compilation Workflow

1. **Scan source** → Tokens via `Scanner`  
2. **Parse & compile** → Generate bytecode in a `Block` via `Parser`  
3. **Emit constants** → Stored in `Block.constants`  
4. **Emit instructions** → Stored in `Block.code`  
5. **Function calls** → Create `Function` objects containing their own `Block`  
6. **VM execution** → Stack-based execution of the bytecode

Compiler features:

- Local variable resolution  
- Nested scopes  
- Assignments, initializers, and return statements  
- Control flow (`if`, `while`, `for`, `and`, `or`)  

---

## Running the Interpreter

Compile a script and execute:

```rust
let source = "var x = 10; print x;";
vm.interpret(source);

- 'interpret' initiates the pipeline of scanning, parsing / compiling, and then executing the provided source code.