use std::{collections::HashMap, rc::Rc};

use crate::{RuntimeError, RuntimeErrorKind, object::Object};

pub struct LexicalScope {
    bindings: HashMap<String, Object>,
    pub parent: Option<Rc<LexicalScope>>,
}

impl LexicalScope {
    pub fn new_root() -> Self {
        Self {
            bindings: HashMap::new(),
            parent: None,
        }
    }

    pub fn new(parent: Rc<LexicalScope>) -> Self {
        Self {
            bindings: HashMap::new(),
            parent: Some(parent),
        }
    }

    pub fn get_binding(&self, name: &str) -> Option<&Object> {
        if let Some(object) = self.bindings.get(name) {
            return Some(object);
        }

        if let Some(parent) = self.parent.as_ref() {
            parent.get_binding(name)
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
}
