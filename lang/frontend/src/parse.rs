use std::{error::Error, fmt::Display, iter::Peekable, rc::Rc};

use crate::{InputSource, InputSourceSpan, Token, TokenKind};

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub kind: NodeKind,
    pub span: InputSourceSpan,
}

impl Node {
    pub fn new(kind: NodeKind, span: InputSourceSpan) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Identifier(String),
    NumberLiteral(f64),

    ModifierApplication {
        name: String,
        arguments: Vec<Node>,

        // Note: the indexes in here might not necessarily line up with children at runtime, since
        // only some of these might evaluate to a manifold
        children: Vec<Node>,
    },
    Call {
        name: String,
        arguments: Vec<Node>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub span: InputSourceSpan,
}

impl ParseError {
    pub fn new(kind: ParseErrorKind, span: InputSourceSpan) -> Self {
        Self { kind, span }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: emit file and line number, but need to have a way to work out line number first
        write!(f, "{}", self.kind)
    }
}
impl Error for ParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    UnexpectedToken(TokenKind),
    UnexpectedEnd,
    InvalidNumber,
}

impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseErrorKind::UnexpectedToken(token_kind) => write!(f, "unexpected {token_kind}"),
            ParseErrorKind::UnexpectedEnd => write!(f, "unexpected end-of-file"),
            ParseErrorKind::InvalidNumber => write!(f, "number could not be parsed, possibly out-of-range?"),
        }
    }
}

pub struct Parser<I: Iterator<Item = Token>> {
    source: Rc<InputSource>,
    tokens: Peekable<I>,
    pub errors: Vec<ParseError>,
}

impl<I: Iterator<Item = Token>> Parser<I> {
    pub fn new(source: Rc<InputSource>, tokens: impl IntoIterator<IntoIter = I>) -> Self {
        Self {
            source,
            tokens: tokens.into_iter().peekable(),
            errors: vec![],
        }
    }

    pub fn parse_statements(&mut self) -> Vec<Node> {
        let mut stmts = vec![];

        // Assumes `parse_statement` always makes forward progress through the token iterator,
        // even in the worst error case
        while self.tokens.peek().is_some() {
            if let Some(stmt) = self.parse_statement() {
                stmts.push(stmt);
            }
        }

        stmts
    }

    // The parser methods here all return an `Option` because they try to do error recovery.
    //
    // If they return `Some`, either all parsing was valid and the returned node is correct, or
    // they emitted an error but are attempting to recover by returning some dummy value.
    // If they return `None`, then they (or a subparser) already emitted an error.

    pub fn parse_statement(&mut self) -> Option<Node> {
        let expr = self.parse_expression()?;

        // TODO: permit not needing this if we just had a closing brace
        self.expect(TokenKind::Semicolon)?;

        Some(expr)
    }

    fn parse_expression(&mut self) -> Option<Node> {
        let Token { kind, span } = self.tokens.next()?;
        match kind {
            TokenKind::Identifier(id) => {
                // TODO: multi-child modifier application (e.g. `difference`)

                if self.tokens.peek().is_some_and(|token| token.kind == TokenKind::LParen) {
                    // An identifier immediately followed by lparen is a call
                    let (arguments, arguments_span) = self.parse_argument_list()?;
                    let call_span = span.union_with(&[arguments_span]);

                    // If, after the call, there's immediately another identifier, then this call
                    // was actually a modifier application. We just parsed the modifier, now parse
                    // its only child.
                    //
                    // Likewise, if there's a brace, we parsed a modifier with multiple children.
                    if self.tokens.peek().is_some_and(|token| matches!(token.kind, TokenKind::Identifier(_))) {
                        let child = self.parse_expression()?;
                        Some(Node::new(NodeKind::ModifierApplication {
                            name: id,
                            arguments,
                            children: vec![child]
                        }, call_span))
                    } else if self.tokens.peek().is_some_and(|token| matches!(token.kind, TokenKind::LBrace)) {
                        let children = self.parse_braced_statement_list()?;
                        Some(Node::new(NodeKind::ModifierApplication {
                            name: id,
                            arguments,
                            children,
                        }, call_span))
                    } else {
                        Some(Node::new(NodeKind::Call {
                            name: id,
                            arguments,
                        }, call_span))
                    }
                } else {
                    // Just a normal identifier usage
                    Some(Node::new(NodeKind::Identifier(id), span))
                }
            }

            TokenKind::Number(num) => {
                if let Ok(value) = num.parse() {
                    Some(Node::new(NodeKind::NumberLiteral(value), span))
                } else {
                    self.errors.push(ParseError::new(ParseErrorKind::InvalidNumber, span.clone()));
                    Some(Node::new(NodeKind::NumberLiteral(0.0), span))
                }
            }

            _ => {
                self.errors.push(ParseError::new(
                    ParseErrorKind::UnexpectedToken(kind),
                    span,
                ));
                None
            }
        }
    }

