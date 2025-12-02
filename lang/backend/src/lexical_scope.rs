use std::{cell::RefCell, collections::HashMap, rc::Rc};

use yascad_frontend::{Node, NodeKind};

use crate::object::Object;

#[derive(Debug)]
pub struct LexicalScope {
    bindings: HashMap<String, Object>,
    operators: HashMap<String, Node>, // Always `Node::OperatorDefinition`
    pub parent: Option<Rc<RefCell<LexicalScope>>>,
}

impl LexicalScope {
    pub fn new_root() -> Self {
        Self {
            bindings: HashMap::new(),
            operators: HashMap::new(),
            parent: None,
        }
    }

    pub fn new(parent: Rc<RefCell<LexicalScope>>) -> Self {
        Self {
            bindings: HashMap::new(),
            operators: HashMap::new(),
            parent: Some(parent),
        }
    }

    pub fn get_binding(&self, name: &str) -> Option<Object> {
        if let Some(object) = self.bindings.get(name) {
            return Some(object.clone());
        }

        if let Some(parent) = self.parent.as_ref() {
            parent.borrow().get_binding(name)
        } else {
            None
        }
    }

    #[must_use = "return value indicates whether a binding was created successfully"]
    pub fn add_binding(&mut self, name: String, value: Object) -> bool {
        if self.get_binding(&name).is_some() {
            return false
        }

        self.bindings.insert(name, value);
        true
    }

    pub fn get_operator(&self, name: &str) -> Option<Node> {
        if let Some(node) = self.operators.get(name) {
            return Some(node.clone());
        }

        if let Some(parent) = self.parent.as_ref() {
            parent.borrow().get_operator(name)
        } else {
            None
        }
    }

    #[must_use = "return value indicates whether an operator was created successfully"]
    pub fn add_operator(&mut self, name: String, definition: Node) -> bool {
        if !matches!(definition.kind, NodeKind::OperatorDefinition { .. }) {
            panic!("lexical scope operators must have NodeKind::OperatorDefinition")
        }

        if self.get_operator(&name).is_some() {
            return false
        }

        self.operators.insert(name, definition);
        true
    }
}
