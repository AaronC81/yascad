use std::{cell::RefCell, collections::HashMap, rc::{Rc, Weak}};

use yascad_frontend::Node;

use crate::{EvaluatedParameters, object::Object};

#[derive(Debug, Clone)]
pub struct UserDefinition {
    pub parameters: EvaluatedParameters,
    pub body: Vec<Node>,
    pub scope: Rc<RefCell<LexicalScope>>,
}

#[derive(Debug)]
pub struct LexicalScope {
    bindings: HashMap<String, Object>,
    operators: HashMap<String, UserDefinition>,
    modules: HashMap<String, UserDefinition>,
    functions: HashMap<String, UserDefinition>,
    pub parent: Option<Rc<RefCell<LexicalScope>>>,
}

impl LexicalScope {
    pub fn new_root() -> Self {
        Self {
            bindings: HashMap::new(),
            operators: HashMap::new(),
            modules: HashMap::new(),
            functions: HashMap::new(),
            parent: None,
        }
    }

    pub fn new(parent: Rc<RefCell<LexicalScope>>) -> Self {
        Self {
            bindings: HashMap::new(),
            operators: HashMap::new(),
            modules: HashMap::new(),
            functions: HashMap::new(),
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

    /// Add a new value binding to this scope.
    /// 
    /// Panics if a binding with this name already exists. It's the caller's responsibility to check
    /// for conflicts, as it may have names beyond the lexical scope which we don't know about.
    pub fn add_binding(&mut self, name: String, value: Object) {
        if self.get_binding(&name).is_some() {
            panic!("binding {name} already exists");
        }

        self.bindings.insert(name, value);
    }

    pub fn get_operator(&self, name: &str) -> Option<UserDefinition> {
        if let Some(item) = self.operators.get(name) {
            return Some(item.clone());
        }

        if let Some(parent) = self.parent.as_ref() {
            parent.borrow().get_operator(name)
        } else {
            None
        }
    }

    pub fn get_module(&self, name: &str) -> Option<UserDefinition> {
        if let Some(item) = self.modules.get(name) {
            return Some(item.clone());
        }

        if let Some(parent) = self.parent.as_ref() {
            parent.borrow().get_module(name)
        } else {
            None
        }
    }

    pub fn get_function(&self, name: &str) -> Option<UserDefinition> {
        if let Some(item) = self.functions.get(name) {
            return Some(item.clone());
        }

        if let Some(parent) = self.parent.as_ref() {
            parent.borrow().get_function(name)
        } else {
            None
        }
    }

    /// Add a new operator definition to this scope.
    /// 
    /// Panics if an operator with this name already exists. It's the caller's responsibility to
    /// check for conflicts, as it may have names beyond the lexical scope which we don't know about.
    pub fn add_operator(&mut self, name: String, definition: UserDefinition) {
        if self.get_operator(&name).is_some() {
            panic!("operator {name} already exists");
        }

        self.operators.insert(name, definition);
    }

    /// Add a new operator definition to this scope.
    /// 
    /// Panics if an operator with this name already exists. It's the caller's responsibility to
    /// check for conflicts, as it may have names beyond the lexical scope which we don't know about.
    pub fn add_module(&mut self, name: String, definition: UserDefinition) {
        if self.get_module(&name).is_some() {
            panic!("module {name} already exists");
        }

        self.modules.insert(name, definition);
    }

    /// Add a new function definition to this scope.
    /// 
    /// Panics if a function with this name already exists. It's the caller's responsibility to
    /// check for conflicts, as it may have names beyond the lexical scope which we don't know about.
    pub fn add_function(&mut self, name: String, definition: UserDefinition) {
        if self.get_function(&name).is_some() {
            panic!("function {name} already exists");
        }

        self.functions.insert(name, definition);
    }
}
