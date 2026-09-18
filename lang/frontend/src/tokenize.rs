use std::{error::Error, fmt::Display, iter::Peekable, rc::Rc};

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
    String(String), // Note: Value is the content of the string, does not include outer quotes

    KwIt,
    KwOperator,
    KwModule,
    KwFunction,
    KwFor,
    KwIf,
    KwEach,
    KwElse,
    KwTrue,
    KwFalse,
    KwNull,
    KwUndef,
    KwLet,

    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LAngle,
    RAngle,
    
    Comma,
    Semicolon,
    Dot,
    Colon,
    Ellipsis,
    QuestionMark,
    ExclamationMark,

    DoubleAmpersand,
    DoublePipe,

    Plus,
    Minus,
    ForwardSlash,
    Star,
    Percent,
    Caret,

    Equals,
    ExclamationMarkEquals,
    DoubleEquals,
    LAngleEquals,
    RAngleEquals,
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Identifier(id) => write!(f, "identifier \"{id}\""),
            TokenKind::Number(number) => write!(f, "number \"{number}\""),
            TokenKind::String(_) => write!(f, "string literal"),

            TokenKind::LParen => write!(f, "left paren"),
            TokenKind::RParen => write!(f, "right paren"),
            TokenKind::LBrace => write!(f, "left brace"),
            TokenKind::RBrace => write!(f, "right brace"),
            TokenKind::LBracket => write!(f, "left bracket"),
            TokenKind::RBracket => write!(f, "right bracket"),
            TokenKind::LAngle => write!(f, "less-than"),
            TokenKind::RAngle => write!(f, "greater-than"),

            TokenKind::KwIt => write!(f, "keyword \"it\""),
            TokenKind::KwOperator => write!(f, "keyword \"operator\""),
            TokenKind::KwModule => write!(f, "keyword \"module\""),
            TokenKind::KwFunction => write!(f, "keyword \"function\""),
            TokenKind::KwFor => write!(f, "keyword \"for\""),
            TokenKind::KwIf => write!(f, "keyword \"if\""),
            TokenKind::KwEach => write!(f, "keyword \"each\""),
            TokenKind::KwElse => write!(f, "keyword \"else\""),
            TokenKind::KwTrue => write!(f, "keyword \"true\""),
            TokenKind::KwFalse => write!(f, "keyword \"false\""),
            TokenKind::KwNull => write!(f, "keyword \"null\""),
            TokenKind::KwUndef => write!(f, "keyword \"undef\""),
            TokenKind::KwLet => write!(f, "keyword \"let\""),

            TokenKind::Comma => write!(f, "comma"),
            TokenKind::Semicolon => write!(f, "semicolon"),
            TokenKind::Dot => write!(f, "dot"),
            TokenKind::Colon => write!(f, "colon"),
            TokenKind::Ellipsis => write!(f, "ellipsis"),
            TokenKind::QuestionMark => write!(f, "question mark"),
            TokenKind::ExclamationMark => write!(f, "exclamation mark"),

            TokenKind::DoubleAmpersand => write!(f, "double-ampersand"),
            TokenKind::DoublePipe => write!(f, "double-pipe"),

            TokenKind::Plus => write!(f, "plus"),
            TokenKind::Minus => write!(f, "minus"),
            TokenKind::ForwardSlash => write!(f, "forward slash"),
            TokenKind::Star => write!(f, "star"),
            TokenKind::Percent => write!(f, "percent"),
            TokenKind::Caret => write!(f, "caret"),

            TokenKind::Equals => write!(f, "equals"),
            TokenKind::ExclamationMarkEquals => write!(f, "not-equals"),
            TokenKind::DoubleEquals => write!(f, "double-equals"),
            TokenKind::LAngleEquals => write!(f, "less-than-or-equals"),
            TokenKind::RAngleEquals => write!(f, "greater-than-or-equals"),
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
    UnterminatedString,
    UnknownEscapeSequence(String),
}

impl Display for TokenizeErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenizeErrorKind::UnexpectedChar(c) => write!(f, "unexpected character {c}"),
            TokenizeErrorKind::UnterminatedString => write!(f, "string is not terminated"),
            TokenizeErrorKind::UnknownEscapeSequence(seq) => write!(f, "unknown escape sequence {seq}"),
        }
    }
}

