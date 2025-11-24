use std::rc::Rc;
use crate::{
    block::{Block}, scanner::{Scanner}, token::{Token, TokenType}, value::Value, error::LoxError, op::OpCode, objects::{LoxString, Function}
};

pub fn compile(source: &str) -> Result<Rc<Function>, LoxError> {
    let mut parser = Parser::new(source);
    let function = parser.compile()?;
    Ok(Rc::new(*function))
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

type ParseFn<'a> = fn(&mut Parser<'a>, bool) -> ();

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

struct Local<'a> {
    token: Token<'a>,
    depth: i32,
}

impl<'a> Local<'a> {
    fn new(token: Token<'a>, depth: i32) -> Self {
        Local { token, depth }
    }
}

#[derive(PartialEq)]
enum FunctionType {
    Function,
    Script,
}

struct Compiler<'a> {
    enclosing: Option<Box<Compiler<'a>>>,
    function: Option<Box<Function>>,
    function_type: FunctionType,
    locals: Vec<Local<'a>>,
    scope_depth: i32,
}

impl<'a> Compiler<'a> {
    const MAX_LOCALS: usize = u8::MAX as usize + 1;

    pub fn new(function_name: Rc<LoxString>, function_type: FunctionType) -> Self {
        let mut compiler = Compiler {
            enclosing: None,
            function: Some(Function::new(function_name)),
            function_type,
            locals: Vec::with_capacity(Compiler::MAX_LOCALS),
            scope_depth: 0,
        };

        compiler.locals.push(Local::new(Token::default(""), 0));

        compiler
    }

    pub fn is_local(&self, name: Token<'a>) -> bool {
        for local in self.locals.iter().rev() {
            if local.depth != -1 && local.depth < self.scope_depth {
                break;
            }

            if name.lexeme == local.token.lexeme {
                return true;
            }
        }
        false
    }

    pub fn resolve_local(&self, name: &Token<'a>, errors: &mut Vec<&'static str>) -> Option<u8> {
        for (i, local) in self.locals.iter().enumerate().rev() {
            if name.lexeme == local.token.lexeme {
                if local.depth == -1 {
                    errors.push("Can't read local variable in its own initializer.");
                }
                return Some(i as u8);
            }
        }

        None
    }
}

