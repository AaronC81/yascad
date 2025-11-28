use std::{collections::HashMap, rc::Rc};

use manifold_rs::Manifold;
use yascad_frontend::{InputSourceSpan, Node, NodeKind};

use crate::{RuntimeError, RuntimeErrorKind, object::Object};

pub struct Interpreter {
    // TODO: scoping
    variables: HashMap<String, Object>, // Language is pure, don't need ability to mutate variables in-place
    top_level_manifolds: Vec<Rc<Manifold>>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            top_level_manifolds: vec![],
        }
    }

    pub fn build_top_level_manifold(&self) -> Manifold {
        let mut result = Manifold::new();
        for manifold in &self.top_level_manifolds {
            result = result.union(manifold);
        }

        result
    }

    // TODO: decide on exact semantics for this
    //       e.g. `virtual` needs to be able to return a manifold without adding to the model, 
    //       and assignments need to be "transparent" for their result to be correctly added here
    pub fn interpret_top_level(&mut self, node: &Node) -> Result<Object, RuntimeError> {
        let result = self.interpret(node)?;

        if let Object::Manifold(ref manifold) = result {
            self.top_level_manifolds.push(manifold.clone());
        }

        Ok(result)
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

            // TODO
            NodeKind::ModifierApplication { name, arguments, target } => {
                let target = self.interpret(target)?.as_manifold(node.span.clone())?;
                let arguments = arguments.iter()
                    .map(|arg| self.interpret(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                
                let manifold = self.apply_builtin_modifier(name, &arguments, target, node.span.clone())?;
                Ok(Object::Manifold(Rc::new(manifold)))
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
                Ok(Object::Manifold(Rc::new(Manifold::cube(x, y, z, false))))
            }

            _ => Err(RuntimeError::new(
                RuntimeErrorKind::UndefinedIdentifier(name.to_owned()),
                span,
            ))
        }
    }

    fn apply_builtin_modifier(&mut self, name: &str, arguments: &[Object], target: Rc<Manifold>, span: InputSourceSpan) -> Result<Manifold, RuntimeError> {
        match name {
            "translate" => {
                let (x, y, z) = Self::get_vec3_from_arguments(arguments, span)?;
                Ok(target.translate(x, y, z))
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
}
