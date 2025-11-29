use std::{cell::RefCell, collections::HashMap, rc::Rc};

use manifold_rs::Manifold;
use yascad_frontend::{BinaryOperator, InputSourceSpan, Node, NodeKind};

use crate::{RuntimeError, RuntimeErrorKind, lexical_scope::LexicalScope, manifold_table::{ManifoldDisposition, ManifoldTable, ManifoldTableIndex}, object::Object};

/// The context of whatever node is currently executing, to encapsulate surrounding state.
#[derive(Clone, Debug)]
pub struct ExecutionContext<'c> {
    /// The manifold (if any) which `it` currently refers to.
    it_manifold: ItManifold<'c>,

    /// If executing an operator body, its `children`.
    operator_children: Option<&'c [ManifoldTableIndex]>,

    /// The current lexical scope.
    lexical_scope: Rc<RefCell<LexicalScope>>,

    /// The current map of arguments available within a module/operator body.
    /// This is distinct from scope so we don't look up to parent frames.
    arguments: HashMap<String, Object>,
}

impl<'c> ExecutionContext<'c> {
    pub fn new() -> Self {
        Self {
            it_manifold: ItManifold::None,
            operator_children: None,
            lexical_scope: Rc::new(RefCell::new(LexicalScope::new_root())),
            arguments: HashMap::new(),
        }
    }