pub fn tokenize(source: Rc<InputSource>) -> (Vec<Token>, Vec<TokenizeError>) {
    let mut tokens = vec![];
    let mut errors = vec![];

    let source_for_chars = source.clone();
    let mut chars = source_for_chars.content.chars().enumerate().peekable();

    'outer: while let Some((start_index, char)) = chars.next() {
        match char {
            _ if char.is_ascii_digit() => {
                let buffer = parse_number(&mut chars, char);
                let length = buffer.len();
                tokens.push(Token::new(TokenKind::Number(buffer), source.span(start_index, length)));
            }

            _ if char.is_alphabetic() || char == '_' || char == '$' => {
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

            // Block comment
            '/' if chars.peek().is_some_and(|(_, char)| *char == '*') => {
                chars.next();

                loop {
                    let Some((_, this_char)) = chars.next()
                    else { break };

                    if this_char == '*' && chars.peek().is_some_and(|(_, char)| *char == '/') {
                        chars.next();
                        break;
                    }
                }
            }

            '"' => {
                // Keep taking characters until we find the closing quote
                let mut buffer = String::new();

                let last_index = loop {
                    let Some((index, char)) = chars.next() else {
                        errors.push(TokenizeError::new(
                            TokenizeErrorKind::UnterminatedString,
                            source.eof_span(),
                        ));
                        break 'outer;
                    };
                    
                    match char {
                        '"' => break index,
                        '\\' => {
                            let Some((_, escaped_char)) = chars.next() else {
                                errors.push(TokenizeError::new(
                                    TokenizeErrorKind::UnterminatedString,
                                    source.eof_span(),
                                ));
                                break 'outer;
                            };

                            match escaped_char {
                                '"' => buffer.push('"'),
                                '\\' => buffer.push('\\'),
                                'n' => buffer.push('\n'),
                                _ => errors.push(TokenizeError::new(
                                    TokenizeErrorKind::UnknownEscapeSequence(escaped_char.to_string()),
                                    source.span(index, 2),
                                )),
                            }
                        },

                        char => buffer.push(char),
                    }
                };

                tokens.push(Token::new(TokenKind::String(buffer), source.span(start_index, last_index - start_index)));
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
            '.' => {
                if chars.peek().is_some_and(|(_, char)| *char == '.') {
                    chars.next();
                    if chars.peek().is_some_and(|(_, char)| *char == '.') {
                        chars.next();

                        tokens.push(Token::new(TokenKind::Ellipsis, source.span(start_index, 3)));
                    } else {
                        errors.push(TokenizeError::new(
                            TokenizeErrorKind::UnexpectedChar(char),
                            source.span(start_index, 1),
                        ))
                    }
                } else if chars.peek().is_some_and(|(_, char)| char.is_digit(10)) {
                    let buffer = parse_number(&mut chars, '.');
                    let length = buffer.len();
                    tokens.push(Token::new(TokenKind::Number(buffer), source.span(start_index, length)));
                } else {
                    tokens.push(Token::new(TokenKind::Dot, source.span(start_index, 1)))
                }
            }
            ':' => {
                tokens.push(Token::new(TokenKind::Colon, source.span(start_index, 1)))
            }
            '?' => {
                tokens.push(Token::new(TokenKind::QuestionMark, source.span(start_index, 1)))
            }
            '!' => {
                if chars.peek().is_some_and(|(_, c)| *c == '=') {
                    chars.next().unwrap();
                    tokens.push(Token::new(TokenKind::ExclamationMarkEquals, source.span(start_index, 2)))
                } else {
                    tokens.push(Token::new(TokenKind::ExclamationMark, source.span(start_index, 1)))
                }
            }

            '&' if chars.peek().is_some_and(|(_, char)| *char == '&') => {
                chars.next();
                tokens.push(Token::new(TokenKind::DoubleAmpersand, source.span(start_index, 1)))
            }
            '|' if chars.peek().is_some_and(|(_, char)| *char == '|') => {
                chars.next();
                tokens.push(Token::new(TokenKind::DoublePipe, source.span(start_index, 1)))
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
            '%' => {
                tokens.push(Token::new(TokenKind::Percent, source.span(start_index, 1)))
            }
            '^' => {
                tokens.push(Token::new(TokenKind::Caret, source.span(start_index, 1)))
            }

            '=' => {
                if chars.peek().is_some_and(|(_, c)| *c == '=') {
                    chars.next().unwrap();
                    tokens.push(Token::new(TokenKind::DoubleEquals, source.span(start_index, 2)))
                } else {
                    tokens.push(Token::new(TokenKind::Equals, source.span(start_index, 1)))
                }
            },
            '<' => {
                if chars.peek().is_some_and(|(_, c)| *c == '=') {
                    chars.next().unwrap();
                    tokens.push(Token::new(TokenKind::LAngleEquals, source.span(start_index, 2)))
                } else {
                    tokens.push(Token::new(TokenKind::LAngle, source.span(start_index, 1)))
                }
            },
            '>' => {
                if chars.peek().is_some_and(|(_, c)| *c == '=') {
                    chars.next().unwrap();
                    tokens.push(Token::new(TokenKind::RAngleEquals, source.span(start_index, 2)))
                } else {
                    tokens.push(Token::new(TokenKind::RAngle, source.span(start_index, 1)))
                }
            },

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

fn parse_number<I: Iterator<Item = (usize, char)>>(chars: &mut Peekable<I>, start_char: char) -> String {
    let mut buffer = start_char.to_string();
    let mut had_decimal_point = false;

    while let Some((_, char)) = chars.peek() {
        if char.is_ascii_digit() {
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

    buffer
}

fn lookup_keyword(name: &str) -> Option<TokenKind> {
    match name {
        "it" => Some(TokenKind::KwIt),
        "operator" => Some(TokenKind::KwOperator),
        "module" => Some(TokenKind::KwModule),
        "function" => Some(TokenKind::KwFunction),
        "for" => Some(TokenKind::KwFor),
        "if" => Some(TokenKind::KwIf),
        "each" => Some(TokenKind::KwEach),
        "else" => Some(TokenKind::KwElse),
        "true" => Some(TokenKind::KwTrue),
        "false" => Some(TokenKind::KwFalse),
        "null" => Some(TokenKind::KwNull),
        "undef" => Some(TokenKind::KwUndef),
        "let" => Some(TokenKind::KwLet),
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
