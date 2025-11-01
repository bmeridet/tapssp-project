
use crate::token::{Token, TokenType};
use std::collections::HashMap;

pub struct Scanner<'a> {
    source: &'a str,
    start: usize,
    current: usize,
    line: usize,
    keywords: HashMap<&'static str, TokenType>,
}

#[derive(thiserror::Error, Debug)]
pub enum ScanError {
    #[error("Unexpected character at line {0}")]
    UnexpectedCharacter(usize),
    #[error("Unterminated string at line {0}")]
    UnterminatedString(usize),
}

impl<'a> Scanner<'a> {
    pub fn new(source: &'a str) -> Self {
        let mut keywords = HashMap::new();
        keywords.insert("and", TokenType::And);
        keywords.insert("class", TokenType::Class);
        keywords.insert("else", TokenType::Else);
        keywords.insert("false", TokenType::False);
        keywords.insert("for", TokenType::For);
        keywords.insert("fun", TokenType::Fun);
        keywords.insert("if", TokenType::If);
        keywords.insert("nil", TokenType::Nil);
        keywords.insert("or", TokenType::Or);
        keywords.insert("print", TokenType::Print);
        keywords.insert("return", TokenType::Return);
        keywords.insert("super", TokenType::Super);
        keywords.insert("this", TokenType::This);
        keywords.insert("true", TokenType::True);
        keywords.insert("var", TokenType::Var);
        keywords.insert("while", TokenType::While);

        Self {
            source,
            start: 0,
            current: 0,
            line: 1,
            keywords,
        }
    }

    pub fn scan_token(&mut self) -> Token<'a> {
        self.skip_whitespace();
        self.start = self.current;

        if self.is_at_end() {
            return self.new_token(TokenType::Eof);
        }

        match self.advance() {
            b'(' => self.new_token(TokenType::LeftParen),
            b')' => self.new_token(TokenType::RightParen),
            b'{' => self.new_token(TokenType::LeftBrace),
            b'}' => self.new_token(TokenType::RightBrace),
            b',' => self.new_token(TokenType::Comma),
            b'.' => self.new_token(TokenType::Dot),
            b'-' => self.new_token(TokenType::Minus),
            b'+' => self.new_token(TokenType::Plus),
            b';' => self.new_token(TokenType::Semicolon),
            b'*' => self.new_token(TokenType::Star),
            b'/' => self.new_token(TokenType::Slash),

            b'!' if self.match_byte(b'=') => self.new_token(TokenType::BangEqual),
            b'!' => self.new_token(TokenType::Bang),

            b'=' if self.match_byte(b'=') => self.new_token(TokenType::EqualEqual),
            b'=' => self.new_token(TokenType::Equal),

            b'<' if self.match_byte(b'=') => self.new_token(TokenType::LessEqual),
            b'<' => self.new_token(TokenType::Less),

            b'>' if self.match_byte(b'=') => self.new_token(TokenType::GreaterEqual),
            b'>' => self.new_token(TokenType::Greater),

            b'"' => self.string(),

            b if is_digit(b) => self.number(),

            b if is_alpha(b) => self.identifier(),

            _ => self.scan_error("Unexpected character."),
        }
    }

    #[inline]
    fn advance(&mut self) -> u8 {
        let b = self.source.as_bytes()[self.current];
        self.current += 1;
        b
    }

    #[inline]
    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    #[inline]
    fn peek(&self) -> u8 {
        if self.is_at_end() {
            0
        } else {
            self.source.as_bytes()[self.current]
        }
    }

    #[inline]
    fn peek_next(&self) -> u8 {
        if self.current + 1 >= self.source.len() {
            0
        } else {
            self.source.as_bytes()[self.current + 1]
        }
    }

    #[inline]
    fn match_byte(&mut self, expected: u8) -> bool {
        if self.is_at_end() {
            return false;
        }
        if self.source.as_bytes()[self.current] != expected {
            return false;
        }
        self.current += 1;
        true
    }

    fn new_token(&self, token_type: TokenType) -> Token<'a> {
        let lexeme = &self.source[self.start..self.current];
        Token {
            token_type,
            lexeme,
            line: self.line,
        }
    }

    fn scan_error(&self, message: &'static str) -> Token<'static> {
        Token {
            token_type: TokenType::Error,
            lexeme: message,
            line: self.line,
        }
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                b' ' | b'\r' | b'\t' => {
                    self.advance();
                }
                b'\n' => {
                    self.line += 1;
                    self.advance();
                }
                b'/' if self.peek_next() == b'/' => {
                    while self.peek() != b'\n' && !self.is_at_end() {
                        self.advance();
                    }
                }
                _ => return,
            }
        }
    }

    fn string(&mut self) -> Token<'a> {
        while !self.is_at_end() && self.peek() != b'"' {
            if self.peek() == b'\n' {
                self.line += 1;
            }
            self.advance();
        }

        if self.is_at_end() {
            return self.scan_error("Unterminated string.");
        }

        self.advance();

        self.new_token(TokenType::String)
    }

    fn number(&mut self) -> Token<'a> {
        while is_digit(self.peek()) {
            self.advance();
        }

        if self.peek() == b'.' && is_digit(self.peek_next()) {
            self.advance();

            while is_digit(self.peek()) {
                self.advance();
            }
        }

        self.new_token(TokenType::Number)
    }

    fn identifier(&mut self) -> Token<'a> {
        while is_alphanumeric(self.peek()) {
            self.advance();
        }

        let lexeme = &self.source[self.start..self.current];
        let token_type = self.keywords.get(lexeme)
            .cloned()
            .unwrap_or(TokenType::Identifier);

        self.new_token(token_type)
    }
}

