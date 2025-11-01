
use crate::{
    block::{Block}, scanner::{ScanError, Scanner}, token::{Token, TokenType}, value::Value, error::LoxError, op::OpCode, debug::disassemble_block
};

pub fn compile(source: &str) -> Result<Block, LoxError> {
    let mut parser = Parser::new(source);
    parser.compile()
}

#[derive(PartialEq, PartialOrd, Clone, Copy)]
enum Precedence {
    None,
    Assignment,
    Or,
    And,
    Equality,
    Comparison,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
}

impl Precedence {
    fn next(self) -> Precedence {
        match self {
            Precedence::None => Precedence::Assignment,
            Precedence::Assignment => Precedence::Or,
            Precedence::Or => Precedence::And,
            Precedence::And => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::Term,
            Precedence::Term => Precedence::Factor,
            Precedence::Factor => Precedence::Unary,
            Precedence::Unary => Precedence::Call,
            Precedence::Call => Precedence::Primary,
            Precedence::Primary => Precedence::Primary,
        }
    }
}

type ParseFn<'a> = fn(&mut Parser<'a>, &mut Block) -> ();

struct ParseRule<'a> {
    prefix: Option<ParseFn<'a>>,
    infix: Option<ParseFn<'a>>,
    precedence: Precedence,
}

impl<'a> ParseRule<'a> {
    fn new(prefix: Option<ParseFn<'a>>, infix: Option<ParseFn<'a>>, precedence: Precedence) -> Self {
        ParseRule { prefix, infix, precedence }
    }
}

struct Parser<'a> {
    scanner: Scanner<'a>,
    current: Token<'a>,
    previous: Token<'a>,
    rules: Vec<ParseRule<'a>>,
    had_error: bool,
    panic_mode: bool,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str) -> Self {
        let scanner = Scanner::new(source);

        let mut parser = Parser {
            scanner,
            current: Token::default(),
            previous: Token::default(),
            rules: Vec::with_capacity(40),
            had_error: false,
            panic_mode: false
        };

        parser.add_rule(Some(Parser::grouping), None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(Some(Parser::unary), Some(Parser::binary), Precedence::Term);
        parser.add_rule(None, Some(Parser::binary), Precedence::Term);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, Some(Parser::binary), Precedence::Factor);
        parser.add_rule(None, Some(Parser::binary), Precedence::Factor);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(Some(Parser::number), None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);
        parser.add_rule(None, None, Precedence::None);

        parser
    }

    pub fn compile(&mut self) -> Result<Block, LoxError> {
        let mut block = Block::new();

        self.advance();
        self.expression(&mut block);
        self.emit_return(&mut block);

        #[cfg(debug_assertions)]
        {
            disassemble_block(&block, "code");
        }

        if self.had_error {
            Err(LoxError::CompileError)
        } else {
            Ok(block)
        }
    }

    pub fn expression(&mut self, block: &mut Block) {
        self.parse_precedence(Precedence::Assignment, block);
    }

    fn number(&mut self, block: &mut Block) {
        let value: f64 = self.previous.lexeme.parse().unwrap();
        self.emit_constant(Value::Number(value), block);
    }

    fn grouping(&mut self, block: &mut Block) {
        self.expression(block);
        self.match_token(TokenType::RightParen, "Expect ')' after expression.");
    }

    fn unary(&mut self, block: &mut Block) {
        let operator = self.previous.token_type;

        self.parse_precedence(Precedence::Unary, block);

        match operator {
            TokenType::Minus => self.emit_byte(OpCode::Negate as u8, block),
            _ => unreachable!()
        }
    }

    fn binary(&mut self, block: &mut Block) {
        let operator = self.previous.token_type;
        let parse_rule = self.get_rule(operator);
        self.parse_precedence(parse_rule.precedence.next(), block);

        match operator {
            TokenType::Plus => self.emit_byte(OpCode::Add as u8, block),
            TokenType::Minus => self.emit_byte(OpCode::Subtract as u8, block),
            TokenType::Star => self.emit_byte(OpCode::Multiply as u8, block),
            TokenType::Slash => self.emit_byte(OpCode::Divide as u8, block),
            _ => unreachable!()
        }
    }

    fn parse_precedence(&mut self, precedence: Precedence, block: &mut Block) {
        self.advance();
        let prefix_rule = self.get_rule(self.previous.token_type).prefix;

        match prefix_rule {
            Some(prefix) => prefix(self, block),
            None => {
                self.error_previous("Expected expression.");
                return;
            }
        }

        while precedence <= self.get_rule(self.current.token_type).precedence {
            self.advance();
            let infix_rule = self.get_rule(self.previous.token_type).infix.unwrap();
            infix_rule(self, block);
        }
    }

    fn get_rule(&self, token_type: TokenType) -> &ParseRule<'a> {
        &self.rules[token_type as usize]
    }

    fn emit_constant(&mut self, value: Value, block: &mut Block) {
        let constant = self.make_constant(value, block);
        self.emit_bytes(OpCode::Constant as u8, constant, block);
    }

    fn emit_byte(&mut self, byte: u8, block: &mut Block) {
        block.write(byte, self.previous.line as u16);
    }

    fn emit_bytes(&mut self, byte1: u8, byte2: u8, block: &mut Block) {
        self.emit_byte(byte1, block);
        self.emit_byte(byte2, block);
    }

    fn emit_return(&mut self, block: &mut Block) {
        self.emit_byte(OpCode::Return as u8, block);
    }

    fn advance(&mut self) {
        self.previous = self.current;
        loop {
            self.current = self.scanner.scan_token();
            if self.current.token_type != TokenType::Error {
                break;
            }
            self.error_current("Unknown token.");
        }
    }

    fn match_token(&mut self, expected: TokenType, message: &str) {
        if self.current.token_type == expected {
            self.advance();
            return;
        }

        self.error_current(message);
    }

    fn make_constant(&mut self, value: Value, block: &mut Block) -> u8 {
        let constant = block.add_constant(value);
        match u8::try_from(constant) {
            Ok(constant) => constant,
            Err(_) => {
                self.error_previous("Too many constants in one chunk.");
                0
            }
        }
    }

    fn add_rule(&mut self, prefix: Option<ParseFn<'a>>, infix: Option<ParseFn<'a>>, precedence: Precedence) {
        self.rules.push(ParseRule::new(prefix, infix, precedence));
    }

    fn error_current(&mut self, msg: &str) {
        self.error(self.current, msg);
    }

    fn error_previous(&mut self, msg: &str) {
        self.error(self.previous, msg);
    }

    fn error(&mut self, token: Token<'a>, msg: &str) {
        if self.panic_mode {
            return;
        }

        self.had_error = true;
        self.panic_mode = true;

        eprint!("[line {}] Error at ", token.line);

        match token.token_type {
            TokenType::Eof => eprintln!("end of file"),
            _ => eprintln!("{}", token.lexeme),
        }
        eprintln!(": {}", msg);

        self.had_error = true;
        self.panic_mode = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_addition() {
        let mut parser = Parser::new("1 + 1");
        match parser.compile() {
            Ok(block) => {
                let expected = vec![
                    OpCode::Constant as u8, 0,
                    OpCode::Constant as u8, 1,
                    OpCode::Add as u8,
                    OpCode::Return as u8
                ];
                assert_eq!(block.code, expected);
            }
            Err(err) => panic!("{:?}", err)
        }
    }

}