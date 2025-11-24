use cpu_time::ProcessTime;
use std::{ptr::null_mut};
use std::rc::Rc;
use crate::{
    block::Block, compiler::compile, error::LoxError, op::OpCode, value::Value, objects::{LoxString, Function, NativeFunction}, table::Table
};

#[derive(Clone, Debug)]
struct CallFrame {
    function: Option<Rc<Function>>,
    ip: *const OpCode,
    slots: usize,
}

impl CallFrame {
    fn new(function: Rc<Function>, slot: usize) -> CallFrame {
        let mut cf = CallFrame {
            function: Some(function),
            ip: null_mut(),
            slots: slot,
        };

        cf.ip = cf.function.as_ref().unwrap().block.code.as_ptr();

        cf
    }

    fn dangling() -> CallFrame {
        CallFrame {
            function: None,
            ip: null_mut(),
            slots: 0,
        }
    }
}

pub struct VM {
    frames: [CallFrame; VM::MAX_FRAMES],
    frame_count: usize,
    stack: [Value; VM::MAX_STACK],
    stack_top: usize,
    strings: Table,
    globals: Table,
    init_time: ProcessTime,
}

impl VM {
    const MAX_FRAMES: usize = 64;
    const MAX_STACK: usize = Self::MAX_FRAMES * u8::MAX as usize;

    pub fn new() -> VM {
        let mut vm =VM {
            frames: std::array::from_fn(|_| CallFrame::dangling()),
            frame_count: 0,
            stack: std::array::from_fn(|_| Value::Nil),
            stack_top: 0,
            strings: Table::new(),
            globals: Table::new(),
            init_time: ProcessTime::now(),
        };

        vm.init_vm();

        vm
    }

    fn init_vm(&mut self) {
        self.define_native("clock", NativeFunction(clock));
    }

    fn push(&mut self, value: Value) {
        self.stack[self.stack_top] = value;
        self.stack_top += 1;
    }

    fn pop(&mut self) -> Value {
        self.stack_top -= 1;
        self.stack[self.stack_top].clone()
    }

    fn peek(&self, n: usize) -> Value {
        self.stack[self.stack_top - 1 - n].clone()
    }

