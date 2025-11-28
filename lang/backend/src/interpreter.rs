use std::{collections::HashMap, rc::Rc};

use manifold_rs::Manifold;
use yascad_frontend::{InputSourceSpan, Node, NodeKind};

use crate::{RuntimeError, RuntimeErrorKind, manifold_table::{ManifoldDisposition, ManifoldTable, ManifoldTableIndex}, object::Object};

pub struct Interpreter {
    // TODO: scoping
    variables: HashMap<String, Object>, // Language is pure, don't need ability to mutate variables in-place
    manifold_table: ManifoldTable,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            manifold_table: ManifoldTable::new(),
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
                self.variables.get(id)
                    .ok_or_else(|| RuntimeError::new(
                        RuntimeErrorKind::UndefinedIdentifier(id.to_owned()),
                        node.span.clone(),
                    ))
                    .cloned()
            },

            NodeKind::NumberLiteral(num) => {
                Ok(Object::Number(*num))
            },

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
        }
    }

    fn call_builtin_function(&mut self, name: &str, arguments: &[Object], span: InputSourceSpan) -> Result<Object, RuntimeError> {
        match name {
            "cube" => {
                let (x, y, z) = Self::get_vec3_from_arguments(arguments, span)?;
                Ok(Object::Manifold(self.manifold_table.add(Manifold::cube(x, y, z, false), ManifoldDisposition::Physical)))
            }

            _ => Err(RuntimeError::new(
                RuntimeErrorKind::UndefinedIdentifier(name.to_owned()),
                span,
            ))
        }
    }

    fn apply_builtin_modifier(&mut self, name: &str, arguments: &[Object], children: Vec<ManifoldTableIndex>, span: InputSourceSpan) -> Result<ManifoldTableIndex, RuntimeError> {
        match name {
            "translate" => {
                let (x, y, z) = Self::get_vec3_from_arguments(arguments, span.clone())?;
                let manifold = self.union_modifier_children(children, span)?;
                Ok(self.manifold_table.map(manifold, |m| m.translate(x, y, z)))
            }

            _ => Err(RuntimeError::new(
                RuntimeErrorKind::UndefinedIdentifier(name.to_owned()),
                span,
            ))
        }
    }

    fn get_vec3_from_arguments(arguments: &[Object], span: InputSourceSpan) -> Result<(f64, f64, f64), RuntimeError> {
        // TODO: this should take a vec2/vec3 later, but that's not implemented in the language yet.
        //       instead, take the sizes as individual arguments

        if arguments.len() != 2 && arguments.len() != 3 {
            return Err(RuntimeError::new(
                RuntimeErrorKind::IncorrectArity { expected: 2..=3, actual: arguments.len() },
                span,
            ));
        }

        let x = arguments[0].as_number(span.clone())?;
        let y = arguments[1].as_number(span.clone())?;

        let z =
            if arguments.len() == 3 {
                arguments[2].as_number(span.clone())?
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
