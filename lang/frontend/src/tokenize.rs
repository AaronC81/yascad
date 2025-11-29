use std::{error::Error, fmt::Display, rc::Rc};

use miette::Diagnostic;

use crate::{InputSource, InputSourceSpan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: InputSourceSpan,
}

impl Token {
    pub fn new(kind: TokenKind, span: InputSourceSpan) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Identifier(String),
    Number(String),

    KwIt,
    KwOperator,

    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    
    Comma,
    Semicolon,
    Equals,
    Dot,

    Plus,
    Minus,
    ForwardSlash,
    Star,
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Identifier(id) => write!(f, "identifier \"{id}\""),
            TokenKind::Number(number) => write!(f, "number \"{number}\""),
            TokenKind::LParen => write!(f, "left paren"),
            TokenKind::RParen => write!(f, "right paren"),
            TokenKind::LBrace => write!(f, "left brace"),
            TokenKind::RBrace => write!(f, "right brace"),
            TokenKind::LBracket => write!(f, "left bracket"),
            TokenKind::RBracket => write!(f, "right bracket"),

            TokenKind::KwIt => write!(f, "keyword \"it\""),
            TokenKind::KwOperator => write!(f, "keyword \"operator\""),

            TokenKind::Comma => write!(f, "comma"),
            TokenKind::Semicolon => write!(f, "semicolon"),
            TokenKind::Equals => write!(f, "equals"),
            TokenKind::Dot => write!(f, "dot"),

            TokenKind::Plus => write!(f, "plus"),
            TokenKind::Minus => write!(f, "minus"),
            TokenKind::ForwardSlash => write!(f, "forward slash"),
            TokenKind::Star => write!(f, "star"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Diagnostic)]
#[diagnostic()]
pub struct TokenizeError {
    pub kind: TokenizeErrorKind,

    #[source_code]
    #[label]
    pub span: InputSourceSpan,
}

impl TokenizeError {
    pub fn new(kind: TokenizeErrorKind, span: InputSourceSpan) -> Self {
        Self { kind, span }
    }
}

impl Display for TokenizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: emit file and line number, but need to have a way to work out line number first
        write!(f, "{}", self.kind)
    }
}
impl Error for TokenizeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenizeErrorKind {
    UnexpectedChar(char),
}

impl Display for TokenizeErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenizeErrorKind::UnexpectedChar(c) => write!(f, "unexpected character {c}"),
        }
    }
}

pub fn tokenize(source: Rc<InputSource>) -> (Vec<Token>, Vec<TokenizeError>) {
    let mut tokens = vec![];
    let mut errors = vec![];

    let source_for_chars = source.clone();
    let mut chars = source_for_chars.content.chars().enumerate().peekable();

    while let Some((start_index, char)) = chars.next() {
        match char {
            _ if char.is_digit(10) => {
                let mut buffer = char.to_string();
                let mut had_decimal_point = false;

                while let Some((_, char)) = chars.peek() {
                    if char.is_digit(10) {
                        let (_, char) = chars.next().unwrap();
                        buffer.push(char)
                    } else if !had_decimal_point && *char == '.' {
                        chars.next().unwrap();
                        had_decimal_point = true;
                        buffer.push('.');
                    } else {
                        break;
                    }
                }

                let length = buffer.len();
                tokens.push(Token::new(TokenKind::Number(buffer), source.span(start_index, length)));
            }

            _ if char.is_alphabetic() || char == '_' => {
                let mut buffer = char.to_string();

                while let Some((_, char)) = chars.peek() {
                    if char.is_alphanumeric() || *char == '_' {
                        let (_, char) = chars.next().unwrap();
                        buffer.push(char)
                    } else {
                        break;
                    }
                }

                let span = source.span(start_index, buffer.len());
                let token_kind = match lookup_keyword(&buffer) {
                    Some(kw) => kw,
                    None => TokenKind::Identifier(buffer),
                };
                tokens.push(Token::new(token_kind, span));
            }

            // Line comment
            '/' if chars.peek().is_some_and(|(_, char)| *char == '/') => {
                loop {
                    let Some((_, char)) = chars.next()
                    else { break };

                    if char == '\n' {
                        break
                    }
                }
            }

            '(' => {
                tokens.push(Token::new(TokenKind::LParen, source.span(start_index, 1)))
            },
            ')' => {
                tokens.push(Token::new(TokenKind::RParen, source.span(start_index, 1)))
            },
            '{' => {
                tokens.push(Token::new(TokenKind::LBrace, source.span(start_index, 1)))
            },
            '}' => {
                tokens.push(Token::new(TokenKind::RBrace, source.span(start_index, 1)))
            },
            '[' => {
                tokens.push(Token::new(TokenKind::LBracket, source.span(start_index, 1)))
            },
            ']' => {
                tokens.push(Token::new(TokenKind::RBracket, source.span(start_index, 1)))
            },
            ',' => {
                tokens.push(Token::new(TokenKind::Comma, source.span(start_index, 1)))
            },
            ';' => {
                tokens.push(Token::new(TokenKind::Semicolon, source.span(start_index, 1)))
            },
            '=' => {
                tokens.push(Token::new(TokenKind::Equals, source.span(start_index, 1)))
            },
            '.' => {
                tokens.push(Token::new(TokenKind::Dot, source.span(start_index, 1)))
            }

            '+' => {
                tokens.push(Token::new(TokenKind::Plus, source.span(start_index, 1)))
            }
            '-' => {
                tokens.push(Token::new(TokenKind::Minus, source.span(start_index, 1)))
            }
            '/' => {
                tokens.push(Token::new(TokenKind::ForwardSlash, source.span(start_index, 1)))
            }
            '*' => {
                tokens.push(Token::new(TokenKind::Star, source.span(start_index, 1)))
            }

            _ if char.is_whitespace() => {
                // Skip
            }

            _ => {
                errors.push(TokenizeError::new(
                    TokenizeErrorKind::UnexpectedChar(char),
                    source.span(start_index, 1),
                ))
            }
        }
    }

    (tokens, errors)
}

fn lookup_keyword(name: &str) -> Option<TokenKind> {
    match name {
        "it" => Some(TokenKind::KwIt),
        "operator" => Some(TokenKind::KwOperator),
        _ => None,
    }
}

#[cfg(test)]
mod test {
    use std::rc::Rc;

    use crate::{InputSource, Token, TokenKind, tokenize};

    #[test]
    fn test_basic_tokenize() {
        let source = Rc::new(InputSource::new_string(
            "cube(10, 20.5, 30);".to_owned()
        ));
        let (tokens, errors) = tokenize(source.clone());

        assert!(errors.is_empty());
        assert_eq!(
            tokens,
            vec![
                Token::new(TokenKind::Identifier("cube".to_owned()), source.span(0, 4)),
                Token::new(TokenKind::LParen,                        source.span(4, 1)),
                Token::new(TokenKind::Number("10".to_string()),      source.span(5, 2)),
                Token::new(TokenKind::Comma,                         source.span(7, 1)),
                Token::new(TokenKind::Number("20.5".to_string()),    source.span(9, 4)),
                Token::new(TokenKind::Comma,                         source.span(13, 1)),
                Token::new(TokenKind::Number("30".to_string()),      source.span(15, 2)),
                Token::new(TokenKind::RParen,                        source.span(17, 1)),
                Token::new(TokenKind::Semicolon,                     source.span(18, 1)),
            ]
        )
    }
}
