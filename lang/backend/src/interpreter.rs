use std::{cell::RefCell, collections::{HashMap, HashSet}, iter::zip, ops::RangeInclusive, rc::Rc};

use manifold_csg::Manifold;
use yascad_frontend::{Arguments, BinaryOperator, InputSourceSpan, Node, NodeKind, Parameters, VectorLiteralItem, VectorLiteralItemKind};

use crate::{RuntimeError, RuntimeErrorKind, builtin::{self, FunctionDefinition, ModuleDefinition, OperatorDefinition, children_definition, get_children}, geometry_table::{GeometryDisposition, GeometryTable, GeometryTableEntry, GeometryTableIndex}, lexical_scope::{LexicalScope, UserDefinition}, object::Object};

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
        self.with_deeper_scope_than(self.lexical_scope.clone())
    }

    pub fn with_deeper_scope_than(&'_ self, parent: Rc<RefCell<LexicalScope>>) -> ExecutionContext<'_> {
        ExecutionContext {
            lexical_scope: Rc::new(RefCell::new(LexicalScope::new(parent))),
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
    pub(crate) debug_hook: Box<dyn Fn(&Object) + 'static>,

    pub outputs: HashMap<String, GeometryTableEntry>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            manifold_table: GeometryTable::new(),

            // TODO: add $fn setter support
            circle_segments: 20,

            debug_hook: Box::new(|obj| println!("{obj:?}")),

            outputs: HashMap::new(),
        }
    }

    /// Set the function which gets called when invoking the `__debug` module.
    pub fn set_debug_hook(&mut self, func: Box<dyn Fn(&Object) + 'static>) {
        self.debug_hook = func;
    }

    pub fn build_top_level_manifold(&self) -> Manifold {
        let mut result = Manifold::empty();

        for (entry, disposition) in self.manifold_table.iter_geometry() {
            if *disposition != GeometryDisposition::Physical {
                continue;
            }

            result = result.union(&entry.as_manifold_for_display());
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
                if id == "$children" {
                    return match ctx.operator_children {
                        Some(children) => Ok(Object::Number(children.len() as f64)),
                        None => Err(RuntimeError::new(
                            RuntimeErrorKind::ChildrenInvalid,
                            node.span.clone(),
                        )),
                    }
                }

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

            NodeKind::NullLiteral => {
                Ok(Object::Null)
            },

            NodeKind::NumberLiteral(num) => {
                Ok(Object::Number(*num))
            },

            NodeKind::StringLiteral(str) => {
                Ok(Object::String(str.clone()))
            }

            NodeKind::BooleanLiteral(bool) => {
                Ok(Object::Boolean(*bool))
            }

            NodeKind::VectorLiteral(items) => {
                Ok(Object::Vector(
                    items.iter()
                        .map(|node| self.interpret_vector_item(node, ctx))
                        .collect::<Result<Vec<_>, _>>()?
                        .into_iter()
                        .flatten()
                        .collect()
                ))
            },

            NodeKind::VectorRangeLiteral { start, end, step } => {
                let start = self.interpret(start, ctx)?.as_number(node.span.clone())?;
                let end = self.interpret(end, ctx)?.as_number(node.span.clone())?;

                let step = match step {
                    Some(step) => self.interpret(step, ctx)?.as_number(node.span.clone())?,
                    None => 1.0,
                };

                if end < start {
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::FlippedRange,
                        node.span.clone(),
                    ));
                }

                let mut current = start;
                let mut items = vec![];
                while current <= end {
                    items.push(Object::Number(current));
                    current += step;
                }

                Ok(Object::Vector(items))
            }

            NodeKind::ItReference => {
                match ctx.it_manifold {
                    ItManifold::Some(manifold_table_index) => {
                        Ok(self.manifold_table.index_to_object(manifold_table_index))
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

            NodeKind::OperatorApplication { name, arguments, children, splatted_children } => {
                let all_children = match (splatted_children, children) {
                    (None, children) => {
                        children.iter()
                            .map(|child| self.interpret(child, &ctx.with_it_manifold(ItManifold::None)))
                            .collect::<Result<Vec<_>, _>>()?
                    },

                    (Some(splat_children), children) if children.is_empty() => {
                        // TODO: horrible special-case, but I'm not sure there's a way to make this work reliably for anything other than `children`
                        // The behaviour is pretty confusing otherwise!
                        match splat_children {
                            deref!(Node { kind: NodeKind::Call { name, arguments }, .. }) if name == "children" => {
                                let arguments = self.evaluate_arguments(arguments, &ctx)?;
                                let arguments = self.match_arguments_to_parameters(
                                    arguments,
                                    children_definition().parameters,
                                    node.span.clone(),
                                )?;
                                
                                get_children(self, arguments, ctx.operator_children.clone(), node.span.clone())?
                                    .into_iter()
                                    .map(|idx| self.manifold_table.index_to_object(&idx))
                                    .collect::<Vec<_>>()
                            },

                            _ => {
                                return Err(RuntimeError::new(
                                    RuntimeErrorKind::UnsupportedSplat,
                                    node.span.clone(),
                                ))
                            }
                        }
                    },

                    (Some(_), _) => {
                        unreachable!("got splat and list of children"); // The syntax shouldn't allow for this
                    },
                };

                // Not `physical_manifolds` because applying an operator to a virtual manifold is
                // allowed
                let manifold_children = self.filter_objects_to_geometry(all_children);

                let it_manifold =
                    if manifold_children.len() == 1 {
                        ItManifold::Some(manifold_children.first().unwrap())
                    } else {
                        ItManifold::UnsupportedNotOneChild
                    };

                let arguments = self.evaluate_arguments(arguments, &ctx.with_it_manifold(it_manifold))?;

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
                    NameDefinition::UserDefinedOperator(UserDefinition { parameters, body, scope }) => {
                        let arguments = self.match_arguments_to_parameters(arguments, parameters, node.span.clone())?;

                        let temporary_virtual_manifolds = manifold_children.into_iter()
                            .map(|index| {
                                let (m, _) = self.manifold_table.remove(index);
                                self.manifold_table.add(m, GeometryDisposition::Virtual)
                            })
                            .collect::<Vec<_>>();

                        let (geom, disp) = self.interpret_scoped_definition_body_into_geometry(
                            &body, ctx, scope, Some(&temporary_virtual_manifolds), arguments, node.span.clone()
                        )?;

                        for index in temporary_virtual_manifolds {
                            self.manifold_table.remove(index);
                        }

                        Ok(self.manifold_table.add_into_object(geom, disp))
                    }

                    NameDefinition::BuiltinOperator(op) => {
                        let arguments = self.match_arguments_to_parameters(arguments, op.parameters, node.span.clone())?;
                        let (geom, disp) = (op.action)(self, arguments, manifold_children, node.span.clone())?;
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
                let arguments = self.evaluate_arguments(arguments, &ctx)?;

                match self.get_existing_name(name, ctx, node.span.clone())? {
                    NameDefinition::BuiltinModule(module) => {
                        let arguments = self.match_arguments_to_parameters(arguments, module.parameters, node.span.clone())?;
                        (module.action)(self, arguments, ctx.operator_children, node.span.clone())
                    }

                    NameDefinition::UserDefinedModule(UserDefinition { parameters, body, scope }) => {
                        let arguments = self.match_arguments_to_parameters(arguments, parameters, node.span.clone())?;
                        let (geom, disp) = self.interpret_scoped_definition_body_into_geometry(
                            &body, ctx, scope, None, arguments, node.span.clone()
                        )?;

                        Ok(self.manifold_table.add_into_object(geom, disp))
                    }

                    NameDefinition::BuiltinFunction(module) => {
                        let arguments = self.match_arguments_to_parameters(arguments, module.parameters, node.span.clone())?;
                        (module.action)(self, arguments, node.span.clone())
                    }

                    NameDefinition::UserDefinedFunction(UserDefinition { parameters, body, scope }) => {
                        // Enforced by parsing
                        if body.len() != 1 {
                            unreachable!("functions should only have one node")
                        }
                        let body = body.first().unwrap();

                        let arguments = self.match_arguments_to_parameters(arguments, parameters, node.span.clone())?;
                        self.interpret(body, &ctx.with_deeper_scope_than(scope).with_arguments(arguments))
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

            NodeKind::IndexAccess { value, index } => {
                let value = self.interpret(value, ctx)?;
                let index = self.interpret(index, ctx)?.as_index(index.span.clone())?;
                
                match value {
                    Object::String(str) => Ok(
                        str
                            .chars().nth(index)
                            .map(|c| Object::String(c.to_string()))
                            .unwrap_or(Object::Null)
                    ),
                    Object::Vector(vec) => Ok(
                        vec
                            .get(index).cloned()
                            .unwrap_or(Object::Null)
                    ),

                    Object::Null
                    | Object::Number(_)
                    | Object::Boolean(_)
                    | Object::Manifold(_)
                    | Object::CrossSection(_)
                    | Object::EmptyGeometry => {
                        Err(RuntimeError::new(
                            RuntimeErrorKind::IncorrectType {
                                expected: "string or vector".to_owned(),
                                actual: value.describe_type(),
                            },
                            node.span.clone()
                        ))
                    }
                }
            }

            NodeKind::BinaryOperation { left, right, op } => {
                let left = self.interpret(left, ctx)?;
                let right = self.interpret(right, ctx)?;

                let numeric_comparison_binop = |operation: &'static dyn Fn(f64, f64) -> bool| {
                    Ok::<Object, RuntimeError>(Object::Boolean(operation(
                        left.as_number(node.span.clone())?,
                        right.as_number(node.span.clone())?,
                    )))
                };

                match op {
                    BinaryOperator::Add => self.apply_numeric_binop(left, right, &|l, r| l + r, &node.span),
                    BinaryOperator::Subtract => self.apply_numeric_binop(left, right, &|l, r| l - r, &node.span),
                    BinaryOperator::Multiply => self.apply_numeric_binop(left, right, &|l, r| l * r, &node.span),
                    BinaryOperator::Divide => self.apply_numeric_binop(left, right, &|l, r| l / r, &node.span),
                    BinaryOperator::Modulo => self.apply_numeric_binop(left, right, &|l, r| l % r, &node.span),
                    BinaryOperator::Power => self.apply_numeric_binop(left, right, &|l, r| l.powf(r), &node.span),

                    BinaryOperator::Equals => Ok(Object::Boolean(left == right)),
                    BinaryOperator::NotEquals => Ok(Object::Boolean(left != right)),

                    BinaryOperator::LessThan => numeric_comparison_binop(&|l, r| l < r),
                    BinaryOperator::LessThanOrEquals => numeric_comparison_binop(&|l, r| l <= r),
                    BinaryOperator::GreaterThan => numeric_comparison_binop(&|l, r| l > r),
                    BinaryOperator::GreaterThanOrEquals => numeric_comparison_binop(&|l, r| l >= r),

                    BinaryOperator::BooleanAnd => Ok(Object::Boolean(
                        left.as_boolean(node.span.clone())? && right.as_boolean(node.span.clone())?,
                    )),
                    BinaryOperator::BooleanOr => Ok(Object::Boolean(
                        left.as_boolean(node.span.clone())? || right.as_boolean(node.span.clone())?,
                    )),
                }
            },

            NodeKind::UnaryNegate(value) => {
                let value = self.interpret(value, ctx)?.as_number(node.span.clone())?;
                Ok(Object::Number(-value))
            },

            NodeKind::UnaryNot(value) => {
                let value = self.interpret(value, ctx)?.as_boolean(node.span.clone())?;
                Ok(Object::Boolean(!value))
            },

            NodeKind::OperatorDefinition { name, parameters, body } => {
                let parameters = self.interpret_parameters(parameters, ctx)?;
                self.add_name(
                    name,
                    NameDefinition::UserDefinedOperator(UserDefinition { parameters, body: body.clone(), scope: ctx.lexical_scope.clone() }),
                    &ctx, node.span.clone()
                )?;
                Ok(Object::Null)
            },

            NodeKind::ModuleDefinition { name, parameters, body } => {
                let parameters = self.interpret_parameters(parameters, ctx)?;
                self.add_name(
                    name,
                    NameDefinition::UserDefinedModule(UserDefinition { parameters, body: body.clone(), scope: ctx.lexical_scope.clone() }),
                    &ctx, node.span.clone()
                )?;
                Ok(Object::Null)
            },

            NodeKind::FunctionDefinition { name, parameters, body } => {
                let parameters = self.interpret_parameters(parameters, ctx)?;
                self.add_name(
                    name,
                    NameDefinition::UserDefinedFunction(UserDefinition { parameters, body: vec![*body.clone()], scope: ctx.lexical_scope.clone() }),
                    &ctx, node.span.clone()
                )?;
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

            NodeKind::TernaryConditional { condition, true_case, false_case } => {
                let condition = self.interpret(condition, ctx)?.as_boolean(node.span.clone())?;

                let true_value = self.interpret(true_case, &ctx)?;
                let false_value = self.interpret(false_case, &ctx)?;

                if condition {
                    Ok(true_value)
                } else {
                    Ok(false_value)
                }
            }
        }
    }

    /// Execute a vector literal item.
    /// Returns the items which should be added into the vector (flattened) for this literal item
    /// or comprehension.
    pub fn interpret_vector_item(&mut self, node: &VectorLiteralItem, ctx: &ExecutionContext) -> Result<Vec<Object>, RuntimeError> {
        match &node.kind {
            VectorLiteralItemKind::Value(node) => {
                let item = self.interpret(&node, ctx)?;
                Ok(vec![item])
            },

            VectorLiteralItemKind::IfComprehension { condition, body } => {
                let condition = self.interpret(&condition, ctx)?.as_boolean(node.span.clone())?;

                if condition {
                    self.interpret_vector_item(body, ctx)
                } else {
                    Ok(vec![])
                }
            },

            VectorLiteralItemKind::ForComprehension { loop_variable, loop_source, body } => {
                let loop_source = self.interpret(&loop_source, ctx)?.into_vector(node.span.clone())?;

                let mut results = vec![];
                for item in loop_source {
                    let ctx = ctx.with_deeper_scope();
                    self.add_name(&loop_variable, NameDefinition::Binding(item), &ctx, node.span.clone())?;

                    let this_item_results = self.interpret_vector_item(body, &ctx)?;
                    results.extend(this_item_results.into_iter());
                }

                Ok(results)
            },

            VectorLiteralItemKind::EachComprehension { body } => {
                let body = self.interpret_vector_item(&body, ctx)?;
                let flattened_items = body.into_iter()
                    .map(|item| {
                        item.as_vector(node.span.clone()).map(|slice| slice.to_vec())
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                Ok(flattened_items)
            }
        }
    }

    /// Execute a list of nodes.
    fn interpret_body(&mut self, nodes: &[Node], ctx: &ExecutionContext) -> Result<Vec<Object>, RuntimeError> {
        nodes.iter()
            .map(|node| self.interpret(node, ctx))
            .collect()
    }

    /// Evaluate any default parameter values.
    fn interpret_parameters(&mut self, parameters: &Parameters, ctx: &ExecutionContext) -> Result<EvaluatedParameters, RuntimeError> {
        Ok(EvaluatedParameters {
            required: parameters.required.clone(),
            optional: parameters.optional.iter()
                .map(|(name, node)| 
                    self.interpret(node, ctx).map(|obj| (name.to_owned(), obj))
                )
                .collect::<Result<_, _>>()?,
            optional_named_only: vec![],
        })
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
        scope: Rc<RefCell<LexicalScope>>,
        operator_children: Option<&[GeometryTableIndex]>,
        arguments: HashMap<String, Object>,
        span: InputSourceSpan,
    ) -> Result<(GeometryTableEntry, GeometryDisposition), RuntimeError> {
        self.interpret_body_into_geometry(
            nodes,
            &ctx
                .with_it_manifold(ItManifold::None)
                .with_operator_children(operator_children)
                .with_deeper_scope_than(scope)
                .with_arguments(arguments),
            span,
        )
    }

    /// Given a list of objects, filter it down to only manifolds, and return them.
    fn filter_objects_to_geometry(&self, objects: Vec<Object>) -> Vec<GeometryTableIndex> {
        objects.into_iter()
            .filter_map(|child|
                if let Object::Manifold(index) | Object::CrossSection(index) = child {
                    Some(index)
                } else {
                    None
                }
            )
            .collect()
    }

    /// Given a list of objects, filter it down to only the *physical* geometries, and return them.
    fn filter_objects_to_physical_geometries(&self, objects: Vec<Object>) -> Vec<GeometryTableIndex> {
        self.filter_objects_to_geometry(objects)
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

        if let Some(def) = ctx.lexical_scope.borrow().get_module(name) {
            return Some(NameDefinition::UserDefinedModule(def))
        }

        if let Some(operator) = builtin::get_builtin_operator(name) {
            return Some(NameDefinition::BuiltinOperator(operator))
        }

        if let Some(def) = ctx.lexical_scope.borrow().get_operator(name) {
            return Some(NameDefinition::UserDefinedOperator(def))
        }

        if let Some(function) = builtin::get_builtin_function(name) {
            return Some(NameDefinition::BuiltinFunction(function))
        }

        if let Some(def) = ctx.lexical_scope.borrow().get_function(name) {
            return Some(NameDefinition::UserDefinedFunction(def))
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
            NameDefinition::UserDefinedOperator(def) => {
                ctx.lexical_scope.borrow_mut().add_operator(name.to_owned(), def);
            }
            NameDefinition::UserDefinedModule(def) => {
                ctx.lexical_scope.borrow_mut().add_module(name.to_owned(), def);
            }
            NameDefinition::UserDefinedFunction(def) => {
                ctx.lexical_scope.borrow_mut().add_function(name.to_owned(), def);
            }

            NameDefinition::Argument(_)
            | NameDefinition::BuiltinModule(_)
            | NameDefinition::BuiltinOperator(_)
            | NameDefinition::BuiltinFunction(_) => panic!("cannot add new definition of this type"),
        }

        Ok(())
    }

    /// Evaluate [`Arguments`]  into [`EvaluatedArguments`] using the interpreter.
    pub fn evaluate_arguments(&mut self, arguments: &Arguments, ctx: &ExecutionContext) -> Result<EvaluatedArguments, RuntimeError> {
        Ok(EvaluatedArguments {
            positional: arguments.positional.iter()
                .map(|arg| self.interpret(arg, ctx))
                .collect::<Result<Vec<_>, _>>()?,
            named: arguments.named.iter()
                .map(|(name, arg)| self.interpret(arg, ctx).map(|obj| (name.clone(), obj)))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    /// Given a list of arguments and parameters, match the arguments to parameters, and return a
    /// set of parameter names matched to argument values (or defaults).
    fn match_arguments_to_parameters(&mut self, arguments: EvaluatedArguments, parameters: EvaluatedParameters, span: InputSourceSpan) -> Result<HashMap<String, Object>, RuntimeError> {
        // TODO: validate on definition that parameter names are unique

        // Check that named arguments are specified no more than once
        for (name, _) in &arguments.named {
            let count_args_with_name = arguments.named.iter()
                .filter(|(n, _)| n == name)
                .count();

            if count_args_with_name > 1 {
                return Err(RuntimeError::new(RuntimeErrorKind::DuplicateNamedArgument(name.to_owned()), span));
            }
        }

        // Validate that there aren't more positional arguments than we can possibly ever accept
        if arguments.positional.len() > parameters.max_len() {
            return Err(RuntimeError::new(
                RuntimeErrorKind::IncorrectArity {
                    expected: parameters.len_range(),
                    actual: arguments.positional.len(),
                },
                span,
            ));
        }

        // Map positional arguments to ascending parameters
        // (We enforce in the parser that required arguments come before optional arguments)
        enum Location { Positional, Named }
        let mut map = HashMap::new();
        for (arg, param) in zip(&arguments.positional, parameters.ordered_positional_names()) {
            map.insert(param, (arg.clone(), Location::Positional));
        }

        // Now map named arguments to parameters, validating the parameters weren't already assigned
        // and do actually exist
        for (name, arg) in &arguments.named {
            if !parameters.names().contains(name) {
                return Err(RuntimeError::new(RuntimeErrorKind::UndefinedNamedArgument(name.to_owned()), span))
            }

            if let Some((_, loc)) = map.get(name) {
                return match loc {
                    Location::Positional => Err(RuntimeError::new(RuntimeErrorKind::NamedArgumentRepeatsPositionalArgument(name.to_owned()), span)),
                    Location::Named => Err(RuntimeError::new(RuntimeErrorKind::DuplicateNamedArgument(name.to_owned()), span)),
                }
            }

            map.insert(name.clone(), (arg.clone(), Location::Named));
        }

        // Check that all required parameters were specified
        let missing_required_params = parameters.required.iter()
            .filter(|param| !map.contains_key(param.to_owned()))
            .cloned()
            .collect::<Vec<_>>();
        if !missing_required_params.is_empty() {
            return Err(RuntimeError::new(RuntimeErrorKind::MissingNamedArguments(missing_required_params), span))
        }

        // For any optional parameters where values weren't given, instantiate the default
        for (name, default) in parameters.all_optionals() {
            if !map.contains_key(name) {
                map.insert(name.to_owned(), (default.clone(), Location::Named));
            }
        }

        // Discard now-irrelevant location
        let map = map.into_iter()
            .map(|(k, (o, _))| (k, o))
            .collect();

        Ok(map)
    }

    fn apply_numeric_binop(&mut self, left: Object, right: Object, op: &dyn Fn(f64, f64) -> f64, span: &InputSourceSpan) -> Result<Object, RuntimeError> {
        match (left, right) {
            (Object::Number(left), Object::Number(right)) => Ok(Object::Number(op(left, right))),

            (Object::Vector(left), Object::Vector(right)) => {
                if left.len() != right.len() {
                    // TODO: support this
                    return Err(
                        RuntimeError::new(
                            RuntimeErrorKind::MixedVectorSizeBinopNotSupported,
                            span.clone(),
                        )
                    )
                }

                Ok(Object::Vector(
                    left.into_iter()
                        .zip(right)
                        .map(|(l, r)| self.apply_numeric_binop(l, r, op, span))
                        .collect::<Result<Vec<_>, _>>()?,
                ))
            }

            (Object::Vector(left), Object::Number(right)) => {
                let vec_of_nums = left.into_iter()
                    .map(|o| o.as_number(span.clone()))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(Object::Vector(
                    vec_of_nums.into_iter()
                        .map(|n| Object::Number(op(n, right)))
                        .collect()
                ))
            }

            (Object::Number(left), Object::Vector(right)) => {
                let vec_of_nums = right.into_iter()
                    .map(|o| o.as_number(span.clone()))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(Object::Vector(
                    vec_of_nums.into_iter()
                        .map(|n| Object::Number(op(left, n)))
                        .collect()
                ))
            }

            (left, right) => Err(
                RuntimeError::new(
                    RuntimeErrorKind::IncorrectBinopTypes { left: left.describe_type(), right: right.describe_type() },
                    span.clone(),
                )
            )
        }
    }

    pub(crate) fn add_output(&mut self, name: &str, geometry: GeometryTableEntry, span: InputSourceSpan) -> Result<(), RuntimeError> {
        if self.outputs.contains_key(name) {
            return Err(RuntimeError::new(
                RuntimeErrorKind::DuplicateOutputName(name.to_owned()),
                span,
            ));
        }

        self.outputs.insert(name.to_owned(), geometry);
        Ok(())
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
    UserDefinedModule(UserDefinition),

    BuiltinOperator(OperatorDefinition),
    UserDefinedOperator(UserDefinition),

    BuiltinFunction(FunctionDefinition),
    UserDefinedFunction(UserDefinition),
}

impl NameDefinition {
    pub fn describe_kind(&self) -> String {
        match self {
            NameDefinition::Binding(_) => "binding",
            NameDefinition::Argument(_) => "parameter",
            NameDefinition::BuiltinModule(_) => "built-in module",
            NameDefinition::UserDefinedModule { .. } => "user-defined module",
            NameDefinition::BuiltinOperator(_) => "built-in operator",
            NameDefinition::UserDefinedOperator { .. } => "user-defined operator",
            NameDefinition::BuiltinFunction(_) => "built-in function",
            NameDefinition::UserDefinedFunction { .. } => "user-defined function",
        }.to_string()
    }
}

/// A collection of evaluated arguments.
pub struct EvaluatedArguments {
    positional: Vec<Object>,
    named: Vec<(String, Object)>,
}

/// A collection of parameters with evaluated defaults.
#[derive(Clone, Debug)]
pub struct EvaluatedParameters {
    pub required: Vec<String>,
    pub optional: Vec<(String, Object)>,

    /// Optional arguments which can only be specified by name, not positionally.
    /// This is an internal language feature to support `r`/`d` parameters, and isn't usable from
    /// language source.
    pub optional_named_only: Vec<(String, Object)>,
}

impl EvaluatedParameters {
    pub fn new(required: Vec<String>, optional: Vec<(String, Object)>) -> Self {
        Self {
            required,
            optional,
        
            // This is so rarely used that we don't expect it in the constructor
            optional_named_only: vec![],
        }
    }

    pub fn empty() -> Self {
        Self::new(vec![], vec![])
    }

    pub fn required(required: Vec<String>) -> Self {
        Self::new(required, vec![])
    }

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

    /// All positional parameters, required and optional, in the order they'd be expected to be
    /// specified.
    pub fn ordered_positional_names(&self) -> impl Iterator<Item = String> {
        self.required.iter().cloned()
            .chain(self.optional.iter().map(|(name, _)| name.clone()))
    }

    /// All optional parameters, both positional and named-only.
    pub fn all_optionals(&self) -> impl Iterator<Item = &(String, Object)> {
        self.optional.iter()
            .chain(self.optional_named_only.iter())
    }

    /// All permissible named parameters.
    pub fn names(&self) -> HashSet<String> {
        self.required.iter()
            .chain(self.all_optionals().map(|(name, _)| name))
            .cloned()
            .collect()
    }
}
