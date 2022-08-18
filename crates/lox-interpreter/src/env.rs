use crate::error::{Error, NameError, Result};
use crate::object::Object;

use rustc_hash::FxHashMap;

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug, Default)]
pub struct Env(Rc<RefCell<EnvNode>>);

impl Env {
    pub fn with_parent(parent: &Env) -> Self {
        let node = EnvNode::with_parent(parent.0.clone());
        Self(Rc::new(RefCell::new(node)))
    }

    pub fn get(&self, name: &str) -> Result<Object> {
        self.0.borrow().get(name)
    }

    pub fn get_at(&self, name: &str, depth: usize) -> Result<Object> {
        self.0.borrow().get_at(name, depth)
    }

    pub fn set(&mut self, name: &str, value: Object) -> Result<()> {
        self.0.borrow_mut().set(name, value)
    }

    pub fn set_at(&mut self, name: &str, value: Object, depth: usize) -> Result<()> {
        self.0.borrow_mut().set_at(name, value, depth)
    }

    pub fn insert_unchecked(&mut self, name: &str, value: Object) {
        self.0.borrow_mut().insert_unchecked(name, value)
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

    fn get(&self, name: &str) -> Result<Object> {
        match self.map.get(name) {
            Some(value) => Ok(value.clone()),
            None => Err(Error::NameError(NameError::NotDefined { name: name.to_string() })),
        }
    }

    fn get_at(&self, name: &str, depth: usize) -> Result<Object> {
        if depth == 0 {
            self.get(name)
        } else {
            self.parent
                .as_ref()
                .unwrap_or_else(|| unreachable!("variable pointed to invalid scope: {:?}", name))
                .borrow()
                .get_at(name, depth - 1)
        }
    }

    fn set(&mut self, name: &str, value: Object) -> Result<()> {
        match self.map.get_mut(name) {
            Some(entry) => {
                *entry = value;
                Ok(())
            }
            None => Err(Error::NameError(NameError::NotDefined { name: name.to_string() })),
        }
    }

    fn set_at(&mut self, name: &str, value: Object, depth: usize) -> Result<()> {
        if depth == 0 {
            self.set(name, value)
        } else {
            self.parent
                .as_ref()
                .unwrap_or_else(|| unreachable!("variable pointed to invalid scope: {:?}", name))
                .borrow_mut()
                .set_at(name, value, depth - 1)
        }
    }

    fn insert_unchecked(&mut self, name: &str, value: Object) {
        self.map.insert(name.to_string(), value);
    }
}
