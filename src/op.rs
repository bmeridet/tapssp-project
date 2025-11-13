#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum OpCode {
    Constant = 0,
    Nil = 1,
    True = 2,
    False = 3,
    Pop = 4,
    GetGlobal = 5,
    DefGlobal = 6,
    SetGlobal = 7,
    Equal = 8,
    Greater = 9,
    Less = 10,
    Add = 11,
    Subtract = 12,
    Multiply = 13,
    Divide = 14,
    Not = 15,
    Negate = 16,
    Print = 17,
    Return = 18,
}

impl From<u8> for OpCode {
    fn from(value: u8) -> Self {
        match value {
            0 => OpCode::Constant,
            1 => OpCode::Nil,
            2 => OpCode::True,
            3 => OpCode::False,
            4 => OpCode::Pop,
            5 => OpCode::GetGlobal,
            6 => OpCode::DefGlobal,
            7 => OpCode::SetGlobal,
            8 => OpCode::Equal,
            9 => OpCode::Greater,
            10 => OpCode::Less,
            11 => OpCode::Add,
            12 => OpCode::Subtract,
            13 => OpCode::Multiply,
            14 => OpCode::Divide,
            15 => OpCode::Not,
            16 => OpCode::Negate,
            17 => OpCode::Print,
            18 => OpCode::Return,
            _ => panic!("Unknown opcode: {}", value),
        }
    }
}