impl<'a> Iterator for Scanner<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_at_end() {
            None
        } else {
            Some(self.scan_token())
        }
    }
}

#[inline]
fn is_alpha(c: u8) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_uppercase() || c == b'_'
}

#[inline]
fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

#[inline]
fn is_alphanumeric(c: u8) -> bool {
    is_alpha(c) || is_digit(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::Token;

    fn scan(source: &str) -> Vec<Token> {
        let mut scanner = Scanner::new(source);
        let mut tokens = Vec::new();
        loop {
            let token = scanner.scan_token();
            if token.token_type == TokenType::Eof {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }
        tokens
    }

    #[test]
    fn test_single_char_tokens() {
        let tokens = scan("(){},.-+;*");
        let expected_types = vec![
            TokenType::LeftParen,
            TokenType::RightParen,
            TokenType::LeftBrace,
            TokenType::RightBrace,
            TokenType::Comma,
            TokenType::Dot,
            TokenType::Minus,
            TokenType::Plus,
            TokenType::Semicolon,
            TokenType::Star,
            TokenType::Eof,
        ];
        assert_eq!(tokens.len(), expected_types.len());
        for (token, expected_type) in tokens.iter().zip(expected_types) {
            assert_eq!(token.token_type, expected_type);
        }
    }

    #[test]
    fn test_two_char_tokens() {
        let tokens = scan("! != = == > >= < <=");
        let expected_types = vec![
            TokenType::Bang,
            TokenType::BangEqual,
            TokenType::Equal,
            TokenType::EqualEqual,
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Less,
            TokenType::LessEqual,
            TokenType::Eof,
        ];
        assert_eq!(tokens.len(), expected_types.len());
        for (token, expected_type) in tokens.iter().zip(expected_types) {
            assert_eq!(token.token_type, expected_type);
        }
    }

    #[test]
    fn test_white_space_and_comments() {
        let tokens = scan("  \n // This is a comment \n ");
        assert_eq!(tokens.len(), 1); // Only EOF token
        assert_eq!(tokens[0].token_type, TokenType::Eof);
    }

    #[test]
    fn test_newlines() {
        let tokens = scan("\n\n\n");
        assert_eq!(tokens.len(), 1); // Only EOF token
        assert_eq!(tokens[0].token_type, TokenType::Eof);
        assert_eq!(tokens[0].line, 4); // Line number should be 4
    }
    
    #[test]
    fn test_numbers() {
        let tokens = scan("123 45.67");
        let expected_types = vec![
            TokenType::Number,
            TokenType::Number,
            TokenType::Eof,
        ];
        assert_eq!(tokens.len(), expected_types.len());
        for (token, expected_type) in tokens.iter().zip(expected_types) {
            assert_eq!(token.token_type, expected_type);
        }
    }

    #[test]
    fn test_identifiers_and_keywords() {
        let tokens = scan("var foo = true;");
        let expected_types = vec![
            TokenType::Var,
            TokenType::Identifier,
            TokenType::Equal,
            TokenType::True,
            TokenType::Semicolon,
            TokenType::Eof,
        ];
        assert_eq!(tokens.len(), expected_types.len());
        for (token, expected_type) in tokens.iter().zip(expected_types) {
            assert_eq!(token.token_type, expected_type);
        }
    }

    #[test]
    fn test_strings() {
        let tokens = scan(r#""hello" "world""#);
        let expected_types = vec![
            TokenType::String,
            TokenType::String,
            TokenType::Eof,
        ];
        assert_eq!(tokens.len(), expected_types.len());
        for (token, expected_type) in tokens.iter().zip(expected_types) {
            assert_eq!(token.token_type, expected_type);
        }
    }
}