    fn reset_stack(&mut self) {
        self.stack_top = 0;
        self.frame_count = 0;
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

    pub fn interpret(&mut self, source: &str) -> Result<(), LoxError> {
        let function = compile(source)?;
        self.push(Value::Function(function.clone()));
        self.call(function.clone(), 0)?;
        self.run()
    }

    fn run(&mut self) -> Result<(), LoxError> {
        let mut current_frame = unsafe { &mut *(&mut self.frames[self.frame_count - 1] as *mut CallFrame) };
        let mut current_block = &current_frame.function.as_ref().unwrap().block;

        loop {
            let op = unsafe { *current_frame.ip };

            #[cfg(feature = "debug_trace")]
            {
                let offset = unsafe { current_frame.ip.offset_from(current_block.code.as_ptr()) as usize };
                print!("stack -> ");
                for i in 0..self.stack_top {
                    print!("[{}] ", self.stack[i]);
                }
                println!();
                self.disassemble_instruction(&current_frame, &current_block, offset);
            }

            current_frame.ip = unsafe { current_frame.ip.offset(1) };

            match op {
                OpCode::Constant(index) => {
                    let value = current_block.read_constant(index);
                    self.push(value.clone());
                },
                OpCode::Nil => self.push(Value::Nil),
                OpCode::True => self.push(Value::Bool(true)),
                OpCode::False => self.push(Value::Bool(false)),
                OpCode::Pop => {
                    self.pop();
                },
                OpCode::GetLocal(index) => {
                    let index = current_frame.slots + index as usize;
                    let val = self.stack[index].clone();
                    self.push(val);
                },
                OpCode::SetLocal(index) => {
                    let index = current_frame.slots + index as usize;
                    self.stack[index] = self.peek(0);
                },
                OpCode::GetGlobal(index) => {
                    let s = current_block.read_string(index).clone();
                    if let Some(v) = self.globals.get(s.clone()) {
                        self.push(v);
                    } else {
                        return Err(LoxError::RuntimeError(format!("Undefined variable '{}'", s.value)));
                    }
                },
                OpCode::DefGlobal(index) => {
                    let s = current_block.read_string(index).clone();
                    let value = self.pop();
                    self.globals.set(s, value);
                },
                OpCode::SetGlobal(index) => {
                    let s = current_block.read_string(index);

                    if self.globals.set(s.clone(), self.peek(0)) {
                        self.globals.delete(s.clone());
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
                },
                OpCode::Jump(offset) => {
                    current_frame.ip = unsafe { current_frame.ip.offset(offset as isize) };
                }
                OpCode::JumpIfFalse(offset) => {
                    if self.peek(0).is_falsey() {
                        current_frame.ip = unsafe { current_frame.ip.offset(offset as isize) };
                    }
                },
                OpCode::Loop(offset) => {
                    current_frame.ip = unsafe { current_frame.ip.offset(-1 - (offset as isize)) };
                },
                OpCode::Call(arg_count) => {
                    self.call_value(arg_count as usize)?;
                    current_frame = unsafe { &mut *(&mut self.frames[self.frame_count - 1] as *mut CallFrame) };
                    current_block = &current_frame.function.as_ref().unwrap().block;
                },
                OpCode::Return => {
                    let result = self.pop();
                    self.frame_count -= 1;

                    if self.frame_count == 0 {
                        self.pop();
                        return Ok(());
                    } else {
                        self.stack_top = current_frame.slots;
                        self.push(result);

                        current_frame = unsafe { &mut *(&mut self.frames[self.frame_count - 1] as *mut CallFrame) };
                        current_block = &current_frame.function.as_ref().unwrap().block;
                    }
                },
            }
        }
    }

    fn call_value(&mut self, arg_count:usize) -> Result<(), LoxError> {
        let callee = &self.peek(arg_count);

        match callee {
            Value::Function(f) => self.call(f.clone(), arg_count),
            Value::NativeFunction(nf) => {
                let start = self.stack_top - arg_count;
                let result = nf.0(self, &self.stack[start..self.stack_top]);
                self.stack_top -= arg_count + 1;
                self.push(result);
                Ok(())
            },
            _ => Err(LoxError::RuntimeError("Can only call functions".to_string())),
        }
    }

    fn call(&mut self, function: Rc<Function>, arg_count: usize) -> Result<(), LoxError> {
        if function.arity != arg_count {
            self.stack_trace();
            Err(LoxError::RuntimeError(format!("Expected {} arguments but got {}", function.arity, arg_count)))
        } else if self.frame_count == VM::MAX_FRAMES {
            Err(LoxError::RuntimeError("Stack overflow".to_string()))
        } else {
            let frame = CallFrame::new(function, self.stack_top - arg_count - 1);
            self.frames[self.frame_count] = frame;
            self.frame_count += 1;
            Ok(())
        }
    }

    fn define_native(&mut self, name: &str, function: NativeFunction) {
        let name = LoxString::from_string(name);
        self.globals.set(name, Value::NativeFunction(function));
    }

    fn stack_trace(&self) {
        for i in (0..self.frame_count).rev() {
            let frame = &self.frames[i];
            let function = frame.function.as_ref().unwrap();
            let offset = unsafe { frame.ip.offset_from(function.block.code.as_ptr()) as usize - 1 };
            println!("[line {}] in {}", function.block.lines[offset], function.name);
        }
    }

    fn display_jump(&self, block: &Block, instruction: OpCode, offset: usize) {
        match instruction {
            OpCode::Jump(jump) | OpCode::JumpIfFalse(jump) => {
                let jump = offset.checked_add_signed(jump as isize).unwrap();
                println!("{:04} {:?} JUMP_TO: {:04} {:?}", offset, instruction, jump, block.code[jump]);
            },
            OpCode::Loop(jump) => {
                let jump = offset.checked_add_signed(-1 - (jump as isize)).unwrap();
                println!("{:04} {:?} JUMP_TO: {:04} {:?}", offset, instruction, jump, block.code[jump]);
            },
            _ => panic!("Not a jump instruction"),
        }
    }

    fn disassemble_instruction(&self, frame: &CallFrame, block: &Block, offset: usize) {
        let line = block.lines[offset];

        if offset > 0 && line == block.lines[offset - 1] {
            print!("   | ");
        } else {
            print!("{:04} ", line);
        }

        let instruction = block.code[offset];

        match instruction {
            OpCode::Constant(index) => {
                println!("{:04} {:?} IDX: {:4} '{:?}'", offset, instruction, index, block.read_constant(index));           
            },
            OpCode::Jump(_) | OpCode::JumpIfFalse(_) | OpCode::Loop(_) => {
                self.display_jump(block, instruction, offset);
            },
            OpCode::DefGlobal(index) => {
                let name = block.read_string(index);
                println!("{:04} {:?} IDX: {:4} '{}' = '{:?}'", offset, instruction, index, name.value, self.peek(0));
            },
            OpCode::SetLocal(index) => {
                println!("{:04} {:?} IDX: {:4} = '{:?}'", offset, instruction, index, self.peek(0));
            },
            OpCode::GetLocal(index) => {
                let index = frame.slots + index as usize;
                println!("{:04} {:?} IDX: {:4} = '{:?}'", offset, instruction, index, self.stack[index]);
            },
            OpCode::SetGlobal(index) => {
                let name = block.read_string(index);
                println!("{:04} {:?} IDX: {:4} '{}' = '{:?}'", offset, instruction, index, name.value, self.peek(0));
            },
            OpCode::GetGlobal(index) => {
                let name = block.read_string(index);
                println!("{:04} {:?} IDX: {:4} '{}' = '{:?}'", offset, instruction, index, name.value, self.globals.get(name.clone()));
            },
            OpCode::Call(arg_count) => {
                println!("{:04} {:?} ARGS: {}", offset, instruction, arg_count);
            },
            _ => {
                println!("{:04} {:?}", offset, instruction);
            }
        }
    }
}

fn clock(vm: &VM, _args: &[Value]) -> Value {
    let elapsed = vm.init_time.elapsed().as_secs_f64();
    Value::Number(elapsed)
}