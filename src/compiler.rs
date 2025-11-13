
use crate::{
    block::{Block}, scanner::{ScanError, Scanner}, token::{Token, TokenType}, value::Value, error::LoxError, op::OpCode, debug::disassemble_block, objects::LoxString
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

type ParseFn<'a> = fn(&mut Parser<'a>, &mut Block, bool) -> ();

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

        parser.add_rule(Some(Parser::grouping), None, Precedence::None); // LeftParen
        parser.add_rule(None, None, Precedence::None); // RightParen
        parser.add_rule(None, None, Precedence::None); // LeftBrace
        parser.add_rule(None, None, Precedence::None); // RightBrace
        parser.add_rule(None, None, Precedence::None); // Comma
        parser.add_rule(None, None, Precedence::None); // Dot
        parser.add_rule(Some(Parser::unary), Some(Parser::binary), Precedence::Term); // Minus
        parser.add_rule(None, Some(Parser::binary), Precedence::Term); // Plus
        parser.add_rule(None, None, Precedence::None); // Semicolon
        parser.add_rule(None, Some(Parser::binary), Precedence::Factor);  // Slash
        parser.add_rule(None, Some(Parser::binary), Precedence::Factor);  // Star
        parser.add_rule(Some(Parser::unary), None, Precedence::None);  // Bang
        parser.add_rule(None, Some(Parser::binary), Precedence::Equality);  // BangEqual
        parser.add_rule(None, None, Precedence::None);  // Equal
        parser.add_rule(None, Some(Parser::binary), Precedence::Equality);  // EqualEqual
        parser.add_rule(None, Some(Parser::binary), Precedence::Comparison);  // Greater
        parser.add_rule(None, Some(Parser::binary), Precedence::Comparison);  // GreaterEqual
        parser.add_rule(None, Some(Parser::binary), Precedence::Comparison);  // Less
        parser.add_rule(None, Some(Parser::binary), Precedence::Comparison);  // LessEqual
        parser.add_rule(Some(Parser::variable), None, Precedence::None);  // Identifier
        parser.add_rule(Some(Parser::string), None, Precedence::None);  // String
        parser.add_rule(Some(Parser::number), None, Precedence::None);  // Number
        parser.add_rule(None, None, Precedence::None);  // And
        parser.add_rule(None, None, Precedence::None);  // Class
        parser.add_rule(None, None, Precedence::None);  // Else
        parser.add_rule(Some(Parser::literal), None, Precedence::None);  // False
        parser.add_rule(None, None, Precedence::None);  // Fun
        parser.add_rule(None, None, Precedence::None);  // For
        parser.add_rule(None, None, Precedence::None);  // If
        parser.add_rule(Some(Parser::literal), None, Precedence::None);  // Nil
        parser.add_rule(None, None, Precedence::None);  // Or
        parser.add_rule(None, None, Precedence::None);  // Print
        parser.add_rule(None, None, Precedence::None);  // Return
        parser.add_rule(None, None, Precedence::None);  // Super
        parser.add_rule(None, None, Precedence::None);  // This
        parser.add_rule(Some(Parser::literal), None, Precedence::None);  // True
        parser.add_rule(None, None, Precedence::None);  // Var
        parser.add_rule(None, None, Precedence::None);  // While
        parser.add_rule(None, None, Precedence::None);  // Error
        parser.add_rule(None, None, Precedence::None);  // Eof

        parser
    }

    pub fn compile(&mut self) -> Result<Block, LoxError> {
        let mut block = Block::new();

        self.advance();

        while !self.matches(TokenType::Eof) {
            self.declaration(&mut block);
        }

        self.emit_return(&mut block);

        #[cfg(debug_assertions)]
        {
            disassemble_block(&block, "code");
        }

        if self.had_error {
            Err(LoxError::CompileError("Compile error".to_string()))
        } else {
            Ok(block)
        }
    }

    fn declaration(&mut self, block: &mut Block) {
        if self.matches(TokenType::Var) {
            self.var_declaration(block);
        } else {
            self.statement(block);
        }

        if self.panic_mode {
            self.sync();
        }
    }

    fn var_declaration(&mut self, block: &mut Block) {
        let global = self.parse_variable("Expected variable name.", block);

        if self.matches(TokenType::Equal) {
            self.expression(block);
        } else {
            self.emit_byte(OpCode::Nil as u8, block);
        }

        self.match_token(TokenType::Semicolon, "Expected semicolon after variable declaration.");
        self.define_variable(block, global);
    }

    fn parse_variable(&mut self, message: &str, block: &mut Block) -> u8 {
        self.match_token(TokenType::Identifier, message);
        self.identifier_constant(self.previous, block)
    }

    fn identifier_constant(&mut self, name: Token, block: &mut Block) -> u8 {
        self.make_constant(Value::String(LoxString::from_string(name.lexeme)), block)
    }

    fn variable(&mut self, block: &mut Block, is_assign: bool) {
        self.named_variable(self.previous, block, is_assign);
    }

    fn named_variable(&mut self, name: Token, block: &mut Block, is_assign: bool) {
        let index = self.identifier_constant(name, block);

        if is_assign &&self.matches(TokenType::Equal) {
            self.expression(block);
            self.emit_bytes(OpCode::SetGlobal as u8, index, block);
        } else {
            self.emit_bytes(OpCode::GetGlobal as u8, index, block);
        }
    }

    fn define_variable(&mut self, block: &mut Block, global: u8) {
        self.emit_bytes(OpCode::DefGlobal as u8, global, block);
    }

    fn statement(&mut self, block: &mut Block) {
        if self.matches(TokenType::Print) {
            self.print_statement(block);
        } else {
            self.expression_statement(block);
        }
    }

    fn print_statement(&mut self, block: &mut Block) {
        self.expression(block);
        self.match_token(TokenType::Semicolon, "Expected semicolon after value.");
        self.emit_byte(OpCode::Print as u8, block);
    }

    fn expression_statement(&mut self, block: &mut Block) {
        self.expression(block);
        self.match_token(TokenType::Semicolon, "Expected semicolon after expression.");
        self.emit_byte(OpCode::Pop as u8, block);
    }

    fn expression(&mut self, block: &mut Block) {
        self.parse_precedence(Precedence::Assignment, block);
    }

    fn number(&mut self, block: &mut Block, is_assign: bool) {
        let value: f64 = self.previous.lexeme.parse().unwrap();
        self.emit_constant(Value::Number(value), block);
    }

    fn grouping(&mut self, block: &mut Block, is_assign: bool) {
        self.expression(block);
        self.match_token(TokenType::RightParen, "Expect ')' after expression.");
    }

    fn unary(&mut self, block: &mut Block, is_assign: bool) {
        let operator = self.previous.token_type;

        self.parse_precedence(Precedence::Unary, block);

        match operator {
            TokenType::Bang => self.emit_byte(OpCode::Not as u8, block),
            TokenType::Minus => self.emit_byte(OpCode::Negate as u8, block),
            _ => unreachable!()
        }
    }

    fn binary(&mut self, block: &mut Block, is_assign: bool) {
        let operator = self.previous.token_type;
        let parse_rule = self.get_rule(operator);
        self.parse_precedence(parse_rule.precedence.next(), block);

        match operator {
            TokenType::BangEqual => self.emit_bytes(OpCode::Equal as u8, OpCode::Not as u8, block),
            TokenType::EqualEqual => self.emit_byte(OpCode::Equal as u8, block),
            TokenType::Greater => self.emit_byte(OpCode::Greater as u8, block),
            TokenType::GreaterEqual => self.emit_bytes(OpCode::Less as u8, OpCode::Not as u8, block),
            TokenType::Less => self.emit_byte(OpCode::Less as u8, block),
            TokenType::LessEqual => self.emit_bytes(OpCode::Greater as u8, OpCode::Not as u8, block),
            TokenType::Plus => self.emit_byte(OpCode::Add as u8, block),
            TokenType::Minus => self.emit_byte(OpCode::Subtract as u8, block),
            TokenType::Star => self.emit_byte(OpCode::Multiply as u8, block),
            TokenType::Slash => self.emit_byte(OpCode::Divide as u8, block),
            _ => unreachable!()
        }
    }

    fn literal(&mut self, block: &mut Block, is_assign: bool) {
        match self.previous.token_type {
            TokenType::True => self.emit_byte(OpCode::True as u8, block),
            TokenType::False => self.emit_byte(OpCode::False as u8, block),
            TokenType::Nil => self.emit_byte(OpCode::Nil as u8, block),
            _ => unreachable!()
        }
    }

    fn string(&mut self, block: &mut Block, is_assign: bool) {
        let value = LoxString::from_string(&self.previous.lexeme[1..self.previous.lexeme.len() - 1]);
        self.emit_constant(Value::String(value), block);
    }

    fn parse_precedence(&mut self, precedence: Precedence, block: &mut Block) {
        self.advance();
        let prefix_rule = self.get_rule(self.previous.token_type).prefix;

        let prefix = match prefix_rule {
            Some(rule) => rule,
            None => {
                self.error_previous("Expected expression.");
                return;
            }
        };

        let is_assign = precedence <= Precedence::Assignment;
        prefix(self, block, is_assign);

        while precedence <= self.get_rule(self.current.token_type).precedence {
            self.advance();
            let infix_rule = self.get_rule(self.previous.token_type).infix.unwrap();
            infix_rule(self, block, is_assign);
        }

        if is_assign && self.matches(TokenType::Equal) {
            self.error_previous("Invalid assignment target.");
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

    fn check(&self, token_type: TokenType) -> bool {
        self.current.token_type == token_type
    }

    fn matches(&mut self, token_type: TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
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

    fn sync(&mut self) {
        self.panic_mode = false;

        while self.current.token_type != TokenType::Eof {
            if self.previous.token_type == TokenType::Semicolon {
                return;
            }

            match self.current.token_type {
                TokenType::Class |
                TokenType::Fun |
                TokenType::Var |
                TokenType::For |
                TokenType::If |
                TokenType::While |
                TokenType::Print |
                TokenType::Return => return,
                _ => {}
            }

            self.advance();
        }
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