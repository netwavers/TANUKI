use crate::frontend::ast::{Node, TokenType};
use crate::frontend::tokenizer::Tokenizer;
use super::MdNode;

#[allow(dead_code, unused_mut, unused_variables, unused_assignments, non_snake_case)]
pub struct MarkdownParser<'a> {
    pub tokenizer: &'a mut Tokenizer,
    pub token: crate::frontend::ast::Token,
    pub source_path: String,
    pub file_hash: Option<String>,
    pub context_stack: Vec<String>,
    pub nodes: Vec<MdNode>,
}

impl<'a> MarkdownParser<'a> {
    pub fn new(tokenizer: &'a mut Tokenizer, source_path: String) -> Self {
        let mut file_hash = None;
        // トークナイザーの全文字からハッシュを事前に探す
        let text: String = tokenizer.chars.iter().collect();
        if let Some(pos) = text.find("<!-- Tanuki-Hash: ") {
            let sub = &text[pos + 18..];
            if let Some(end) = sub.find(" -->") {
                file_hash = Some(sub[..end].to_string());
            }
        }

        Self {
            tokenizer,
            token: crate::frontend::ast::Token::default(),
            source_path,
            file_hash,
            context_stack: Vec::new(),
            nodes: Vec::new(),
        }
    }

    fn on_header(&mut self, level: u8) {
        let full_text = self.tokenizer.get_and_clear_last_content();
        let title = full_text.trim_start_matches('#').trim().to_string();
        
        while self.context_stack.len() >= level as usize {
            self.context_stack.pop();
        }

        let display_title = if level == 1 && self.file_hash.is_some() {
            let h = self.file_hash.as_ref().unwrap();
            let short_h = if h.len() > 8 { &h[..8] } else { h };
            format!("{} ({})", title, short_h)
        } else {
            title.clone()
        };

        // 親IDの決定論的ハッシュ解決
        let parent_id = if self.context_stack.is_empty() {
            None
        } else {
            Some(calculate_fnv1a(&self.context_stack.join(" > ")))
        };

        self.context_stack.push(display_title);
        
        let h = self.file_hash.as_deref().unwrap_or("unknown");
        self.nodes.push(MdNode {
            id: format!("{}-{}-{}", h, level, self.nodes.len()),
            source_path: self.source_path.clone(),
            file_hash: self.file_hash.clone(),
            context_path: self.context_stack.join(" > "),
            parent_id,
            title: title,
            content: full_text,
            kind: super::MdNodeKind::Header(level),
        });
    }

    fn on_paragraph(&mut self) {
        let content = self.tokenizer.get_and_clear_last_content();
        let h = self.file_hash.as_deref().unwrap_or("unknown");
        
        let parent_id = if self.context_stack.is_empty() {
            None
        } else {
            Some(calculate_fnv1a(&self.context_stack.join(" > ")))
        };

        self.nodes.push(MdNode {
            id: format!("{}-p-{}", h, self.nodes.len()),
            source_path: self.source_path.clone(),
            file_hash: self.file_hash.clone(),
            context_path: self.context_stack.join(" > "),
            parent_id,
            title: "Paragraph".into(),
            content: content,
            kind: super::MdNodeKind::Paragraph,
        });
    }

    fn on_code_block(&mut self) {
        let content = self.tokenizer.get_and_clear_last_content();
        let h = self.file_hash.as_deref().unwrap_or("unknown");
        
        let parent_id = if self.context_stack.is_empty() {
            None
        } else {
            Some(calculate_fnv1a(&self.context_stack.join(" > ")))
        };

        self.nodes.push(MdNode {
            id: format!("{}-code-{}", h, self.nodes.len()),
            source_path: self.source_path.clone(),
            file_hash: self.file_hash.clone(),
            context_path: self.context_stack.join(" > "),
            parent_id,
            title: "Code Block".into(),
            content: content,
            kind: super::MdNodeKind::CodeBlock,
        });
    }

