#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Self { start, end, line, column }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TokenType {
    #[default]
    TError,
    TEof,
    TNl,
    H6, H5, H4, H3, H2, H1,
    CODE,
    HR,
    TEXT,
    SPACE,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub content: String,
    pub span: Span,
}

impl Default for Token {
    fn default() -> Self {
        Self {
            token_type: TokenType::TError,
            content: String::new(),
            span: Span::default(),
        }
    }
}

pub type Node<'a> = (); 
