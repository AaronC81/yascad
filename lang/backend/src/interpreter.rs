use std::{cell::RefCell, collections::HashMap, rc::Rc};

use manifold_rs::Manifold;
use yascad_frontend::{BinaryOperator, InputSourceSpan, Node, NodeKind, Parameters};

use crate::{RuntimeError, RuntimeErrorKind, builtin::{self, ModuleDefinition, OperatorDefinition}, geometry_table::{GeometryDisposition, GeometryTable, GeometryTableEntry, GeometryTableIndex}, lexical_scope::LexicalScope, object::Object};

/// The context of whatever node is currently executing, to encapsulate surrounding state.
#[derive(Clone, Debug)]
pub struct ExecutionContext<'c> {
    /// The manifold (if any) which `it` currently refers to.
    it_manifold: ItManifold<'c>,

    /// If executing an operator body, its `children`.
    operator_children: Option<&'c [GeometryTableIndex]>,

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

    pub fn with_operator_children<'r>(&self, operator_children: Option<&'r [GeometryTableIndex]>) -> ExecutionContext<'r>
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

impl Default for ExecutionContext<'_> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Interpreter {
    pub(crate) manifold_table: GeometryTable,
    pub(crate) circle_segments: i32,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            manifold_table: GeometryTable::new(),

            // TODO: add $fn setter support
            circle_segments: 20,
        }
    }

    pub fn build_top_level_manifold(&self) -> Manifold {
        // Height which 2D geometry is extruded to, for 3D display
        const CROSS_SECTION_EXTRUDE_HEIGHT: f64 = 0.01;

        let mut result = Manifold::new();

        for (entry, disposition) in self.manifold_table.iter_geometry() {
            if *disposition != GeometryDisposition::Physical {
                continue;
            }

            match entry {
                GeometryTableEntry::Manifold(manifold) => {
                    result = result.union(manifold);
                },
                GeometryTableEntry::CrossSection(cross_section) => {
                    result = result.union(&Manifold::extrude(cross_section.polygons(), CROSS_SECTION_EXTRUDE_HEIGHT));
                }
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
                match self.get_existing_name(id, ctx, node.span.clone())? {
                    NameDefinition::Argument(obj) | NameDefinition::Binding(obj) => Ok(obj),
                    
                    def => Err(RuntimeError::new(
                        RuntimeErrorKind::InvalidIdentifier {
                            id: id.to_owned(),
                            kind: def.describe_kind(),
                        },
                        node.span.clone(),
                    )),
                }
            },

            NodeKind::NumberLiteral(num) => {
                Ok(Object::Number(*num))
            },

            NodeKind::BooleanLiteral(bool) => {
                Ok(Object::Boolean(*bool))
            }

            NodeKind::VectorLiteral(items) => {
                Ok(Object::Vector(
                    items.iter()
                        .map(|node| self.interpret(node, ctx))
                        .collect::<Result<Vec<_>, _>>()?
                ))
            },

            NodeKind::VectorRangeLiteral { start, end } => {
                let start = self.interpret(start, ctx)?.as_number(node.span.clone())?;
                let end = self.interpret(end, ctx)?.as_number(node.span.clone())?;

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

                // Not `physical_manifolds` because applying an operator to a virtual manifold is
                // allowed
                let manifold_children = self.filter_objects_to_manifolds(all_children);

                let it_manifold =
                    if manifold_children.len() == 1 {
                        ItManifold::Some(manifold_children.first().unwrap())
                    } else {
                        ItManifold::UnsupportedNotOneChild
                    };

                if !arguments.named.is_empty() {
                    todo!("named arguments are not yet supported"); // TODO
                }
                let arguments = &arguments.positional;

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
                match self.get_existing_name(name, ctx, node.span.clone())? {
                    NameDefinition::UserDefinedOperator(user_operator) => {
                        let NodeKind::OperatorDefinition { body, parameters, name: _ } = &user_operator.kind.clone()
                        else { unreachable!() };

                        let arguments = Self::match_arguments_to_parameters(arguments, parameters, node.span.clone())?;

                        let temporary_virtual_manifolds = manifold_children.into_iter()
                            .map(|index| {
                                let (m, _) = self.manifold_table.remove(index);
                                self.manifold_table.add(m, GeometryDisposition::Virtual)
                            })
                            .collect::<Vec<_>>();

                        let (geom, disp) = self.interpret_scoped_definition_body_into_geometry(
                            body, ctx, Some(&temporary_virtual_manifolds), arguments, node.span.clone()
                        )?;

                        for index in temporary_virtual_manifolds {
                            self.manifold_table.remove(index);
                        }

                        Ok(self.manifold_table.add_into_object(geom, disp))
                    }

                    NameDefinition::BuiltinOperator(op) => {
                        let (geom, disp) = op(self, arguments, manifold_children, node.span.clone())?;
                        Ok(self.manifold_table.add_into_object(geom, disp))
                    }

                    def => Err(RuntimeError::new(
                        RuntimeErrorKind::InvalidIdentifier {
                            id: name.to_owned(),
                            kind: def.describe_kind(),
                        },
                        node.span.clone(),
                    )),
                }
            }

            NodeKind::Call { name, arguments } => {
                if !arguments.named.is_empty() {
                    todo!("named arguments are not yet supported"); // TODO
                }
                let arguments = &arguments.positional;

                let arguments = arguments.iter()
                    .map(|arg| self.interpret(arg, ctx))
                    .collect::<Result<Vec<_>, _>>()?;

                match self.get_existing_name(name, ctx, node.span.clone())? {
                    NameDefinition::BuiltinModule(module) =>
                        module(self, arguments, ctx.operator_children, node.span.clone()),

                    NameDefinition::UserDefinedModule(user_module) => {
                        let NodeKind::ModuleDefinition { body, parameters, name: _ } = &user_module.kind.clone()
                        else { unreachable!() };

                        let arguments = Self::match_arguments_to_parameters(arguments, parameters, node.span.clone())?;
                        let (geom, disp) = self.interpret_scoped_definition_body_into_geometry(
                            body, ctx, None, arguments, node.span.clone()
                        )?;

                        Ok(self.manifold_table.add_into_object(geom, disp))
                    }

                    def => Err(RuntimeError::new(
                        RuntimeErrorKind::InvalidIdentifier {
                            id: name.to_owned(),
                            kind: def.describe_kind(),
                        },
                        node.span.clone(),
                    )),
                }
            },
            
            NodeKind::Binding { name, value } => {
                let value = self.interpret(value, ctx)?;

                self.add_name(name, NameDefinition::Binding(value.clone()), &ctx, node.span.clone())?;
                Ok(value)
            },

            NodeKind::FieldAccess { value, field } => {
                let value = self.interpret(value, ctx)?;

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
                let left = self.interpret(left, ctx)?;
                let right = self.interpret(right, ctx)?;

                let numeric_binop = |operation: &'static dyn Fn(f64, f64) -> f64| {
                    Ok::<Object, RuntimeError>(Object::Number(operation(
                        left.as_number(node.span.clone())?,
                        right.as_number(node.span.clone())?,
                    )))
                };
                let numeric_comparison_binop = |operation: &'static dyn Fn(f64, f64) -> bool| {
                    Ok::<Object, RuntimeError>(Object::Boolean(operation(
                        left.as_number(node.span.clone())?,
                        right.as_number(node.span.clone())?,
                    )))
                };

                match op {
                    BinaryOperator::Add => numeric_binop(&|l, r| l + r),
                    BinaryOperator::Subtract => numeric_binop(&|l, r| l - r),
                    BinaryOperator::Multiply => numeric_binop(&|l, r| l * r),
                    BinaryOperator::Divide => numeric_binop(&|l, r| l / r),

                    BinaryOperator::Equals => Ok(Object::Boolean(left == right)),

                    BinaryOperator::LessThan => numeric_comparison_binop(&|l, r| l < r),
                    BinaryOperator::LessThanOrEquals => numeric_comparison_binop(&|l, r| l <= r),
                    BinaryOperator::GreaterThan => numeric_comparison_binop(&|l, r| l > r),
                    BinaryOperator::GreaterThanOrEquals => numeric_comparison_binop(&|l, r| l >= r),
                }
            },

            NodeKind::UnaryNegate(value) => {
                let value = self.interpret(value, ctx)?.as_number(node.span.clone())?;
                Ok(Object::Number(-value))
            },

            NodeKind::OperatorDefinition { name, .. } => {
                self.add_name(name, NameDefinition::UserDefinedOperator(node.clone()), &ctx, node.span.clone())?;
                Ok(Object::Null)
            },

            NodeKind::ModuleDefinition { name, .. } => {
                self.add_name(name, NameDefinition::UserDefinedModule(node.clone()), &ctx, node.span.clone())?;
                Ok(Object::Null)
            },

            NodeKind::ForLoop { loop_variable, loop_source, body } => {
                let loop_source = self.interpret(loop_source, ctx)?.into_vector(node.span.clone())?;

                let mut result_indices = vec![];
                for item in loop_source {
                    let ctx = ctx.with_deeper_scope();
                    self.add_name(&loop_variable, NameDefinition::Binding(item), &ctx, node.span.clone())?;
                    let (geom, disp) = self.interpret_body_into_geometry(&body, &ctx, node.span.clone())?;
                    
                    result_indices.push(self.manifold_table.add(geom, disp));
                }

                let (geom, disp) = self.manifold_table.remove_many_into_union(result_indices, node.span.clone())?;
                Ok(self.manifold_table.add_into_object(geom, disp))
            },

            NodeKind::IfConditional { condition, true_body, false_body } => {
                let condition = self.interpret(condition, ctx)?.as_boolean(node.span.clone())?;

                let ctx = ctx.with_deeper_scope();
                if condition {
                    let (geom, disp) = self.interpret_body_into_geometry(&true_body, &ctx, node.span.clone())?;
                    Ok(self.manifold_table.add_into_object(geom, disp))
                } else if let Some(false_body) = false_body {
                    let (geom, disp) = self.interpret_body_into_geometry(&false_body, &ctx, node.span.clone())?;
                    Ok(self.manifold_table.add_into_object(geom, disp))
                } else {
                    Ok(Object::Null)
                }
            },
        }
    }

    /// Execute a list of nodes.
    fn interpret_body(&mut self, nodes: &[Node], ctx: &ExecutionContext) -> Result<Vec<Object>, RuntimeError> {
        nodes.iter()
            .map(|node| self.interpret(node, ctx))
            .collect()
    }

    /// Execute a list of nodes and collect any geometry that they generate into a single union'ed
    /// geometry. This is how control-flow operations behave.
    /// 
    /// It is the caller's responsibility to create a new deeper scope if necessary, because the
    /// caller may wish to inject variables into it (e.g. the `for` loop counter).
    fn interpret_body_into_geometry(
        &mut self,
        nodes: &[Node],
        ctx: &ExecutionContext,
        span: InputSourceSpan,
    ) -> Result<(GeometryTableEntry, GeometryDisposition), RuntimeError> {
        let result_objects = self.interpret_body(nodes, &ctx)?;
        let result_manifolds = self.filter_objects_to_physical_geometries(result_objects);
        self.manifold_table.remove_many_into_union(result_manifolds, span)
    }

    /// Execute a list of nodes in a new scope, with a given set of arguments and children, and
    /// collect any geometry that they generate into a single union'ed geometry. This is how modules
    /// and operators behave.
    fn interpret_scoped_definition_body_into_geometry(
        &mut self,
        nodes: &[Node],
        ctx: &ExecutionContext,
        operator_children: Option<&[GeometryTableIndex]>,
        arguments: HashMap<String, Object>,
        span: InputSourceSpan,
    ) -> Result<(GeometryTableEntry, GeometryDisposition), RuntimeError> {
        self.interpret_body_into_geometry(
            nodes,
            &ctx
                .with_it_manifold(ItManifold::None)
                .with_operator_children(operator_children)
                .with_deeper_scope()
                .with_arguments(arguments),
            span,
        )
    }

    /// Given a list of objects, filter it down to only manifolds, and return them.
    fn filter_objects_to_manifolds(&self, objects: Vec<Object>) -> Vec<GeometryTableIndex> {
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

    /// Given a list of objects, filter it down to only the *physical* geometries, and return them.
    fn filter_objects_to_physical_geometries(&self, objects: Vec<Object>) -> Vec<GeometryTableIndex> {
        self.filter_objects_to_manifolds(objects)
            .into_iter()
            .filter(|index| self.manifold_table.get_disposition(index) == GeometryDisposition::Physical)
            .collect()
    }

    /// Look up a name.
    fn get_name(&self, name: &str, ctx: &ExecutionContext) -> Option<NameDefinition> {
        if let Some(object) = ctx.lexical_scope.borrow().get_binding(name) {
            return Some(NameDefinition::Binding(object))
        }

        if let Some(object) = ctx.arguments.get(name) {
            return Some(NameDefinition::Argument(object.clone()));
        }

        if let Some(module) = builtin::get_builtin_module(name) {
            return Some(NameDefinition::BuiltinModule(module))
        }

        if let Some(module) = ctx.lexical_scope.borrow().get_module(name) {
            return Some(NameDefinition::UserDefinedModule(module))
        }

        if let Some(operator) = builtin::get_builtin_operator(name) {
            return Some(NameDefinition::BuiltinOperator(operator))
        }

        if let Some(operator) = ctx.lexical_scope.borrow().get_operator(name) {
            return Some(NameDefinition::UserDefinedOperator(operator))
        }

        None
    }

    /// Like [`Self::get_name`] but returns a [`RuntimeErrorKind::UndefinedIdentifier`] if the name
    /// is not defined.
    fn get_existing_name(&self, name: &str, ctx: &ExecutionContext, span: InputSourceSpan) -> Result<NameDefinition, RuntimeError> {
        self.get_name(name, ctx).ok_or_else(||
            RuntimeError::new(RuntimeErrorKind::UndefinedIdentifier(name.to_owned()), span))
    }

    /// Define a new name.
    /// 
    /// Returns an error if the name is already defined.
    fn add_name(&self, name: &str, def: NameDefinition, ctx: &ExecutionContext, span: InputSourceSpan) -> Result<(), RuntimeError> {
        if self.get_name(name, ctx).is_some() {
            return Err(RuntimeError::new(RuntimeErrorKind::DuplicateName(name.to_owned()), span))
        }

        match def {
            NameDefinition::Binding(object) => {
                ctx.lexical_scope.borrow_mut().add_binding(name.to_owned(), object);
            }
            NameDefinition::UserDefinedOperator(node) => {
                ctx.lexical_scope.borrow_mut().add_operator(name.to_owned(), node);
            }
            NameDefinition::UserDefinedModule(node) => {
                ctx.lexical_scope.borrow_mut().add_module(name.to_owned(), node);
            }

            NameDefinition::Argument(_)
            | NameDefinition::BuiltinModule(_)
            | NameDefinition::BuiltinOperator(_) => panic!("cannot add new definition of this type"),
        }

        Ok(())
    }

    /// Given a list of arguments, and a set of expected parameter names, match the arguments and
    /// parameters.
    fn match_arguments_to_parameters(arguments: Vec<Object>, parameters: &Parameters, span: InputSourceSpan) -> Result<HashMap<String, Object>, RuntimeError> {
        if !parameters.optional.is_empty() {
            todo!("Optional parameters are not yet supported"); // TODO
        }
        let parameters = &parameters.required;

        // Validate number of arguments so forthcoming `zip` is definitely balanced
        if arguments.len() != parameters.len() {
            return Err(RuntimeError::new(
                RuntimeErrorKind::IncorrectArity {
                    expected: parameters.len()..=parameters.len(),
                    actual: arguments.len(),
                },
                span,
            ));
        }

        // Convert to hash
        Ok(arguments.into_iter()
            .zip(parameters)
            .map(|(arg, param)| (param.to_owned(), arg))
            .collect::<HashMap<_, _>>())
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

/// Describes the manifold which will be referenced by `it`.
#[derive(Clone, Copy, Debug)]
pub enum ItManifold<'a> {
    /// `it` is valid and references a manifold.
    Some(&'a GeometryTableIndex),

    /// `it` would usually be valid here, but it is unsupported because there is not one child.
    UnsupportedNotOneChild,

    /// `it` is not valid here.
    None,
}

/// Describes the name sourced from somewhere in the interpreter.
#[derive(Clone)]
pub enum NameDefinition {
    Binding(Object),
    Argument(Object),

    BuiltinModule(ModuleDefinition),
    UserDefinedModule(Node),

    BuiltinOperator(OperatorDefinition),
    UserDefinedOperator(Node),
}

impl NameDefinition {
    pub fn describe_kind(&self) -> String {
        match self {
            NameDefinition::Binding(_) => "binding",
            NameDefinition::Argument(_) => "parameter",
            NameDefinition::BuiltinModule(_) => "built-in module",
            NameDefinition::UserDefinedModule(_) => "user-defined module",
            NameDefinition::BuiltinOperator(_) => "built-in operator",
            NameDefinition::UserDefinedOperator(_) => "user-defined operator",
        }.to_string()
    }
}