    fn document(&mut self) -> Option<Node<'a>> {
        let mut ret_val : Option<Node<'a>> = None;
        let mut ret_val_0 : Option<Node<'a>> = None;
        while (self.token.token_type != TokenType::TEof) {
            let last_pos = self.tokenizer.pos;
            ret_val_0 = self.element();
            if self.tokenizer.pos == last_pos {
                // 進展がない場合は強制的に1トークン消費して無限ループを避ける
                self.token = self.tokenizer.get_token();
            }
        }
        return ret_val;
    }
    
    fn element(&mut self) -> Option<Node<'a>> {
        let mut ret_val : Option<Node<'a>> = None;
        let mut ret_val_0 : Option<Node<'a>> = None;
        match self.token.token_type {
            TokenType::H1 | TokenType::H2 | TokenType::H3 | TokenType::H4 | TokenType::H5 | TokenType::H6 => {
                ret_val_0 = self.header();
            }
            TokenType::CODE => {
                ret_val_0 = self.code_block();
            }
            TokenType::TEXT => {
                ret_val_0 = self.paragraph();
            }
            TokenType::SPACE | TokenType::TNl => {
                ret_val_0 = self.empty_line();
            }
            _ => {}
        }
        return ret_val;
    }
    
    fn header(&mut self) -> Option<Node<'a>> {
        let mut ret_val : Option<Node<'a>> = None;
        let mut ret_val_0 : Option<Node<'a>> = None;
        match self.token.token_type {
            TokenType::H1 => {
                ret_val_0 = self.h1();
            }
            TokenType::H2 => {
                ret_val_0 = self.h2();
            }
            TokenType::H3 => {
                ret_val_0 = self.h3();
            }
            TokenType::H4 => {
                ret_val_0 = self.h4();
            }
            TokenType::H5 => {
                ret_val_0 = self.h5();
            }
            TokenType::H6 => {
                ret_val_0 = self.h6();
            }
            TokenType::HR => {
                self.token = self.tokenizer.get_token();
            }
            TokenType::TEXT => {
                ret_val_0 = self.paragraph();
            }
            TokenType::CODE => {
                ret_val_0 = self.code_block();
            }
            TokenType::SPACE | TokenType::TNl => {
                self.token = self.tokenizer.get_token();
            }
            _ => {}
        }
        return ret_val;
    }
    
    fn h1(&mut self) -> Option<Node<'a>> {
        let mut ret_val : Option<Node<'a>> = None;
        let mut ret_val_0 : Option<Node<'a>> = None;
        if (self.token.token_type != TokenType::H1) {
            return None;
        }
        self.token = self.tokenizer.get_token();
        while (self.token.token_type == TokenType::SPACE) {
            if (self.token.token_type != TokenType::SPACE) {
                return None;
            }
            self.token = self.tokenizer.get_token();
        }
        ret_val_0 = self.text_line();
        self.on_header(1);
        if self.token.token_type == TokenType::TNl {
            self.token = self.tokenizer.get_token();
        }
        return ret_val;
    }
    
    fn h2(&mut self) -> Option<Node<'a>> {
        let mut ret_val : Option<Node<'a>> = None;
        let mut ret_val_0 : Option<Node<'a>> = None;
        if (self.token.token_type != TokenType::H2) {
            return None;
        }
        self.token = self.tokenizer.get_token();
        while (self.token.token_type == TokenType::SPACE) {
            if (self.token.token_type != TokenType::SPACE) {
                return None;
            }
            self.token = self.tokenizer.get_token();
        }
        ret_val_0 = self.text_line();
        self.on_header(2);
        if self.token.token_type == TokenType::TNl {
            self.token = self.tokenizer.get_token();
        }
        return ret_val;
    }
    
    fn h3(&mut self) -> Option<Node<'a>> {
        let mut ret_val : Option<Node<'a>> = None;
        let mut ret_val_0 : Option<Node<'a>> = None;
        if (self.token.token_type != TokenType::H3) {
            return None;
        }
        self.token = self.tokenizer.get_token();
        while (self.token.token_type == TokenType::SPACE) {
            if (self.token.token_type != TokenType::SPACE) {
                return None;
            }
            self.token = self.tokenizer.get_token();
        }
        ret_val_0 = self.text_line();
        self.on_header(3);
        if self.token.token_type == TokenType::TNl {
            self.token = self.tokenizer.get_token();
        }
        return ret_val;
    }
    
    fn h4(&mut self) -> Option<Node<'a>> {
        let mut ret_val : Option<Node<'a>> = None;
        let mut ret_val_0 : Option<Node<'a>> = None;
        if (self.token.token_type != TokenType::H4) {
            return None;
        }
        self.token = self.tokenizer.get_token();
        while (self.token.token_type == TokenType::SPACE) {
            if (self.token.token_type != TokenType::SPACE) {
                return None;
            }
            self.token = self.tokenizer.get_token();
        }
        ret_val_0 = self.text_line();
        self.on_header(4);
        if self.token.token_type == TokenType::TNl {
            self.token = self.tokenizer.get_token();
        }
        return ret_val;
    }
    
    fn h5(&mut self) -> Option<Node<'a>> {
        let mut ret_val : Option<Node<'a>> = None;
        let mut ret_val_0 : Option<Node<'a>> = None;
        if (self.token.token_type != TokenType::H5) {
            return None;
        }
        self.token = self.tokenizer.get_token();
        while (self.token.token_type == TokenType::SPACE) {
            if (self.token.token_type != TokenType::SPACE) {
                return None;
            }
            self.token = self.tokenizer.get_token();
        }
        ret_val_0 = self.text_line();
        self.on_header(5);
        if self.token.token_type == TokenType::TNl {
            self.token = self.tokenizer.get_token();
        }
        return ret_val;
    }
    
    fn h6(&mut self) -> Option<Node<'a>> {
        let mut ret_val : Option<Node<'a>> = None;
        let mut ret_val_0 : Option<Node<'a>> = None;
        if (self.token.token_type != TokenType::H6) {
            return None;
        }
        self.token = self.tokenizer.get_token();
        while (self.token.token_type == TokenType::SPACE) {
            if (self.token.token_type != TokenType::SPACE) {
                return None;
            }
            self.token = self.tokenizer.get_token();
        }
        ret_val_0 = self.text_line();
        self.on_header(6);
        if self.token.token_type == TokenType::TNl {
            self.token = self.tokenizer.get_token();
        }
        return ret_val;
    }
    
    fn text_line(&mut self) -> Option<Node<'a>> {
        let mut ret_val : Option<Node<'a>> = None;
        while (self.token.token_type == TokenType::TEXT) || (self.token.token_type == TokenType::SPACE) {
            match self.token.token_type {
                TokenType::TEXT => {
                    if (self.token.token_type != TokenType::TEXT) {
                        return None;
                    }
                    self.token = self.tokenizer.get_token();
                }
                TokenType::SPACE => {
                    if (self.token.token_type != TokenType::SPACE) {
                        return None;
                    }
                    self.token = self.tokenizer.get_token();
                }
                _ => {}
            }
        }
        return ret_val;
    }
    
    fn paragraph(&mut self) -> Option<Node<'a>> {
        let mut ret_val : Option<Node<'a>> = None;
        if (self.token.token_type != TokenType::TEXT) {
            return None;
        }
        self.token = self.tokenizer.get_token();
        while (self.token.token_type == TokenType::TEXT) || (self.token.token_type == TokenType::SPACE) || (self.token.token_type == TokenType::TNl) {
            if self.token.token_type == TokenType::HR {
                break;
            }
            match self.token.token_type {
                TokenType::TEXT => {
                    if (self.token.token_type != TokenType::TEXT) {
                        return None;
                    }
                    self.token = self.tokenizer.get_token();
                }
                TokenType::SPACE => {
                    if (self.token.token_type != TokenType::SPACE) {
                        return None;
                    }
                    self.token = self.tokenizer.get_token();
                }
                TokenType::TNl => {
                    if (self.token.token_type != TokenType::TNl) {
                        return None;
                    }
                    self.token = self.tokenizer.get_token();
                }
                _ => {}
            }
        }
        self.on_paragraph();
        if self.token.token_type == TokenType::TNl {
            self.token = self.tokenizer.get_token();
        }
        return ret_val;
    }
    
    fn code_block(&mut self) -> Option<Node<'a>> {
        let mut ret_val : Option<Node<'a>> = None;
        let mut ret_val_0 : Option<Node<'a>> = None;
        if (self.token.token_type != TokenType::CODE) {
            return None;
        }
        self.token = self.tokenizer.get_token();
        if (self.token.token_type == TokenType::TEXT) {
            if (self.token.token_type != TokenType::TEXT) {
                return None;
            }
            self.token = self.tokenizer.get_token();
        }
        if (self.token.token_type != TokenType::TNl) {
            return None;
        }
        self.token = self.tokenizer.get_token();
        while (self.token.token_type == TokenType::TEXT) || (self.token.token_type == TokenType::SPACE) || (self.token.token_type == TokenType::TNl) {
            ret_val_0 = self.body_line();
        }
        if (self.token.token_type != TokenType::CODE) {
            return None;
        }
        self.token = self.tokenizer.get_token();
        if (self.token.token_type != TokenType::TNl) {
            return None;
        }
        self.on_code_block();
        self.token = self.tokenizer.get_token();
        return ret_val;
    }
    
    fn body_line(&mut self) -> Option<Node<'a>> {
        let mut ret_val : Option<Node<'a>> = None;
        match self.token.token_type {
            TokenType::TEXT => {
                if (self.token.token_type != TokenType::TEXT) {
                    return None;
                }
                self.token = self.tokenizer.get_token();
            }
            TokenType::SPACE => {
                if (self.token.token_type != TokenType::SPACE) {
                    return None;
                }
                self.token = self.tokenizer.get_token();
            }
            TokenType::TNl => {
                if (self.token.token_type != TokenType::TNl) {
                    return None;
                }
                self.token = self.tokenizer.get_token();
            }
            _ => {}
        }
        return ret_val;
    }
    
    fn empty_line(&mut self) -> Option<Node<'a>> {
        let mut ret_val : Option<Node<'a>> = None;
        while (self.token.token_type == TokenType::SPACE) {
            if (self.token.token_type != TokenType::SPACE) {
                return None;
            }
            self.token = self.tokenizer.get_token();
        }
        if (self.token.token_type == TokenType::TNl) {
            self.token = self.tokenizer.get_token();
        }
        return ret_val;
    }
    
    pub fn parse(&mut self) -> Option<Node<'a>> {
        let mut ret_val : Option<Node<'a>> = None;
        ret_val = self.document();
        return ret_val;
    }
    
}

fn calculate_fnv1a(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
