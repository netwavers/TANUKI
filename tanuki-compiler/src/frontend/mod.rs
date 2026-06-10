use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MdNode {
    pub id: String,
    pub source_path: String,
    pub file_hash: Option<String>,
    pub context_path: String, // e.g. "Header1 > Header2"
    pub parent_id: Option<u64>, // 親ノードのハッシュID
    pub title: String,
    pub content: String,
    pub kind: MdNodeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MdNodeKind {
    Header(u8),
    Paragraph,
    CodeBlock,
}

pub mod ast;
pub mod parser;
pub mod tokenizer;

use anyhow::Result;
use std::path::Path;
use self::tokenizer::Tokenizer;
use self::parser::MarkdownParser;

pub fn parse_markdown_file<P: AsRef<Path>>(path: P) -> Result<Vec<MdNode>> {
    let source_path = path.as_ref().to_string_lossy().to_string();
    let mut tokenizer = Tokenizer::new(path)?;
    let mut parser = MarkdownParser::new(&mut tokenizer, source_path);
    
    // 初回のトークンを取得
    parser.token = parser.tokenizer.get_token();
    
    parser.parse();
    
    Ok(parser.nodes)
}
