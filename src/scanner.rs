
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

    pub fn scan(&mut self) -> Result<Vec<Token>, ScanError> {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            self.start = self.current;
            if let Some(token) = self.scan_token()? {
                tokens.push(token);
            }
        }

        tokens.push(Token::new(TokenType::Eof, self.current, self.current, self.line));
        Ok(tokens)
    }

    fn scan_token(&mut self) -> Result<Option<Token>, ScanError> {
        let byte = self.advance();

        let token = match byte {
            b'(' => Token::new(TokenType::LeftParen, self.start, self.current, self.line),
            b')' => Token::new(TokenType::RightParen, self.start, self.current, self.line),
            b'{' => Token::new(TokenType::LeftBrace, self.start, self.current, self.line),
            b'}' => Token::new(TokenType::RightBrace, self.start, self.current, self.line),
            b',' => Token::new(TokenType::Comma, self.start, self.current, self.line),
            b'.' => Token::new(TokenType::Dot, self.start, self.current, self.line),
            b'-' => Token::new(TokenType::Minus, self.start, self.current, self.line),
            b'+' => Token::new(TokenType::Plus, self.start, self.current, self.line),
            b';' => Token::new(TokenType::Semicolon, self.start, self.current, self.line),
            b'*' => Token::new(TokenType::Star, self.start, self.current, self.line),

            b'"' => {
                while !self.is_at_end() && self.peek() != b'"' {
                    if self.peek() == b'\n' {
                        self.line += 1;
                    }
                    self.advance();
                }

                if self.is_at_end() {
                    return Err(ScanError::UnterminatedString(self.line));
                }

                self.advance();

                Token::new(TokenType::String, self.start, self.current, self.line)  
            },
            
            b'!' => {
                let token_type = if self.match_byte(b'=') {
                    TokenType::BangEqual
                } else {
                    TokenType::Bang
                };
                Token::new(token_type, self.start, self.current, self.line)
            },
            b'=' => {
                let token_type = if self.match_byte(b'=') {
                    TokenType::EqualEqual
                } else {
                    TokenType::Equal
                };
                Token::new(token_type, self.start, self.current, self.line)
            },
            b'>' => {
                let token_type = if self.match_byte(b'=') {
                    TokenType::GreaterEqual
                } else {
                    TokenType::Greater
                };
                Token::new(token_type, self.start, self.current, self.line)
            },
            b'<' => {
                let token_type = if self.match_byte(b'=') {
                    TokenType::LessEqual
                } else {
                    TokenType::Less
                };
                Token::new(token_type, self.start, self.current, self.line)
            },

            b'/' => {
                match self.peek() {
                    b'/' => {
                        while !self.is_at_end() && self.peek() != b'\n' {
                            self.advance();
                        }
                        return Ok(None);
                    },
                    b'*' => {
                        self.advance();
                        while !self.is_at_end() {
                            if self.peek() == b'*' && self.peek_next() == b'/' {
                                self.advance();
                                self.advance();
                                break;
                            } else {
                                if self.peek() == b'\n' {
                                    self.line += 1;
                                }
                                self.advance();
                            }
                        }
                        return Ok(None);
                    },
                    _ => Token::new(TokenType::Slash, self.start, self.current, self.line),
                }
            },

            b' ' | b'\r' | b'\t' => return Ok(None),
            b'\n' => {
                self.line += 1;
                return Ok(None);
            },

            b if is_digit(b) => {
                while is_digit(self.peek()) {
                    self.advance();
                }

                if self.peek() == b'.' && is_digit(self.peek_next()) {
                    self.advance();

                    while is_digit(self.peek()) {
                        self.advance();
                    }
                }

                Token::new(TokenType::Number, self.start, self.current, self.line)
            },

            b if is_alpha(b) => {
                while is_alphanumeric(self.peek()) {
                    self.advance();
                }

                let lexeme = &self.source[self.start..self.current];
                let token_type = self.keywords.get(lexeme)
                    .cloned()
                    .unwrap_or(TokenType::Identifier);

                Token::new(token_type, self.start, self.current, self.line)
            },

            _ => {
                return Err(ScanError::UnexpectedCharacter(self.line));
            },
        };

        Ok(Some(token))
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
        scanner.scan().unwrap()
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
        let tokens = scan("  \n // This is a comment \n /* Multi-line \n comment */ ");
        assert_eq!(tokens.len(), 1); // Only EOF token
        assert_eq!(tokens[0].token_type, TokenType::Eof);
    }

    #[test]
    fn test_newlines() {
        let mut scanner = Scanner::new("\n\n");
        let tokens = scanner.scan().unwrap();
        assert_eq!(scanner.line, 3);
        assert_eq!(tokens.len(), 1); // Only EOF token
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