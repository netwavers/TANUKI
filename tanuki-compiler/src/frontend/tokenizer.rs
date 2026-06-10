use crate::frontend::ast::{Token, TokenType, Span};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub struct Tokenizer {
    pub chars: Vec<char>,
    pub pos: usize,
    line: usize,
    column: usize,
    pub last_content: String,
}

impl Tokenizer {
    pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut all_chars = Vec::new();
        for line in reader.lines() {
            let line = line?;
            all_chars.extend(line.chars());
            all_chars.push('\n');
        }
        Ok(Self {
            chars: all_chars,
            pos: 0,
            line: 1,
            column: 1,
            last_content: String::new(),
        })
    }

    fn peek(&self) -> char {
        self.chars.get(self.pos).cloned().unwrap_or('\0')
    }

    fn consume(&mut self) -> char {
        let c = self.peek();
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else if c != '\0' {
            self.column += 1;
        }
        self.pos += 1;
        c
    }

    pub fn get_and_clear_last_content(&mut self) -> String {
        let content = self.last_content.clone();
        self.last_content.clear();
        content
    }

    pub fn get_token(&mut self) -> Token {
        let start_pos = self.pos;
        let start_line = self.line;
        let start_col = self.column;

        let c = self.peek();
        if c == '\0' {
            return Token {
                token_type: TokenType::TEof,
                content: "".to_string(),
                span: Span::new(start_pos, start_pos, start_line, start_col),
            };
        }

        let mut token = Token::default();
        token.span.start = start_pos;
        token.span.line = start_line;
        token.span.column = start_col;

        if c == '\n' {
            self.consume();
            token.token_type = TokenType::TNl;
            token.content = "\n".to_string();
        } else if c == ' ' {
            self.consume();
            token.token_type = TokenType::SPACE;
            token.content = " ".to_string();
        } else if c == '#' {
            let mut s = String::new();
            while self.peek() == '#' {
                s.push(self.consume());
            }
            // Check if it's followed by space (standard header) or start of line
            if self.peek() == ' ' || self.peek() == '\n' || self.peek() == '\0' {
                token.token_type = match s.len() {
                    1 => TokenType::H1,
                    2 => TokenType::H2,
                    3 => TokenType::H3,
                    4 => TokenType::H4,
                    5 => TokenType::H5,
                    6 => TokenType::H6,
                    _ => {
                        token.token_type = TokenType::TEXT;
                        token.content = s;
                        token.span.end = self.pos;
                        self.last_content.push_str(&token.content);
                        return token;
                    }
                };
                token.content = s;
            } else {
                token.token_type = TokenType::TEXT;
                token.content = s;
            }
        } else if c == '`' && self.pos + 2 < self.chars.len() && self.chars[self.pos+1] == '`' && self.chars[self.pos+2] == '`' {
            self.consume(); self.consume(); self.consume();
            token.token_type = TokenType::CODE;
            token.content = "```".to_string();
        } else if c == '-' && self.pos + 2 < self.chars.len() && self.chars[self.pos+1] == '-' && self.chars[self.pos+2] == '-' {
            self.consume(); self.consume(); self.consume();
            token.token_type = TokenType::HR;
            token.content = "---".to_string();
        } else {
            let mut s = String::new();
            while self.peek() != '\0' && self.peek() != '\n' && self.peek() != ' ' && self.peek() != '#' && 
                  !(self.peek() == '`' && self.pos + 2 < self.chars.len() && self.chars[self.pos+1] == '`' && self.chars[self.pos+2] == '`') {
                s.push(self.consume());
            }
            token.token_type = TokenType::TEXT;
            token.content = s;
        }

        token.span.end = self.pos;
        self.last_content.push_str(&token.content);
        token
    }
}
