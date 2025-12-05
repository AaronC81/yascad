use std::{error::Error, fmt::Display, iter::Peekable, ops::RangeInclusive, rc::Rc, slice};

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
    BooleanLiteral(bool),
    VectorLiteral(Vec<Node>),
    VectorRangeLiteral {
        start: Box<Node>,
        end: Box<Node>,
        // TODO: step
    },
    ItReference,

    OperatorApplication {
        name: String,
        arguments: Arguments,

        // Note: the indexes in here might not necessarily line up with children at runtime, since
        // only some of these might evaluate to a manifold
        children: Vec<Node>,
    },
    Call {
        name: String,
        arguments: Arguments,
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
        parameters: Parameters,
        body: Vec<Node>,
    },
    ModuleDefinition {
        name: String,
        parameters: Parameters,
        body: Vec<Node>,
    },

    ForLoop {
        loop_variable: String,
        loop_source: Box<Node>,
        body: Vec<Node>,
    },
    IfConditional {
        condition: Box<Node>,
        true_body: Vec<Node>,
        false_body: Option<Vec<Node>>,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameters {
    pub required: Vec<String>,
    pub optional: Vec<(String, Node)>,
}

impl Parameters {
    /// The total number of required and optional arguments.
    pub fn max_len(&self) -> usize {
        self.required.len() + self.optional.len()
    }

    /// The number of required arguments.
    pub fn min_len(&self) -> usize {
        self.required.len()
    }

    /// The range of allowed argument lengths.
    pub fn len_range(&self) -> RangeInclusive<usize> {
        (self.min_len())..=(self.max_len())
    }

    /// All parameters, required and optional, in the order they'd be expected to be specified.
    pub fn ordered_names(&self) -> impl Iterator<Item = String> {
        self.required.iter().cloned()
            .chain(self.optional.iter().map(|(name, _)| name.clone()))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Arguments {
    pub positional: Vec<Node>,
    pub named: Vec<(String, Node)>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,

    Equals,
    LessThan,
    LessThanOrEquals,
    GreaterThan,
    GreaterThanOrEquals,
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
    RequiredParameterAfterOptionalParameter(String),
    PositionalArgumentAfterNamedArgument,
}

impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseErrorKind::UnexpectedToken(token_kind) => write!(f, "unexpected {token_kind}"),
            ParseErrorKind::UnexpectedEnd => write!(f, "unexpected end-of-file"),
            ParseErrorKind::InvalidNumber => write!(f, "number could not be parsed, possibly out-of-range?"),
            ParseErrorKind::RequiredParameterAfterOptionalParameter(name) => write!(f, "required parameter \"{name}\" appears after optional parameters - required parameters must come first"),
            ParseErrorKind::PositionalArgumentAfterNamedArgument => write!(f, "positional argument appears after named arguments - positional arguments must come first"),
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
            let (name, parameters, body, span) = self.parse_definition()?;
            return Some(Node::new(
                NodeKind::OperatorDefinition {
                    name: name.to_owned(),
                    parameters,
                    body,
                },
                span,
            ))
        }

        // Try parse module definition
        if self.tokens.peek().is_some_and(|token| token.kind == TokenKind::KwModule) {
            let (name, parameters, body, span) = self.parse_definition()?;
            return Some(Node::new(
                NodeKind::ModuleDefinition {
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

        // Try parse `if` statement
        if self.tokens.peek().is_some_and(|token| token.kind == TokenKind::KwIf) {
            return self.parse_if_statement()
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
        self.parse_comparison_expression()
    }

    fn parse_comparison_expression(&mut self) -> Option<(Node, StatementTerminator)> {
        let (mut left, mut terminator) = self.parse_add_sub_expression()?;

        while let Some(Token { kind, .. }) = self.tokens.peek() {
            let op = match kind {
                TokenKind::DoubleEquals => BinaryOperator::Equals,
                TokenKind::LAngle => BinaryOperator::LessThan,
                TokenKind::LAngleEquals => BinaryOperator::LessThanOrEquals,
                TokenKind::RAngle => BinaryOperator::GreaterThan,
                TokenKind::RAngleEquals => BinaryOperator::GreaterThanOrEquals,
                _ => break
            };

            self.tokens.next().unwrap();

            let (right, right_terminator) = self.parse_add_sub_expression()?;
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
                    let (arguments, arguments_span) = self.parse_argument_list()?;
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

                        let (mut items, end_span) = self.parse_bracketed_comma_separated_expression_list(TokenKind::RBracket)?;
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

            TokenKind::LParen => {
                let (node, _) = self.parse_expression()?;
                self.expect(TokenKind::RParen)?;

                Some((node, StatementTerminator::NeedsSemicolon))
            }

            TokenKind::KwTrue => {
                Some((Node::new(NodeKind::BooleanLiteral(true), span), StatementTerminator::NeedsSemicolon))
            }
            TokenKind::KwFalse => {
                Some((Node::new(NodeKind::BooleanLiteral(false), span), StatementTerminator::NeedsSemicolon))
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

    /// Generic parsing logic for the definition of an operator or module.
    /// 
    /// Assumes the leading definition keyword has NOT yet been parsed. It'll be popped but not
    /// validated, just to get its span. It's the caller's responsibility to peek this first to
    /// figure out the kind of definition.
    /// 
    /// Returns, in order:
    ///   - Name of the definition
    ///   - Parameters
    ///   - Body
    ///   - Span of entire definition
    fn parse_definition(&mut self) -> Option<(String, Parameters, Vec<Node>, InputSourceSpan)> {
        let Token { span: start_span, .. } = self.tokens.next().unwrap();

        let (name, _) = self.expect_identifier()?;

        // Parse parameters
        self.expect(TokenKind::LParen)?;
        let (parsed_parameters, _) = self.parse_bracketed_comma_separated_list(TokenKind::RParen, |parser| {
            let (name, _) = parser.expect_identifier()?;

            if parser.tokens.peek().is_some_and(|token| token.kind == TokenKind::Equals) {
                parser.tokens.next().unwrap();
                let (default, _) = parser.parse_expression()?;
                Some((name, Some(default)))
            } else {
                Some((name, None))
            }
        })?;
        
        // Transform "parameters, maybe with defaults" into distinct lists, validating that any
        // optionals appear after required
        let mut encountered_optional = false;
        let mut parameters = Parameters { required: vec![], optional: vec![] };
        for (name, default) in parsed_parameters {
            if let Some(default) = default {
                encountered_optional = true;
                parameters.optional.push((name, default));
            } else {
                if encountered_optional {
                    self.errors.push(ParseError::new(ParseErrorKind::RequiredParameterAfterOptionalParameter(name.clone()), start_span.clone()))
                }

                parameters.required.push(name);
            }
        }

        // Parse body
        let body = self.parse_braced_statement_list()?;
        let body_spans = body
            .iter()
            .map(|item| item.span.clone())
            .collect::<Vec<_>>();

        let span = start_span.union_with(&body_spans);

        Some((name.to_owned(), parameters, body, span))
    }

    fn parse_argument_list(&mut self) -> Option<(Arguments, InputSourceSpan)> {
        self.expect(TokenKind::LParen)?;
        let (parsed_arguments, span) = self.parse_bracketed_comma_separated_list(TokenKind::RParen, |parser| {
            let (expr, _) = parser.parse_expression()?;

            if let NodeKind::Identifier(name) = &expr.kind
                && parser.tokens.peek().is_some_and(|token| token.kind == TokenKind::Equals)
            {
                let name = name.to_owned();
                parser.tokens.next().unwrap();
                let (expr, _) = parser.parse_expression()?;
                Some((expr, Some(name)))
            } else {
                Some((expr, None))
            }
        })?;

        // Positional arguments must appear before named arguments
        let mut encountered_named = false;
        let mut arguments = Arguments { positional: vec![], named: vec![] };
        for (value, name) in parsed_arguments {
            if let Some(name) = name {
                encountered_named = true;
                arguments.named.push((name, value));
            } else {
                if encountered_named {
                    self.errors.push(ParseError::new(ParseErrorKind::PositionalArgumentAfterNamedArgument, span.clone()))
                }

                arguments.positional.push(value);
            }
        }

        Some((arguments, span))
    }

    /// If the next tokens are a field access, e.g. `.x.y`, wrap the given node in these accesses.
    /// Otherwise, return the node unchanged.
    fn parse_any_field_access_suffixes(&mut self, mut value: Node) -> Node {
        while self.tokens.peek().is_some_and(|token| token.kind == TokenKind::Dot) {
            let Token { span: dot_span, .. } = self.tokens.next().unwrap();

            let Some((field, name_span)) = self.expect_identifier()
            else { break };

            value = Node::new(
                NodeKind::FieldAccess {
                    value: Box::new(value),
                    field,
                },
                dot_span.union_with(&[name_span]),
            )
        }

        value
    }

    // Assumes you have already consumed the start of the list (e.g. left paren)
    fn parse_bracketed_comma_separated_list<T>(&mut self, end: TokenKind, parse_fn: impl Fn(&mut Self) -> Option<T>) -> Option<(Vec<T>, InputSourceSpan)> {
        let start_span = self.tokens.peek()?.span.clone();
        let mut end_span = None;

        // Special case for empty list
        if self.tokens.peek().is_some_and(|token| token.kind == TokenKind::RParen) {
            let Token { span, .. } = self.tokens.next().unwrap();
            return Some((vec![], start_span.union_with(&[span])));
        }

        let mut items = vec![];
        loop {
            if let Some(item) = parse_fn(self) {
                items.push(item);
            }

            let Some(separator) = self.tokens.next() else {
                self.errors.push(ParseError::new(ParseErrorKind::UnexpectedEnd, self.source.eof_span()));
                break;
            };

            if separator.kind == TokenKind::Comma {
                // This is the expected separator, nothing specific required - we'll loop round to
                // parse another item.
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

        let mut span = start_span;
        if let Some(end_span) = end_span {
            span = span.union_with(slice::from_ref(&end_span));
        }
        Some((items, span))
    }

    // Assumes you have already consumed the start of the list (e.g. left paren)
    fn parse_bracketed_comma_separated_expression_list(&mut self, end: TokenKind) -> Option<(Vec<Node>, InputSourceSpan)> {
        self.parse_bracketed_comma_separated_list(end, |p|
            p.parse_expression().map(|(n, _)| n))
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

    fn parse_if_statement(&mut self) -> Option<Node> {
        let Token { span: start_span, .. } = self.expect(TokenKind::KwIf)??;
        self.expect(TokenKind::LParen)??;

        let (condition, _) = self.parse_expression()?;
        let Token { span: body_end_span, .. } = self.expect(TokenKind::RParen)??;

        let true_body = self.parse_braced_statement_list()?;
        let false_body =
            if self.tokens.peek().is_some_and(|token| token.kind == TokenKind::KwElse) {
                self.tokens.next().unwrap();
                
                match self.tokens.peek() {
                    Some(Token { kind: TokenKind::KwIf, .. }) => Some(vec![self.parse_if_statement()?]),
                    Some(Token { kind: TokenKind::LBrace, .. }) => Some(self.parse_braced_statement_list()?),
                    
                    Some(Token { kind, span }) => {
                        self.errors.push(ParseError::new(ParseErrorKind::UnexpectedToken(kind.clone()), span.clone()));
                        None
                    }
                    None => {
                        self.errors.push(ParseError::new(ParseErrorKind::UnexpectedEnd, self.source.eof_span()));
                        None
                    }
                }
            } else {
                None
            };

        let span = start_span.union_with(&[body_end_span]);        
        return Some(Node::new(
            NodeKind::IfConditional {
                condition: Box::new(condition),
                true_body,
                false_body,
            },
            span,
        ))
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

    /// Like [`expect`] but specifically expects an identifier, and returns its string value.
    fn expect_identifier(&mut self) -> Option<(String, InputSourceSpan)> {
        match self.tokens.next() {
            Some(Token { kind: TokenKind::Identifier(id), span }) => Some((id, span)),
            Some(Token { kind, span }) => {
                self.errors.push(ParseError::new(ParseErrorKind::UnexpectedToken(kind), span));
                None
            }
            None => {
                self.errors.push(ParseError::new(ParseErrorKind::UnexpectedEnd, self.source.eof_span()));
                None
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::rc::Rc;

    use crate::{Arguments, InputSource, Node, NodeKind, Parser, tokenize};

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
                    arguments: Arguments {
                        positional: vec![
                            Node::new(NodeKind::NumberLiteral(10.0), source.span(5, 2)),
                            Node::new(NodeKind::NumberLiteral(20.5), source.span(9, 4)),
                            Node::new(NodeKind::NumberLiteral(30.0), source.span(15, 2)),
                        ],
                        named: vec![],
                    }
                },
                source.span(0, 18)
            )
        )
    }
}