struct Parser<'a> {
    scanner: Scanner<'a>,
    compiler: Compiler<'a>,
    current: Token<'a>,
    previous: Token<'a>,
    rules: Vec<ParseRule<'a>>,
    resolve_errors: Vec<&'static str>,
    had_error: bool,
    panic_mode: bool,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str) -> Self {
        let scanner = Scanner::new(source);

        let mut parser = Parser {
            scanner,
            compiler: Compiler::new(LoxString::new("script"), FunctionType::Script),
            current: Token::default(""),
            previous: Token::default(""),
            rules: Vec::with_capacity(40),
            resolve_errors: Vec::with_capacity(16),
            had_error: false,
            panic_mode: false
        };

        parser.add_rule(Some(Parser::grouping), Some(Parser::call), Precedence::Call); // LeftParen
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
        parser.add_rule(None, Some(Parser::and), Precedence::And);  // And
        parser.add_rule(None, None, Precedence::None);  // Class
        parser.add_rule(None, None, Precedence::None);  // Else
        parser.add_rule(Some(Parser::literal), None, Precedence::None);  // False
        parser.add_rule(None, None, Precedence::None);  // Fun
        parser.add_rule(None, None, Precedence::None);  // For
        parser.add_rule(None, None, Precedence::None);  // If
        parser.add_rule(Some(Parser::literal), None, Precedence::None);  // Nil
        parser.add_rule(None, Some(Parser::or), Precedence::Or);  // Or
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

    pub fn compile(&mut self) -> Result<Box<Function>, LoxError> {
        self.advance();

        while !self.matches(TokenType::Eof) {
            self.declaration();
        }

        self.emit_return();

        if self.had_error {
            Err(LoxError::CompileError("Compile error".to_string()))
        } else {
            Ok(self.compiler.function.take().unwrap())
        }
    }

    fn call(&mut self, _is_assign: bool) {
        let arg_count = self.argument_list();
        self.emit_instr(OpCode::Call(arg_count));
    }

    fn argument_list(&mut self) -> u8 {
        let mut count = 0usize;

        if !self.check(TokenType::RightParen) {
            loop {
                self.expression();
                count += 1;

                if count > 255 {
                    self.error_previous("Can't have more than 255 arguments.");
                }

                if !self.matches(TokenType::Comma) {
                    break;
                }
            }
        }

        self.match_token(TokenType::RightParen, "Expected ')' after arguments.");

        count as u8
    }

    fn declaration(&mut self) {
        if self.matches(TokenType::Fun) {
            self.fun_declaration();
        } else if self.matches(TokenType::Var) {
            self.var_declaration();
        } else {
            self.statement();
        }

        if self.panic_mode {
            self.sync();
        }
    }

    fn fun_declaration(&mut self) {
        let global = self.parse_variable("Expected function name.");
        self.mark_initialized();
        self.function(FunctionType::Function);
        self.define_variable(global);
    }

    fn function(&mut self, function_type: FunctionType) {
        self.compiler_push(function_type);
        self.begin_scope();

        self.match_token(TokenType::LeftParen, "Expected '(' after function name.");

        if !self.check(TokenType::RightParen) {
            loop {
                self.compiler.function.as_mut().unwrap().arity += 1;
                if self.compiler.function.as_ref().unwrap().arity > 255 {
                    self.error_previous("Can't have more than 255 parameters.");
                }

                let param_constant = self.parse_variable("Expected parameter name.");
                self.define_variable(param_constant);

                if !self.matches(TokenType::Comma) {
                    break;
                }
            }
        }

        self.match_token(TokenType::RightParen, "Expected ')' after parameters.");

        self.match_token(TokenType::LeftBrace, "Expected '{' before function body.");
        self.block();

        let function = self.compiler_pop();

        let index = self.make_constant(Value::Function(Rc::new(*function)));
        self.emit_instr(OpCode::Constant(index));
    }

    fn compiler_push(&mut self, function_type: FunctionType) {
        let name = self.previous.lexeme;
        let compiler = Compiler::new(LoxString::from_string(name), function_type);
        let prev_compiler = std::mem::replace(&mut self.compiler, compiler);
        self.compiler.enclosing = Some(Box::new(prev_compiler));
    }

    fn compiler_pop(&mut self) -> Box<Function> {
        self.emit_return();

        match self.compiler.enclosing.take() {
            Some(enclosing) => {
                let compiler = std::mem::replace(&mut self.compiler, *enclosing);
                compiler.function.unwrap()
            },
            None => panic!("No enclosing compiler to pop to."),
        }
    }

    fn var_declaration(&mut self) {
        let global = self.parse_variable("Expected variable name.");

        if self.matches(TokenType::Equal) {
            self.expression();
        } else {
            self.emit_instr(OpCode::Nil);
        }

        self.match_token(TokenType::Semicolon, "Expected semicolon after variable declaration.");
        self.define_variable(global);
    }

    fn add_local(&mut self, name: Token<'a>) {
        if self.compiler.locals.len() == Compiler::MAX_LOCALS {
            self.error(name, "Too many local variables in scope.");
            return;
        }

        let local = Local::new(name, -1);
        self.compiler.locals.push(local);
    }

    fn declare_variable(&mut self) {
        if self.compiler.scope_depth == 0 {
            return;
        }

        let name = self.previous;

        if self.compiler.is_local(name) {
            self.error(name, "Already a variable with this name in this scope.");
        }

        self.add_local(name)
    }

    fn parse_variable(&mut self, message: &str) -> u8 {
        self.match_token(TokenType::Identifier, message);

        self.declare_variable();
        if self.compiler.scope_depth > 0 {
            return 0;
        }

        self.identifier_constant(self.previous)
    }

    fn identifier_constant(&mut self, name: Token) -> u8 {
        self.make_constant(Value::String(LoxString::from_string(name.lexeme)))
    }

    fn variable(&mut self, is_assign: bool) {
        self.named_variable(self.previous, is_assign);
    }

    fn resolve_local(&mut self, name: &Token<'a>) -> Option<u8> {
        let result = self.compiler.resolve_local(name, &mut self.resolve_errors);
        
        while let Some(error) = self.resolve_errors.pop() {
            self.error(*name, error);
        }

        result
    }

    fn named_variable(&mut self, name: Token<'a>, is_assign: bool) {
        let (get_op, set_op) = if let Some(arg) = self.resolve_local(&name) {
            (OpCode::GetLocal(arg), OpCode::SetLocal(arg))
        } else {
            let global = self.identifier_constant(name);
            (OpCode::GetGlobal(global), OpCode::SetGlobal(global))
        };

        if is_assign && self.matches(TokenType::Equal) {
            self.expression();
            self.emit_instr(set_op);
        } else {
            self.emit_instr(get_op);
        }
    }

    fn mark_initialized(&mut self) {
        if self.compiler.scope_depth > 0 {
            let last = self.compiler.locals.last_mut().unwrap();
            last.depth = self.compiler.scope_depth;
        }
    }

    fn define_variable(&mut self, global: u8) {
        if self.compiler.scope_depth > 0 {
            self.mark_initialized();
            return;
        }

        self.emit_instr(OpCode::DefGlobal(global));
    }

    fn block(&mut self) {
        while !self.check(TokenType::RightBrace) && !self.check(TokenType::Eof) {
            self.declaration();
        }

        self.match_token(TokenType::RightBrace, "Expected '}' after block.");
    }

    fn begin_scope(&mut self) {
        self.compiler.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.compiler.scope_depth -= 1;

        while !self.compiler.locals.is_empty() && self.compiler.locals[self.compiler.locals.len() - 1].depth > self.compiler.scope_depth {
            self.emit_instr(OpCode::Pop);
            self.compiler.locals.pop();
        }
    }

    fn statement(&mut self) {
        if self.matches(TokenType::Print) {
            self.print_statement();
        } else if self.matches(TokenType::For) {
            self.for_statement();
        } else if self.matches(TokenType::If) {
            self.if_statement();
        } else if self.matches(TokenType::Return) {
            self.return_statement();
        } else if self.matches(TokenType::While) {
            self.while_statement();
        } else if self.matches(TokenType::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else {
            self.expression_statement();
        }
    }

    fn return_statement(&mut self) {
        if self.compiler.function_type == FunctionType::Script {
            self.error_previous("Can't return from top-level code.");
        }

        if self.matches(TokenType::Semicolon) {
            self.emit_return();
        } else {
            self.expression();
            self.match_token(TokenType::Semicolon, "Expected ';' after return value.");
            self.emit_instr(OpCode::Return);
        }
    }

    fn for_statement(&mut self) {
        self.begin_scope();

        self.match_token(TokenType::LeftParen, "Expected '(' after 'for'.");

        if self.matches(TokenType::Semicolon) {
            // No initializer.
        } else if self.matches(TokenType::Var) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }

        let mut loop_start = self.compiler.function.as_ref().unwrap().block.code.len();

        let mut exit_jump = None;
        if !self.matches(TokenType::Semicolon) {
            self.expression();
            self.match_token(TokenType::Semicolon, "Expected ';' after loop condition.");

            exit_jump = Some(self.emit_instr(OpCode::JumpIfFalse(0xFFFF)));
            self.emit_instr(OpCode::Pop);
        }

        if !self.matches(TokenType::RightParen) {
            let body_jump = self.emit_instr(OpCode::Jump(0xFFFF));
            let increment_start = self.compiler.function.as_ref().unwrap().block.code.len();

            self.expression();

            self.emit_instr(OpCode::Pop);
            self.match_token(TokenType::RightParen, "Expected ')' after for clauses.");

            self.emit_loop(loop_start);
            loop_start = increment_start;
            self.patch_jump(body_jump);
        }

        self.statement();
        self.emit_loop(loop_start);

        if let Some(exit_jump) = exit_jump {
            self.patch_jump(exit_jump);
            self.emit_instr(OpCode::Pop);
        }

        self.end_scope();
    }

    fn while_statement(&mut self) {
        let loop_start = self.compiler.function.as_ref().unwrap().block.code.len();
        self.match_token(TokenType::LeftParen, "Expected '(' after 'while'.");
        self.expression();
        self.match_token(TokenType::RightParen, "Expected ')' after condition.");

        let exit_jump = self.emit_instr(OpCode::JumpIfFalse(0xFFFF));
        self.emit_instr(OpCode::Pop);

        self.statement();
        self.emit_loop(loop_start);

        self.patch_jump(exit_jump);

        self.emit_instr(OpCode::Pop);
    }

    fn if_statement(&mut self) {
        self.match_token(TokenType::LeftParen, "Expected '(' after 'if'.");
        self.expression();
        self.match_token(TokenType::RightParen, "Expected ')' after condition.");

        let then_jump = self.emit_instr(OpCode::JumpIfFalse(0xFFFF));
        self.emit_instr(OpCode::Pop);
        self.statement();

        let else_jump = self.emit_instr(OpCode::Jump(0xFFFF));

        self.patch_jump(then_jump);
        self.emit_instr(OpCode::Pop);

        if self.matches(TokenType::Else) {
            self.statement();
        }

        self.patch_jump(else_jump);
    }

    fn and(&mut self, _is_assign: bool) {
        let jump = self.emit_instr(OpCode::JumpIfFalse(0xFFFF));
        self.emit_instr(OpCode::Pop);
        self.parse_precedence(Precedence::And);
        self.patch_jump(jump);
    }

    fn or(&mut self, _is_assign: bool) {
        let else_jump = self.emit_instr(OpCode::JumpIfFalse(0xFFFF));
        let end_jump = self.emit_instr(OpCode::Jump(0xFFFF));

        self.patch_jump(else_jump);
        self.emit_instr(OpCode::Pop);

        self.parse_precedence(Precedence::Or);
        self.patch_jump(end_jump);
    }

    fn print_statement(&mut self) {
        self.expression();
        self.match_token(TokenType::Semicolon, "Expected semicolon after value.");
        self.emit_instr(OpCode::Print);
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.match_token(TokenType::Semicolon, "Expected semicolon after expression.");
        self.emit_instr(OpCode::Pop);
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn number(&mut self, _is_assign: bool) {
        let value: f64 = self.previous.lexeme.parse().unwrap();
        self.emit_constant(Value::Number(value));
    }

    fn grouping(&mut self, _is_assign: bool) {
        self.expression();
        self.match_token(TokenType::RightParen, "Expect ')' after expression.");
    }

    fn unary(&mut self, _is_assign: bool) {
        let operator = self.previous.token_type;

        self.parse_precedence(Precedence::Unary);

        match operator {
            TokenType::Bang => self.emit_instr(OpCode::Not),
            TokenType::Minus => self.emit_instr(OpCode::Negate),
            _ => unreachable!()
        };
    }

    fn binary(&mut self, _is_assign: bool) {
        let operator = self.previous.token_type;
        let parse_rule = self.get_rule(operator);
        self.parse_precedence(parse_rule.precedence.next());

        match operator {
            TokenType::BangEqual => self.emit_two_instr(OpCode::Equal, OpCode::Not),
            TokenType::EqualEqual => self.emit_instr(OpCode::Equal),
            TokenType::Greater => self.emit_instr(OpCode::Greater),
            TokenType::GreaterEqual => self.emit_two_instr(OpCode::Less, OpCode::Not),
            TokenType::Less => self.emit_instr(OpCode::Less),
            TokenType::LessEqual => self.emit_two_instr(OpCode::Greater, OpCode::Not),
            TokenType::Plus => self.emit_instr(OpCode::Add),
            TokenType::Minus => self.emit_instr(OpCode::Subtract),
            TokenType::Star => self.emit_instr(OpCode::Multiply),
            TokenType::Slash => self.emit_instr(OpCode::Divide),
            _ => unreachable!()
        };
    }

    fn literal(&mut self, _is_assign: bool) {
        match self.previous.token_type {
            TokenType::True => self.emit_instr(OpCode::True),
            TokenType::False => self.emit_instr(OpCode::False),
            TokenType::Nil => self.emit_instr(OpCode::Nil),
            _ => unreachable!()
        };
    }

    fn string(&mut self, _is_assign: bool) {
        let value = LoxString::from_string(&self.previous.lexeme[1..self.previous.lexeme.len() - 1]);
        self.emit_constant(Value::String(value));
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
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
        prefix(self, is_assign);

        while precedence <= self.get_rule(self.current.token_type).precedence {
            self.advance();
            let infix_rule = self.get_rule(self.previous.token_type).infix.unwrap();
            infix_rule(self, is_assign);
        }

        if is_assign && self.matches(TokenType::Equal) {
            self.error_previous("Invalid assignment target.");
        }
    }

    fn get_rule(&self, token_type: TokenType) -> &ParseRule<'a> {
        &self.rules[token_type as usize]
    }

    fn emit_constant(&mut self, value: Value) {
        let constant = self.make_constant(value);
        self.emit_instr(OpCode::Constant(constant));
    }

    fn emit_instr(&mut self, byte: OpCode) -> usize {
        self.compiler.function.as_mut().unwrap().block.write(byte, self.previous.line as u16)
    }

    fn emit_two_instr(&mut self, byte1: OpCode, byte2: OpCode) -> usize{
        self.emit_instr(byte1);
        self.emit_instr(byte2)
    }

    fn emit_return(&mut self) {
        self.emit_instr(OpCode::Nil);
        self.emit_instr(OpCode::Return);
    }

    fn emit_loop(&mut self, loop_start: usize) {
        let offset = self.compiler.function.as_ref().unwrap().block.code.len() - loop_start;
        let offset = match u16::try_from(offset) {
            Ok(offset) => offset,
            Err(_) => {
                self.error_previous("Loop body too large.");
                return;
            }
        };

        self.emit_instr(OpCode::Loop(offset));
    }

    fn patch_jump(&mut self, offset: usize) {
        let jump = self.compiler.function.as_ref().unwrap().block.code.len() - offset - 1;

        let jump = match u16::try_from(jump) {
            Ok(jump) => jump,
            Err(_) => {
                self.error_previous("Too much code to jump over.");
                return;
            }
        };

        match self.compiler.function.as_mut().unwrap().block.code[offset] {
            OpCode::Jump(ref mut val) | OpCode::JumpIfFalse(ref mut val) => {
                *val = jump;
            },
            _ => {
                self.error_previous("Can only patch jump instructions.");
                return;
            }
        }
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

    fn make_constant(&mut self, value: Value) -> u8 {
        let constant = self.compiler.function.as_mut().unwrap().block.add_constant(value);
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