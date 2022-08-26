use std::fmt::{self, Display, Formatter};

use crate::object::{Callable, Class, Object};

use gc::{Finalize, Gc, GcCell, Trace};
use rustc_hash::FxHashMap;

#[derive(Clone, Debug, Finalize, Trace)]
pub struct Instance(Gc<GcCell<InstanceImpl>>);

impl Instance {
    pub fn new(class: &Class) -> Self {
        Self(Gc::new(GcCell::new(InstanceImpl {
            class: class.clone(),
            fields: FxHashMap::default(),
        })))
    }

    pub fn class(&self) -> Class {
        self.0.borrow().class.clone()
    }

    pub fn get(&self, name: &str) -> Option<Object> {
        self.0.borrow().fields.get(name).cloned()
    }

    pub fn set(&self, name: &str, value: Object) {
        self.0.borrow_mut().fields.insert(name.to_string(), value);
    }
}

impl Display for Instance {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<object {}>", self.class().name())
    }
}

impl Eq for Instance {}

#[allow(clippy::from_over_into)]
impl Into<Object> for Instance {
    fn into(self) -> Object {
        Object::Instance(self)
    }
}

impl PartialEq for Instance {
    fn eq(&self, other: &Self) -> bool {
        Gc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, Debug, Finalize, Trace)]
pub struct InstanceImpl {
    pub class: Class,
    pub fields: FxHashMap<String, Object>,
}