    fn parse_argument_list(&mut self) -> Option<(Vec<Node>, InputSourceSpan)> {
        let start_span = self.tokens.peek()?.span.clone();
        let mut end_span = None;

        self.expect(TokenKind::LParen)?;

        let mut arguments = vec![];
        loop {
            if let Some(arg) = self.parse_expression() {
                arguments.push(arg);
            }

            let Some(separator) = self.tokens.next() else {
                self.errors.push(ParseError::new(ParseErrorKind::UnexpectedEnd, self.source.eof_span()));
                break;
            };

            if separator.kind == TokenKind::Comma {
                // This is the expected separator, nothing specific required - we'll loop round to
                // parse another argument.
                //
                // Trailing commas are allowed though, so check for rparen
                if self.tokens.peek().is_some_and(|token| token.kind == TokenKind::RParen) {
                    let Token { span, .. } = self.tokens.next().unwrap();
                    end_span = Some(span);
                    break;
                }
            } else if separator.kind == TokenKind::RParen {
                end_span = Some(separator.span);
                break;
            } else {
                self.errors.push(ParseError::new(ParseErrorKind::UnexpectedToken(separator.kind), separator.span));
            }
        }

        let mut spans = arguments.iter()
            .map(|node| node.span.clone())
            .collect::<Vec<_>>();
        if let Some(end_span) = end_span {
            spans.push(end_span);
        }
        Some((arguments, start_span.union_with(&spans)))
    }

    fn parse_braced_statement_list(&mut self) -> Option<Vec<Node>> {
        self.expect(TokenKind::LBrace)?;

        let mut stmts = vec![];
        loop {
            if self.tokens.peek().is_some_and(|token| token.kind == TokenKind::RBrace) {
                break
            } else if self.tokens.peek().is_none() {
                self.errors.push(ParseError::new(ParseErrorKind::UnexpectedEnd, self.source.eof_span()));
                break
            } else {
                if let Some(stmt) = self.parse_statement() {
                    stmts.push(stmt);
                }
            }
        }

        Some(stmts)
    }

    /// Consume a token which is expected to be of a certain kind, generating an error if it's not.
    /// 
    /// Returns:
    ///   - `Some(Some(token))` if the token matched the expectation
    ///   - `Some(None)` if a token was consumed, but not the expected kind of token
    ///   - `None` if there was no token (EOF)
    fn expect(&mut self, kind: TokenKind) -> Option<Option<Token>> {
        let token = self.tokens.next();
        if token.as_ref().is_some_and(|token| token.kind == kind) {
            Some(Some(token.unwrap()))
        } else if let Some(Token { kind, span }) = token {
            self.errors.push(ParseError::new(ParseErrorKind::UnexpectedToken(kind), span));
            Some(None)
        } else {
            self.errors.push(ParseError::new(ParseErrorKind::UnexpectedEnd, self.source.eof_span()));
            None
        }
    }
}

#[cfg(test)]
mod test {
    use std::rc::Rc;

    use crate::{InputSource, Node, NodeKind, Parser, tokenize};

    #[test]
    fn test_basic_parse() {
        let source = Rc::new(InputSource::new_string(
            "cube(10, 20.5, 30);".to_owned()
        ));
        let (tokens, errors) = tokenize(source.clone());
        assert!(errors.is_empty());

        let mut parser = Parser::new(source.clone(), tokens);
        let stmts = parser.parse_statements();
        assert_eq!(stmts.len(), 1);
        let stmt = &stmts[0];

        assert_eq!(
            stmt,
            &Node::new(
                NodeKind::Call {
                    name: "cube".to_owned(),
                    arguments: vec![
                        Node::new(NodeKind::NumberLiteral(10.0), source.span(5, 2)),
                        Node::new(NodeKind::NumberLiteral(20.5), source.span(9, 4)),
                        Node::new(NodeKind::NumberLiteral(30.0), source.span(15, 2)),
                    ],
                },
                source.span(0, 18)
            )
        )
    }
}
