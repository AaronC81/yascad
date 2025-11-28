use std::{collections::HashMap, rc::Rc};

use manifold_rs::Manifold;
use yascad_frontend::{InputSourceSpan, Node, NodeKind};

use crate::{RuntimeError, RuntimeErrorKind, lexical_scope::LexicalScope, manifold_table::{ManifoldDisposition, ManifoldTable, ManifoldTableIndex}, object::Object};

pub struct Interpreter {
    current_scope: LexicalScope,
    manifold_table: ManifoldTable,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            manifold_table: ManifoldTable::new(),
            current_scope: LexicalScope::new_root(),
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

    pub fn interpret_top_level(&mut self, node: &Node) -> Result<Object, RuntimeError> {
        self.interpret(node)
    }

    pub fn interpret(&mut self, node: &Node) -> Result<Object, RuntimeError> {
        match &node.kind {
            NodeKind::Identifier(id) => {
                self.current_scope.get_binding(id)
                    .ok_or_else(|| RuntimeError::new(
                        RuntimeErrorKind::UndefinedIdentifier(id.to_owned()),
                        node.span.clone(),
                    ))
                    .cloned()
            },

            NodeKind::NumberLiteral(num) => {
                Ok(Object::Number(*num))
            },

            NodeKind::VectorLiteral(items) => {
                Ok(Object::Vector(
                    items.iter()
                        .map(|node| self.interpret(node))
                        .collect::<Result<Vec<_>, _>>()?
                ))
            }

            NodeKind::ModifierApplication { name, arguments, children } => {
                // TODO: change later to render each child as a virtual manifold, which the modifier
                //       body can copy as needed
                let all_children = children.iter()
                    .map(|child| self.interpret(child))
                    .collect::<Result<Vec<_>, _>>()?;
                let manifold_children = all_children.into_iter()
                    .filter_map(|child| child.into_manifold(node.span.clone()).ok())
                    .collect::<Vec<_>>();

                let arguments = arguments.iter()
                    .map(|arg| self.interpret(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                
                let manifold = self.apply_builtin_modifier(name, &arguments, manifold_children, node.span.clone())?;
                Ok(Object::Manifold(manifold))
            }

            NodeKind::Call { name, arguments } => {
                let arguments = arguments.iter()
                    .map(|arg| self.interpret(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                self.call_builtin_function(name, &arguments, node.span.clone())
            },
            
            NodeKind::Binding { name, value } => {
                let value = self.interpret(&value)?;
                if !self.current_scope.add_binding(name.to_owned(), value.clone()) {
                    return Err(RuntimeError::new(
                        RuntimeErrorKind::DuplicateBinding(name.to_owned()),
                        node.span.clone(),
                    ))
                }
                Ok(value)
            },

            NodeKind::FieldAccess { value, field } => {
                let value = self.interpret(&value)?;

                if let Some(field_value) = value.get_field(field, &self.manifold_table) {
                    Ok(field_value)
                } else {
                    Err(RuntimeError::new(
                        RuntimeErrorKind::UndefinedField { field: field.clone(), ty: value.describe_type() },
                        node.span.clone(),
                    ))
                }
            }
        }
    }

    fn call_builtin_function(&mut self, name: &str, arguments: &[Object], span: InputSourceSpan) -> Result<Object, RuntimeError> {
        match name {
            "cube" => {
                let (x, y, z) = Self::get_vec3_from_arguments(arguments, span)?;
                Ok(Object::Manifold(self.manifold_table.add(Manifold::cube(x, y, z, false), ManifoldDisposition::Physical)))
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

            _ => Err(RuntimeError::new(
                RuntimeErrorKind::UndefinedIdentifier(name.to_owned()),
                span,
            ))
        }
    }

    fn apply_builtin_modifier(&mut self, name: &str, arguments: &[Object], mut children: Vec<ManifoldTableIndex>, span: InputSourceSpan) -> Result<ManifoldTableIndex, RuntimeError> {
        match name {
            "translate" => {
                let (x, y, z) = Self::get_vec3_from_arguments(arguments, span.clone())?;
                let manifold = self.union_modifier_children(children, span)?;
                Ok(self.manifold_table.map(manifold, |m| m.translate(x, y, z)))
            }

            "union" => {
                self.union_modifier_children(children, span)
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

                let subtrahend = self.union_modifier_children(children, span)?;
                let (subtrahend, _) = self.manifold_table.remove(subtrahend);

                Ok(self.manifold_table.map(minuend, |m| m.difference(&subtrahend)))
            }

            "buffer" => {
                let manifold = self.union_modifier_children(children, span)?;
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

    fn union_modifier_children(&mut self, mut children: Vec<ManifoldTableIndex>, span: InputSourceSpan) -> Result<ManifoldTableIndex, RuntimeError> {
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
}
