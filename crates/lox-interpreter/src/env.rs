use std::ops::Deref;

use gc::{Finalize, Gc, GcCell, Trace};
use rustc_hash::FxHashMap;

use crate::object::Object;

#[derive(Clone, Debug, Default, Finalize, Trace)]
pub struct Env(Gc<GcCell<EnvImpl>>);

impl Deref for Env {
    type Target = Gc<GcCell<EnvImpl>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Env {
    pub fn with_parent(parent: &Env) -> Self {
        let env = EnvImpl { map: FxHashMap::default(), parent: Some(parent.clone()) };
        Self(Gc::new(GcCell::new(env)))
    }

    pub fn contains(&self, name: &str) -> bool {
        self.borrow().map.contains_key(name)
    }

    pub fn get(&self, name: &str) -> Option<Object> {
        self.borrow().map.get(name).cloned()
    }

    pub fn get_at(&self, name: &str, depth: usize) -> Option<Object> {
        if depth == 0 {
            self.get(name)
        } else {
            self.borrow()
                .parent
                .as_ref()
                .unwrap_or_else(|| unreachable!("variable pointed to invalid scope: {name:?}"))
                .get_at(name, depth - 1)
        }
    }

    pub fn set(&mut self, name: &str, value: Object) -> Result<(), ()> {
        match self.borrow_mut().map.get_mut(name) {
            Some(entry) => {
                *entry = value;
                Ok(())
            }
            None => Err(()),
        }
    }

    pub fn set_at(&mut self, name: &str, value: Object, depth: usize) -> Result<(), ()> {
        if depth == 0 {
            self.set(name, value)
        } else {
            self.borrow_mut()
                .parent
                .as_mut()
                .unwrap_or_else(|| unreachable!("variable pointed to invalid scope: {name:?}"))
                .set_at(name, value, depth - 1)
        }
    }

    pub fn insert_unchecked(&mut self, name: &str, value: Object) {
        self.borrow_mut().map.insert(name.to_string(), value);
    }
}

#[derive(Debug, Default, Finalize, Trace)]
pub struct EnvImpl {
    map: FxHashMap<String, Object>,
    parent: Option<Env>,
}
