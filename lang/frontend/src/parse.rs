use std::{error::Error, fmt::Display, iter::Peekable, rc::Rc, slice};

use miette::Diagnostic;

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
    VectorLiteral(Vec<Node>),
    VectorRangeLiteral {
        start: Box<Node>,
        end: Box<Node>,
        // TODO: step
    },
    ItReference,

    OperatorApplication {
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

    Binding {
        name: String,
        value: Box<Node>,
    },
    FieldAccess {
        value: Box<Node>,
        field: String,
    },

    BinaryOperation {
        left: Box<Node>,
        right: Box<Node>,
        op: BinaryOperator,
    },
    UnaryNegate(Box<Node>),

    OperatorDefinition {
        name: String,
        parameters: Vec<String>, // TODO: optional parameters
        body: Vec<Node>,
    },

    ForLoop {
        loop_variable: String,
        loop_source: Box<Node>,
        body: Vec<Node>,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum StatementTerminator {
    // Needs a semicolon for termination
    NeedsSemicolon,

    /// Ends with a brace, so semicolon is optional
    Braced,
}

#[derive(Debug, Clone, PartialEq, Diagnostic)]
pub struct ParseError {
    pub kind: ParseErrorKind,

    #[source_code]
    #[label]
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

#[derive(Debug, Clone, PartialEq)]
pub enum ParseErrorKind {
    UnexpectedToken(TokenKind),
    UnexpectedEnd,
    InvalidNumber,
    InvalidOperatorParameter,
}

impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseErrorKind::UnexpectedToken(token_kind) => write!(f, "unexpected {token_kind}"),
            ParseErrorKind::UnexpectedEnd => write!(f, "unexpected end-of-file"),
            ParseErrorKind::InvalidNumber => write!(f, "number could not be parsed, possibly out-of-range?"),
            ParseErrorKind::InvalidOperatorParameter => write!(f, "invalid operator parameter"),
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
        // Try parse operator definition
        if self.tokens.peek().is_some_and(|token| token.kind == TokenKind::KwOperator) {
            let Token { span: start_span, .. } = self.tokens.next().unwrap();
            
            let Some(name_token) = self.tokens.next()
            else {
                self.errors.push(ParseError::new(ParseErrorKind::UnexpectedEnd, self.source.eof_span()));
                return None;
            };

            let TokenKind::Identifier(name) = &name_token.kind
            else {
                self.errors.push(ParseError::new(ParseErrorKind::UnexpectedToken(name_token.kind), name_token.span));
                return None;
            };

            // Parse parameters
            self.expect(TokenKind::LParen)?;
            let (parameters, _) = self.parse_bracketed_comma_separated_list(TokenKind::RParen)?;
            let parameters = parameters.into_iter()
                .map(|node| match node.kind {
                    NodeKind::Identifier(id) => id,
                    _ => {
                        self.errors.push(ParseError::new(ParseErrorKind::InvalidOperatorParameter, node.span));
                        "DUMMY".to_owned()
                    }
                })
                .collect::<Vec<_>>();

            // Parse body
            let body = self.parse_braced_statement_list()?;
            let body_spans = body
                .iter()
                .map(|item| item.span.clone())
                .collect::<Vec<_>>();

            let span = start_span.union_with(&body_spans);
            return Some(Node::new(
                NodeKind::OperatorDefinition {
                    name: name.to_owned(),
                    parameters,
                    body,
                },
                span,
            ))
        }

        // Try parse `for` loop
        if self.tokens.peek().is_some_and(|token| token.kind == TokenKind::KwFor) {
            let Token { span: start_span, .. } = self.tokens.next().unwrap();

            self.expect(TokenKind::LParen)?;
            
            // TODO: REALLY need to break this sequence out into a function
            let Some(Token { kind, span: name_span }) = self.tokens.next()
            else {
                self.errors.push(ParseError::new(ParseErrorKind::UnexpectedEnd, self.source.eof_span()));
                return None
            };
            let TokenKind::Identifier(loop_variable) = &kind
            else {
                self.errors.push(ParseError::new(ParseErrorKind::UnexpectedToken(kind), name_span));
                return None
            };

            self.expect(TokenKind::Equals)?;
            let (loop_source, _) = self.parse_expression()?;
            self.expect(TokenKind::RParen)?;

            let body = self.parse_braced_statement_list()?;
            let body_spans = body
                .iter()
                .map(|item| item.span.clone())
                .collect::<Vec<_>>();

            let span = start_span.union_with(&body_spans);
            return Some(Node::new(
                NodeKind::ForLoop {
                    loop_variable: loop_variable.to_owned(),
                    loop_source: Box::new(loop_source),
                    body,
                },
                span,
            ))
        }

        let (mut expr, mut terminator) = self.parse_expression()?;

        // Parse assignment
        if let Node { span, kind: NodeKind::Identifier(id) } = &expr
            && self.tokens.peek().is_some_and(|token| token.kind == TokenKind::Equals)
        {
            self.tokens.next().unwrap(); // discard equals

            let (value, value_terminator) = self.parse_expression()?;
            let binding_span = span.union_with(slice::from_ref(&value.span));
            expr = Node::new(NodeKind::Binding {
                name: id.clone(),
                value: Box::new(value),
            }, binding_span);
            terminator = value_terminator;
        }

        match terminator {
            StatementTerminator::NeedsSemicolon => {
                self.expect(TokenKind::Semicolon)?;
            },
            StatementTerminator::Braced => {
                if self.tokens.peek().is_some_and(|token| token.kind == TokenKind::Semicolon) {
                    self.tokens.next();
                }
            },
        };
        
        Some(expr)
    }

    fn parse_expression(&mut self) -> Option<(Node, StatementTerminator)> {
        self.parse_add_sub_expression()
    }

    fn parse_add_sub_expression(&mut self) -> Option<(Node, StatementTerminator)> {
        let (mut left, mut terminator) = self.parse_mul_div_expression()?;

        while self.tokens.peek().is_some_and(|token| token.kind == TokenKind::Plus || token.kind == TokenKind::Minus) {
            let Token { kind, .. } = self.tokens.next().unwrap();
            let op = match kind {
                TokenKind::Plus => BinaryOperator::Add,
                TokenKind::Minus => BinaryOperator::Subtract,
                _ => unreachable!(),
            };

            let (right, right_terminator) = self.parse_mul_div_expression()?;
            let span = left.span.union_with(slice::from_ref(&right.span));
            left = Node::new(
                NodeKind::BinaryOperation {
                    left: Box::new(left),
                    right: Box::new(right),
                    op,
                },
                span,
            );
            terminator = right_terminator;
        }

        Some((left, terminator))
    }

    fn parse_mul_div_expression(&mut self) -> Option<(Node, StatementTerminator)> {
        let (mut left, mut terminator) = self.parse_bottom_expression()?;

        while self.tokens.peek().is_some_and(|token| token.kind == TokenKind::Star || token.kind == TokenKind::ForwardSlash) {
            let Token { kind, .. } = self.tokens.next().unwrap();
            let op = match kind {
                TokenKind::Star => BinaryOperator::Multiply,
                TokenKind::ForwardSlash => BinaryOperator::Divide,
                _ => unreachable!(),
            };

            let (right, right_terminator) = self.parse_bottom_expression()?;
            let span = left.span.union_with(slice::from_ref(&right.span));
            left = Node::new(
                NodeKind::BinaryOperation {
                    left: Box::new(left),
                    right: Box::new(right),
                    op,
                },
                span,
            );
            terminator = right_terminator;
        }

        Some((left, terminator))
    }

    fn parse_bottom_expression(&mut self) -> Option<(Node, StatementTerminator)> {
        let Token { kind, span } = self.tokens.next()?;
        match kind {
            TokenKind::Identifier(id) => {
                if self.tokens.peek().is_some_and(|token| token.kind == TokenKind::LParen) {
                    // An identifier immediately followed by lparen is a call
                    self.tokens.next().unwrap(); // discard lparen
                    let (arguments, arguments_span) = self.parse_bracketed_comma_separated_list(TokenKind::RParen)?;
                    let call_span = span.union_with(&[arguments_span]);

                    // If, after the call, there's immediately another identifier, then this call
                    // was actually an operator application. We just parsed the operator, now parse
                    // its only child.
                    //
                    // Likewise, if there's a brace, we parsed a operator with multiple children.
                    if self.tokens.peek().is_some_and(|token| matches!(token.kind, TokenKind::Identifier(_))) {
                        let (child, operator) = self.parse_expression()?;
                        Some((
                            Node::new(NodeKind::OperatorApplication {
                                name: id,
                                arguments,
                                children: vec![child],
                            }, call_span),
                            operator,
                        ))
                    } else if self.tokens.peek().is_some_and(|token| matches!(token.kind, TokenKind::LBrace)) {
                        let children = self.parse_braced_statement_list()?;
                        Some((
                            Node::new(NodeKind::OperatorApplication {
                                name: id,
                                arguments,
                                children,
                            }, call_span),
                            StatementTerminator::Braced,
                        ))
                    } else {
                        Some((
                            self.parse_any_field_access_suffixes(
                                Node::new(NodeKind::Call {
                                    name: id,
                                    arguments,
                                }, call_span)
                            ),
                            StatementTerminator::NeedsSemicolon,
                        ))
                    }
                } else {
                    // Just a normal identifier usage
                    Some((
                        self.parse_any_field_access_suffixes(
                            Node::new(NodeKind::Identifier(id), span)
                        ),
                        StatementTerminator::NeedsSemicolon,
                    ))
                }
            }

            TokenKind::Number(num) => {
                if let Ok(value) = num.parse() {
                    Some((Node::new(NodeKind::NumberLiteral(value), span), StatementTerminator::NeedsSemicolon))
                } else {
                    self.errors.push(ParseError::new(ParseErrorKind::InvalidNumber, span.clone()));
                    Some((Node::new(NodeKind::NumberLiteral(0.0), span), StatementTerminator::NeedsSemicolon))
                }
            }
            
            TokenKind::LBracket => {
                // TODO: support parsing empty vector

                // Parse the first item ourselves, because we need to check whether this is an
                // item-based vector or a range vector.
                let (first_item, _) = self.parse_expression()?;

                match self.tokens.peek() {
                    // Single-item vector
                    Some(Token { kind: TokenKind::RBracket, span: end_span }) => {
                        let vector_span = span.union_with(slice::from_ref(end_span));
                        Some((Node::new(NodeKind::VectorLiteral(vec![first_item]), vector_span), StatementTerminator::NeedsSemicolon))
                    },

                    // Multi-item vector
                    Some(Token { kind: TokenKind::Comma, .. }) => {
                        self.tokens.next().unwrap();

                        let (mut items, end_span) = self.parse_bracketed_comma_separated_list(TokenKind::RBracket)?;
                        items.insert(0, first_item);

                        let vector_span = span.union_with(slice::from_ref(&end_span));
                        Some((Node::new(NodeKind::VectorLiteral(items), vector_span), StatementTerminator::NeedsSemicolon))
                    },

                    // Range vector
                    Some(Token { kind: TokenKind::Colon, .. }) => {
                        self.tokens.next().unwrap();

                        let (end_item, _) = self.parse_expression()?;
                        self.expect(TokenKind::RBracket)?;

                        let vector_span = span.union_with(slice::from_ref(&end_item.span));
                        Some((
                            Node::new(NodeKind::VectorRangeLiteral {
                                start: Box::new(first_item),
                                end: Box::new(end_item),
                            }, vector_span),
                            StatementTerminator::NeedsSemicolon
                        ))
                    }

                    Some(Token { span, .. }) => {
                        self.errors.push(ParseError::new(ParseErrorKind::UnexpectedToken(kind), span.clone()));
                        None
                    },
                    None => {
                        self.errors.push(ParseError::new(ParseErrorKind::UnexpectedEnd, self.source.eof_span()));
                        None
                    },
                }
            }

            TokenKind::KwIt => {
                Some((
                    self.parse_any_field_access_suffixes(
                        Node::new(NodeKind::ItReference, span)
                    ),
                    StatementTerminator::NeedsSemicolon
                ))
            }

            TokenKind::Minus => {
                let (value, terminator) = self.parse_bottom_expression()?;
                let span = span.union_with(slice::from_ref(&value.span));
                Some((
                    Node::new(NodeKind::UnaryNegate(Box::new(value)), span),
                    terminator,
                ))
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

    /// If the next tokens are a field access, e.g. `.x.y`, wrap the given node in these accesses.
    /// Otherwise, return the node unchanged.
    fn parse_any_field_access_suffixes(&mut self, mut value: Node) -> Node {
        while self.tokens.peek().is_some_and(|token| token.kind == TokenKind::Dot) {
            let Token { span: dot_span, .. } = self.tokens.next().unwrap();

            let Some(Token { span: name_span, kind }) = self.tokens.next()
            else {
                self.errors.push(ParseError::new(ParseErrorKind::UnexpectedEnd, self.source.eof_span()));
                break;
            };

            let TokenKind::Identifier(name) = &kind
            else {
                self.errors.push(ParseError::new(ParseErrorKind::UnexpectedToken(kind), name_span));
                break;
            };

            value = Node::new(
                NodeKind::FieldAccess {
                    value: Box::new(value),
                    field: name.to_owned(),
                },
                dot_span.union_with(&[name_span]),
            )
        }

        value
    }

    // Assumes you have already consumed the start of the list (e.g. left paren)
    fn parse_bracketed_comma_separated_list(&mut self, end: TokenKind) -> Option<(Vec<Node>, InputSourceSpan)> {
        let start_span = self.tokens.peek()?.span.clone();
        let mut end_span = None;

        // Special case for empty list
        if self.tokens.peek().is_some_and(|token| token.kind == TokenKind::RParen) {
            let Token { span, .. } = self.tokens.next().unwrap();
            return Some((vec![], start_span.union_with(&[span])));
        }

        let mut items = vec![];
        loop {
            if let Some((arg, _)) = self.parse_expression() {
                items.push(arg);
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
                if self.tokens.peek().is_some_and(|token| token.kind == end) {
                    let Token { span, .. } = self.tokens.next().unwrap();
                    end_span = Some(span);
                    break;
                }
            } else if separator.kind == end {
                end_span = Some(separator.span);
                break;
            } else {
                self.errors.push(ParseError::new(ParseErrorKind::UnexpectedToken(separator.kind), separator.span));
            }
        }

        let mut spans = items.iter()
            .map(|node| node.span.clone())
            .collect::<Vec<_>>();
        if let Some(end_span) = end_span {
            spans.push(end_span);
        }
        Some((items, start_span.union_with(&spans)))
    }

    fn parse_braced_statement_list(&mut self) -> Option<Vec<Node>> {
        self.expect(TokenKind::LBrace)?;

        let mut stmts = vec![];
        loop {
            if self.tokens.peek().is_some_and(|token| token.kind == TokenKind::RBrace) {
                self.tokens.next().unwrap();
                break
            } else if self.tokens.peek().is_none() {
                self.errors.push(ParseError::new(ParseErrorKind::UnexpectedEnd, self.source.eof_span()));
                break
            } else if let Some(stmt) = self.parse_statement() {
                stmts.push(stmt);
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
        assert_eq!(parser.errors, vec![]);
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
