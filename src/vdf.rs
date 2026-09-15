//! Minimal reader for Valve's plain-text KeyValues ("VDF") format, enough to
//! read `loginusers.vdf` and `libraryfolders.vdf`: quoted `"key" "value"`
//! pairs, `"key" { ... }` nested blocks, and `//` line comments.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone)]
pub enum Node {
    Str(String),
    Block(BTreeMap<String, Node>),
}

impl Node {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Node::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_block(&self) -> Option<&BTreeMap<String, Node>> {
        match self {
            Node::Block(b) => Some(b),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Node> {
        self.as_block()?.get(key)
    }
}

#[derive(Debug)]
pub struct ParseError(String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "vdf parse error: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    src: &'a str,
}

#[derive(Debug, PartialEq)]
enum Token {
    Str(String),
    Open,
    Close,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Lexer {
            chars: src.char_indices().peekable(),
            src,
        }
    }

    fn skip_ignorable(&mut self) {
        loop {
            match self.chars.peek() {
                Some((_, c)) if c.is_whitespace() => {
                    self.chars.next();
                }
                Some((i, '/')) => {
                    let mut lookahead = self.src[*i..].chars();
                    lookahead.next();
                    if lookahead.next() == Some('/') {
                        for (_, c) in self.chars.by_ref() {
                            if c == '\n' {
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
    }

    fn next_token(&mut self) -> Option<Token> {
        self.skip_ignorable();
        let (start, c) = *self.chars.peek()?;
        match c {
            '{' => {
                self.chars.next();
                Some(Token::Open)
            }
            '}' => {
                self.chars.next();
                Some(Token::Close)
            }
            '"' => {
                self.chars.next();
                let mut s = String::new();
                for (_, ch) in self.chars.by_ref() {
                    if ch == '"' {
                        break;
                    }
                    s.push(ch);
                }
                Some(Token::Str(s))
            }
            _ => {
                // Unquoted token (not used by the files we read, but handled
                // for robustness): read until whitespace/brace.
                let mut end = start;
                while let Some((i, ch)) = self.chars.peek() {
                    if ch.is_whitespace() || *ch == '{' || *ch == '}' {
                        break;
                    }
                    end = *i + ch.len_utf8();
                    self.chars.next();
                }
                Some(Token::Str(self.src[start..end].to_string()))
            }
        }
    }
}

fn parse_block(lexer: &mut Lexer) -> Result<BTreeMap<String, Node>, ParseError> {
    let mut map = BTreeMap::new();
    loop {
        match lexer.next_token() {
            None | Some(Token::Close) => return Ok(map),
            Some(Token::Open) => {
                return Err(ParseError("unexpected '{' where a key was expected".into()));
            }
            Some(Token::Str(key)) => match lexer.next_token() {
                Some(Token::Open) => {
                    let child = parse_block(lexer)?;
                    map.insert(key, Node::Block(child));
                }
                Some(Token::Str(value)) => {
                    map.insert(key, Node::Str(value));
                }
                other => {
                    return Err(ParseError(format!(
                        "expected value or block after key {key:?}, got {other:?}"
                    )));
                }
            },
        }
    }
}

/// Parses a VDF document. Valve's format has a single root key wrapping the
/// whole file (e.g. `"users" { ... }`); this returns that root node's value.
pub fn parse(src: &str) -> Result<Node, ParseError> {
    let mut lexer = Lexer::new(src);
    match lexer.next_token() {
        Some(Token::Str(_root_key)) => match lexer.next_token() {
            Some(Token::Open) => Ok(Node::Block(parse_block(&mut lexer)?)),
            Some(Token::Str(v)) => Ok(Node::Str(v)),
            other => Err(ParseError(format!("expected root value, got {other:?}"))),
        },
        other => Err(ParseError(format!("expected root key, got {other:?}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_blocks_and_comments() {
        let src = r#"
            "users"
            {
                // a comment
                "76561198083183641"
                {
                    "AccountName"   "burnskull55"
                    "PersonaName"   "Burnskull55"
                    "Timestamp"     "1789331114"
                }
            }
        "#;
        let root = parse(src).unwrap();
        let user = root.get("76561198083183641").unwrap();
        assert_eq!(user.get("PersonaName").unwrap().as_str(), Some("Burnskull55"));
    }
}
