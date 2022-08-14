use crate::object::Object;
use crate::{NameError, Result, RuntimeError, Span};

use std::cell::RefCell;
use std::rc::Rc;

use rustc_hash::FxHashMap;

#[derive(Clone, Debug, Default)]
pub struct Env {
    node: Rc<RefCell<EnvNode>>,
}

impl Env {
    pub fn with_parent(parent: &Env) -> Self {
        let node = EnvNode::with_parent(parent.node.clone());
        Self { node: Rc::new(RefCell::new(node)) }
    }

    pub fn read(&self, name: &str, span: &Span) -> Result<Object> {
        self.node.borrow().read(name, span)
    }

    pub fn define(&mut self, name: &str, value: Object, span: &Span) -> Result<()> {
        self.node.borrow_mut().define(name, value, span)
    }

    pub fn assign(&mut self, name: &str, value: Object, span: &Span) -> Result<()> {
        self.node.borrow_mut().assign(name, value, span)
    }
}

#[derive(Debug, Default)]
struct EnvNode {
    map: FxHashMap<String, Object>,
    parent: Option<Rc<RefCell<EnvNode>>>,
}

impl EnvNode {
    fn with_parent(parent: Rc<RefCell<EnvNode>>) -> Self {
        Self { map: FxHashMap::default(), parent: Some(parent) }
    }

    fn read(&self, name: &str, span: &Span) -> Result<Object> {
        if let Some(object) = self.map.get(name) {
            Ok(object.clone())
        } else if let Some(parent) = &self.parent {
            parent.as_ref().borrow().read(name, span)
        } else {
            Err(RuntimeError::NameError(NameError::NotDefined {
                name: name.to_string(),
                span: span.clone(),
            }))
        }
    }

    // var foo = 123;
    fn define(&mut self, name: &str, value: Object, span: &Span) -> Result<()> {
        if !self.is_global() && self.map.contains_key(name) {
            return Err(RuntimeError::NameError(NameError::AlreadyDefined {
                name: name.to_string(),
                span: span.clone(),
            }));
        }
        self.map.insert(name.to_string(), value);
        Ok(())
    }

    // foo = 123;
    fn assign(&mut self, name: &str, value: Object, span: &Span) -> Result<()> {
        if self.map.get(name).is_some() {
            self.map.insert(name.to_string(), value);
            Ok(())
        } else if let Some(parent) = &self.parent {
            parent.as_ref().borrow_mut().assign(name, value, span)
        } else {
            Err(RuntimeError::NameError(NameError::NotDefined {
                name: name.to_string(),
                span: span.clone(),
            }))
        }
    }

    fn is_global(&self) -> bool {
        self.parent.is_none()
    }
}