    pub fn with_it_manifold<'r>(&self, it_manifold: ItManifold<'r>) -> ExecutionContext<'r>
    where 'c: 'r
    {
        ExecutionContext {
            it_manifold,
            ..self.clone()
        }
    }

    pub fn with_operator_children<'r>(&self, operator_children: Option<&'r [ManifoldTableIndex]>) -> ExecutionContext<'r>
    where 'c: 'r
    {
        ExecutionContext {
            operator_children,
            ..self.clone()
        }
    }

    pub fn with_deeper_scope(&'_ self) -> ExecutionContext<'_> {
        ExecutionContext {
            lexical_scope: Rc::new(RefCell::new(LexicalScope::new(self.lexical_scope.clone()))),
            ..self.clone()
        }
    }

    pub fn with_arguments(&'_ self, arguments: HashMap<String, Object>) -> ExecutionContext<'_> {
        ExecutionContext {
            arguments,
            ..self.clone()
        }
    }
}

pub struct Interpreter {
    manifold_table: ManifoldTable,
    circle_segments: i32,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            manifold_table: ManifoldTable::new(),

            // TODO: add $fn setter support
            circle_segments: 20,
        }
    }

    pub fn build_top_level_manifold(&self) -> Manifold {
        let mut result = Manifold::new();
        for (manifold, disposition) in self.manifold_table.iter_manifolds() {
            if *disposition == ManifoldDisposition::Physical {
                result = result.union(manifold);
            }
        }

        result
    }

    pub fn interpret_top_level(&mut self, nodes: &[Node]) -> Result<(), RuntimeError> {
        let ctx = ExecutionContext::new();
        for node in nodes {
            self.interpret(node, &ctx)?;
        }
        Ok(())
    }

    pub fn interpret(&mut self, node: &Node, ctx: &ExecutionContext) -> Result<Object, RuntimeError> {
        match &node.kind {
            NodeKind::Identifier(id) => {
                // Arguments have highest priority - they probably need to be less separated in
                // future, but, eh
                if let Some(object) = ctx.arguments.get(id) {
                    return Ok(object.clone());
                }

                ctx.lexical_scope.borrow()
                    .get_binding(id)
                    .ok_or_else(|| RuntimeError::new(
                        RuntimeErrorKind::UndefinedIdentifier(id.to_owned()),
                        node.span.clone(),
                    ))
            },

            NodeKind::NumberLiteral(num) => {
                Ok(Object::Number(*num))
            },

            NodeKind::VectorLiteral(items) => {
                Ok(Object::Vector(
                    items.iter()
                        .map(|node| self.interpret(node, ctx))
                        .collect::<Result<Vec<_>, _>>()?
                ))
            },

            NodeKind::VectorRangeLiteral { start, end } => {
                let start = self.interpret(&start, ctx)?.as_number(node.span.clone())?;
                let end = self.interpret(&end, ctx)?.as_number(node.span.clone())?;

                if end < start {
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::FlippedRange,
                        node.span.clone(),
                    ));
                }

                let mut current = start;
                let mut items = vec![Object::Number(current)];
                while current < end {
                    current += 1.0;
                    items.push(Object::Number(current));
                }

                Ok(Object::Vector(items))
            }

            NodeKind::ItReference => {
                match ctx.it_manifold {
                    ItManifold::Some(manifold_table_index) => {
                        Ok(Object::Manifold(manifold_table_index.clone()))
                    },
                    ItManifold::UnsupportedNotOneChild => {
                        Err(RuntimeError::new(
                            RuntimeErrorKind::ItReferenceUnsupportedNotOneChild,
                            node.span.clone(),
                        ))
                    },
                    ItManifold::None => {
                        Err(RuntimeError::new(
                            RuntimeErrorKind::ItReferenceInvalid,
                            node.span.clone(),
                        ))
                    },
                }
            },

            NodeKind::OperatorApplication { name, arguments, children } => {
                let all_children = children.iter()
                    .map(|child| self.interpret(child, &ctx.with_it_manifold(ItManifold::None)))
                    .collect::<Result<Vec<_>, _>>()?;
                let manifold_children = Self::filter_objects_to_manifolds(all_children);

                let it_manifold =
                    if manifold_children.len() == 1 {
                        ItManifold::Some(&manifold_children.first().unwrap())
                    } else {
                        ItManifold::UnsupportedNotOneChild
                    };

                let arguments = arguments.iter()
                    .map(|arg| self.interpret(arg, &ctx.with_it_manifold(it_manifold)))
                    .collect::<Result<Vec<_>, _>>()?;
                
                // We handle user-defined operators and built-in operators differently.
                //
                // User-defined operators can use `children` to access a new copy of the children.
                // To implement this, we create virtual manifolds with all of the children
                // rendered already. The user code never gets access to these manifolds - only
                // copies of it - and these virtual manifolds are destroyed afterwards.
                // (See the implementation for the `children` built-in function.)
                //
                // Built-in operators can do their own manifold table manipulation, so these are
                // directly given the physical manifold indexes. They can do whatever they like with
                // them.
                if let Some(user_operator) = ctx.lexical_scope.borrow().get_operator(name) {
                    let NodeKind::OperatorDefinition { body, parameters, name: _ } = &user_operator.kind.clone()
                    else { unreachable!() };

                    // Validate number of arguments so forthcoming `zip` is definitely balanced
                    if arguments.len() != parameters.len() {
                        return Err(RuntimeError::new(
                            RuntimeErrorKind::IncorrectArity {
                                expected: parameters.len()..=parameters.len(),
                                actual: arguments.len(),
                            },
                            node.span.clone(),
                        ));
                    }

                    // Convert to hash, that's what the context expects
                    let arguments = arguments.into_iter()
                        .zip(parameters)
                        .map(|(arg, param)| (param.to_owned(), arg))
                        .collect::<HashMap<_, _>>();

                    let temporary_virtual_manifolds = manifold_children.into_iter()
                        .map(|index| {
                            let (m, _) = self.manifold_table.remove(index);
                            self.manifold_table.add(m, ManifoldDisposition::Virtual)
                        })
                        .collect::<Vec<_>>();

                    let result_manifolds = Self::filter_objects_to_manifolds(
                        self.interpret_body(body, &ctx
                            .with_it_manifold(ItManifold::None)
                            .with_operator_children(Some(&temporary_virtual_manifolds))
                            .with_deeper_scope()
                            .with_arguments(arguments))?
                    );
                    let result_manifold = self.union_manifolds(result_manifolds, node.span.clone())?;

                    for index in temporary_virtual_manifolds {
                        self.manifold_table.remove(index);
                    }

                    Ok(Object::Manifold(result_manifold))
                } else {
                    let manifold = self.apply_builtin_operator(name, &arguments, manifold_children, node.span.clone())?;
                    Ok(Object::Manifold(manifold))
                }
            }

            NodeKind::Call { name, arguments } => {
                let arguments = arguments.iter()
                    .map(|arg| self.interpret(arg, ctx))
                    .collect::<Result<Vec<_>, _>>()?;
                self.call_builtin_function(name, &arguments, ctx.operator_children, node.span.clone())
            },
            
            NodeKind::Binding { name, value } => {
                let value = self.interpret(&value, ctx)?;

                // As far as a language user is concerned, bindings and arguments existing in the
                // same scope
                if ctx.arguments.contains_key(name) {
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::DuplicateBinding(name.to_owned()),
                        node.span.clone(),
                    ))
                }

                if !ctx.lexical_scope.borrow_mut().add_binding(name.to_owned(), value.clone()) {
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::DuplicateBinding(name.to_owned()),
                        node.span.clone(),
                    ))
                }

                Ok(value)
            },

            NodeKind::FieldAccess { value, field } => {
                let value = self.interpret(&value, ctx)?;

                if let Some(field_value) = value.get_field(field, &self.manifold_table) {
                    Ok(field_value)
                } else {
                    Err(RuntimeError::new(
                        RuntimeErrorKind::UndefinedField { field: field.clone(), ty: value.describe_type() },
                        node.span.clone(),
                    ))
                }
            },

            NodeKind::BinaryOperation { left, right, op } => {
                let left = self.interpret(&left, ctx)?.as_number(node.span.clone())?;
                let right = self.interpret(&right, ctx)?.as_number(node.span.clone())?;

                let result = match op {
                    BinaryOperator::Add => left + right,
                    BinaryOperator::Subtract => left - right,
                    BinaryOperator::Multiply => left * right,
                    BinaryOperator::Divide => left / right,
                };

                Ok(Object::Number(result))
            },

            NodeKind::UnaryNegate(value) => {
                let value = self.interpret(&value, ctx)?.as_number(node.span.clone())?;
                Ok(Object::Number(-value))
            },

            NodeKind::OperatorDefinition { name, .. } => {
                if !ctx.lexical_scope.borrow_mut().add_operator(name.to_owned(), node.clone()) {
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::DuplicateBinding(name.to_owned()),
                        node.span.clone(),
                    ))
                }
                Ok(Object::Null)
            },

            NodeKind::ForLoop { loop_variable, loop_source, body } => {
                let loop_source = self.interpret(&loop_source, ctx)?.into_vector(node.span.clone())?;

                let mut result_objects = vec![];
                for item in loop_source {
                    let ctx = ctx.with_deeper_scope();
                    if !ctx.lexical_scope.borrow_mut().add_binding(loop_variable.clone(), item) {
                        return Err(RuntimeError::new(
                            RuntimeErrorKind::DuplicateBinding(loop_variable.to_owned()),
                            node.span.clone(),
                        ))
                    }
                    
                    result_objects.extend(self.interpret_body(body, &ctx)?)
                }

                let result_manifold = self.union_manifolds(
                    Self::filter_objects_to_manifolds(result_objects),
                    node.span.clone(),
                )?;

                Ok(Object::Manifold(result_manifold))
            }
        }
    }

    fn interpret_body(&mut self, nodes: &[Node], ctx: &ExecutionContext) -> Result<Vec<Object>, RuntimeError> {
        nodes.iter()
            .map(|node| self.interpret(node, ctx))
            .collect()
    }

    fn call_builtin_function(&mut self, name: &str, arguments: &[Object], operator_children: Option<&[ManifoldTableIndex]>, span: InputSourceSpan) -> Result<Object, RuntimeError> {
        match name {
            "cube" => {
                let (x, y, z) = Self::get_vec3_from_arguments(arguments, span)?;
                Ok(Object::Manifold(self.manifold_table.add(Manifold::cube(x, y, z, false), ManifoldDisposition::Physical)))
            }

            "cylinder" => {
                // TODO: needs to support diameters or cone forms
                if arguments.len() != 2 {
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::IncorrectArity { expected: 2..=2, actual: arguments.len() },
                        span,
                    ));
                };

                let height = arguments[0].as_number(span.clone())?;
                let radius = arguments[1].as_number(span.clone())?;

                Ok(Object::Manifold(self.manifold_table.add(Manifold::cylinder(radius, height, self.circle_segments, false), ManifoldDisposition::Physical)))
            }

            "copy" => {
                if arguments.len() != 1 {
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::IncorrectArity { expected: 1..=1, actual: arguments.len() },
                        span,
                    ));
                }

                let manifold_index = arguments[0].clone().into_manifold(span)?;
                let manifold = self.manifold_table.get(&manifold_index);

                // Even if it's being copied in a virtual disposition, we can make it physical here.
                // The `buffer` will "downgrade" it later.
                let copied_manifold = self.manifold_table.add(manifold.clone(), ManifoldDisposition::Physical);
                Ok(Object::Manifold(copied_manifold))
            }

            // TODO: specific children selectors
            "children" => {
                let Some(children) = operator_children
                else {
                    return Err(RuntimeError::new(RuntimeErrorKind::ChildrenInvalid, span));
                };

                // The children are temporary virtual manifolds.
                // Copy them as physical and then build a union of all of the copies.
                let copied_children = children.iter()
                    .map(|child| {
                        let m = self.manifold_table.get(child).clone();
                        self.manifold_table.add(m, ManifoldDisposition::Physical)
                    })
                    .collect::<Vec<_>>();

                Ok(Object::Manifold(self.union_manifolds(copied_children, span)?))
            }

            "__debug" => {
                println!("{arguments:#?}");
                Ok(Object::Null)
            }

            _ => Err(RuntimeError::new(
                RuntimeErrorKind::UndefinedIdentifier(name.to_owned()),
                span,
            ))
        }
    }

    fn apply_builtin_operator(&mut self, name: &str, arguments: &[Object], mut children: Vec<ManifoldTableIndex>, span: InputSourceSpan) -> Result<ManifoldTableIndex, RuntimeError> {
        match name {
            "translate" => {
                let (x, y, z) = Self::get_vec3_from_arguments(arguments, span.clone())?;
                let manifold = self.union_manifolds(children, span)?;
                Ok(self.manifold_table.map(manifold, |m| m.translate(x, y, z)))
            }

            "union" => {
                self.union_manifolds(children, span)
            }

            "difference" => {
                if children.len() == 0 {
                    // TODO: should create an empty manifold, but don't know what disposition it should have
                    todo!()
                }
            
                let minuend = children.remove(0);
                if children.is_empty() {
                    return Ok(minuend);
                }

                let subtrahend = self.union_manifolds(children, span)?;
                let (subtrahend, _) = self.manifold_table.remove(subtrahend);

                Ok(self.manifold_table.map(minuend, |m| m.difference(&subtrahend)))
            }

            "buffer" => {
                let manifold = self.union_manifolds(children, span)?;
                let (manifold, _) = self.manifold_table.remove(manifold);
                
                Ok(self.manifold_table.add(manifold, ManifoldDisposition::Virtual))
            }

            _ => Err(RuntimeError::new(
                RuntimeErrorKind::UndefinedIdentifier(name.to_owned()),
                span,
            ))
        }
    }

    fn get_vec3_from_arguments(arguments: &[Object], span: InputSourceSpan) -> Result<(f64, f64, f64), RuntimeError> {
        if arguments.len() != 1 {
            return Err(RuntimeError::new(
                RuntimeErrorKind::IncorrectArity { expected: 1..=1, actual: arguments.len() },
                span,
            ));
        }

        let vector = arguments[0].clone().into_vector(span.clone())?;
        if vector.len() != 2 && vector.len() != 3 {
            return Err(RuntimeError::new(
                RuntimeErrorKind::IncorrectVectorLength { expected: 2..=3, actual: vector.len() },
                span,
            ));
        }

        let x = vector[0].as_number(span.clone())?;
        let y = vector[1].as_number(span.clone())?;

        let z =
            if vector.len() == 3 {
                vector[2].as_number(span.clone())?
            } else {
                0.0
            };

        Ok((x, y, z))
    }

    fn union_manifolds(&mut self, mut children: Vec<ManifoldTableIndex>, span: InputSourceSpan) -> Result<ManifoldTableIndex, RuntimeError> {
        if children.len() == 1 {
            return Ok(children.remove(0))
        }

        let (all_manifolds, all_dispositions): (Vec<_>, Vec<_>) = children.into_iter()
            .map(|child| self.manifold_table.remove(child))
            .unzip();

        let disposition = ManifoldDisposition::flatten(&all_dispositions, span)?;
        
        let mut result = Manifold::new();
        for manifold in all_manifolds {
            result = result.union(&manifold);
        }
        Ok(self.manifold_table.add(result, disposition))
    }

    /// Given a list of objects, filter it down to only the manifolds, and return them.
    fn filter_objects_to_manifolds(objects: Vec<Object>) -> Vec<ManifoldTableIndex> {
        objects.into_iter()
            .filter_map(|child|
                if let Object::Manifold(index) = child {
                    Some(index)
                } else {
                    None
                }
            )
            .collect()
    }
}

/// Describes the manifold which will be referenced by `it`.
#[derive(Clone, Copy, Debug)]
pub enum ItManifold<'a> {
    /// `it` is valid and references a manifold.
    Some(&'a ManifoldTableIndex),

    /// `it` would usually be valid here, but it is unsupported because there is not one child.
    UnsupportedNotOneChild,

    /// `it` is not valid here.
    None,
}
