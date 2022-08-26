use crate::object::Object;

use gc::{Finalize, Gc, GcCell, Trace};
use rustc_hash::FxHashMap;

use std::ops::Deref;

#[derive(Clone, Debug, Default, Finalize, Trace)]
pub struct Env(Gc<GcCell<EnvNode>>);

impl Deref for Env {
    type Target = Gc<GcCell<EnvNode>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Env {
    pub fn with_parent(parent: &Env) -> Self {
        let node = EnvNode::with_parent(parent.0.clone());
        Self(Gc::new(GcCell::new(node)))
    }

    pub fn contains(&self, name: &str) -> bool {
        self.borrow().contains(name)
    }

    pub fn get(&self, name: &str) -> Option<Object> {
        self.borrow().get(name)
    }

    pub fn get_at(&self, name: &str, depth: usize) -> Option<Object> {
        self.borrow().get_at(name, depth)
    }

    pub fn set(&mut self, name: &str, value: Object) -> Result<(), ()> {
        self.borrow_mut().set(name, value)
    }

    pub fn set_at(&mut self, name: &str, value: Object, depth: usize) -> Result<(), ()> {
        self.borrow_mut().set_at(name, value, depth)
    }

    pub fn insert_unchecked(&mut self, name: &str, value: Object) {
        self.borrow_mut().insert_unchecked(name, value)
    }
}

#[derive(Debug, Default, Finalize, Trace)]
pub struct EnvNode {
    map: FxHashMap<String, Object>,
    parent: Option<Gc<GcCell<EnvNode>>>,
}

impl EnvNode {
    fn with_parent(parent: Gc<GcCell<EnvNode>>) -> Self {
        Self { map: FxHashMap::default(), parent: Some(parent) }
    }

    fn contains(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }

    fn get(&self, name: &str) -> Option<Object> {
        self.map.get(name).cloned()
    }

    fn get_at(&self, name: &str, depth: usize) -> Option<Object> {
        if depth == 0 {
            self.get(name)
        } else {
            self.parent
                .as_ref()
                .unwrap_or_else(|| unreachable!("variable pointed to invalid scope: {name:?}"))
                .borrow()
                .get_at(name, depth - 1)
        }
    }

    fn set(&mut self, name: &str, value: Object) -> Result<(), ()> {
        match self.map.get_mut(name) {
            Some(entry) => {
                *entry = value;
                Ok(())
            }
            None => Err(()),
        }
    }

    fn set_at(&mut self, name: &str, value: Object, depth: usize) -> Result<(), ()> {
        if depth == 0 {
            self.set(name, value)
        } else {
            self.parent
                .as_ref()
                .unwrap_or_else(|| unreachable!("variable pointed to invalid scope: {name:?}"))
                .borrow_mut()
                .set_at(name, value, depth - 1)
        }
    }

    fn insert_unchecked(&mut self, name: &str, value: Object) {
        self.map.insert(name.to_string(), value);
    }
}
