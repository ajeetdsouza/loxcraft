use crate::object::Object;

use rustc_hash::FxHashMap;

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug, Default)]
pub struct Env {
    node: Rc<RefCell<EnvNode>>,
}

impl Env {
    pub fn with_parent(parent: &Env) -> Self {
        let node = EnvNode::with_parent(parent.node.clone());
        Self { node: Rc::new(RefCell::new(node)) }
    }

    pub fn get(&self, name: &str) -> Option<Object> {
        self.node.borrow().get(name)
    }

    pub fn get_at(&self, name: &str, depth: usize) -> Option<Object> {
        self.node.borrow().get_at(name, depth)
    }

    pub fn set(&mut self, name: &str, value: Object) {
        self.node.borrow_mut().set(name, value)
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

    fn get(&self, name: &str) -> Option<Object> {
        self.map.get(name).cloned()
    }

    fn get_at(&self, name: &str, depth: usize) -> Option<Object> {
        if depth == 0 {
            self.map.get(name).cloned()
        } else {
            self.parent
                .as_ref()
                .unwrap_or_else(|| unreachable!("local pointed to invalid scope"))
                .borrow()
                .get_at(name, depth - 1)
        }
    }

    fn set(&mut self, name: &str, value: Object) {
        self.map.insert(name.to_string(), value);
    }
}
