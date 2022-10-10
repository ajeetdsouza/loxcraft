use std::hash::BuildHasherDefault;
use std::mem;

use hashbrown::hash_map::RawEntryMut;
use hashbrown::HashMap;
use rustc_hash::FxHasher;

use crate::object::{Object, ObjectString, ObjectType};
use crate::value::Value;

#[derive(Default)]
pub struct Gc {
    strings: HashMap<String, *mut ObjectString, BuildHasherDefault<FxHasher>>,
    objects: Vec<Object>,
    gray_objects: Vec<Object>,
}

impl Gc {
    pub fn alloc<T>(&mut self, object: impl GcAlloc<T>) -> T {
        object.alloc(self)
    }

    pub fn mark(&mut self, object: impl GcMark) {
        object.mark(self);
    }

    pub fn unmark_all(&mut self) {
        for object in &self.objects {
            unsafe { (*object.common).is_marked = true };
        }
    }

    pub fn trace(&mut self) {
        while let Some(object) = self.gray_objects.pop() {
            match unsafe { (*object.common).type_ } {
                ObjectType::Class => {
                    let class = unsafe { object.class };
                    self.mark(unsafe { (*class).name });
                }
                ObjectType::Closure => {
                    let closure = unsafe { object.closure };
                    self.mark(unsafe { (*closure).function });
                    for &upvalue in unsafe { &(*closure).upvalues } {
                        self.mark(upvalue);
                    }
                }
                ObjectType::Function => {
                    let function = unsafe { object.function };
                    self.mark(unsafe { (*function).name });
                    for constant in unsafe { &(*function).chunk.constants } {
                        if let &Value::Object(object) = constant {
                            self.mark(object);
                        }
                    }
                }
                ObjectType::String => {}
                ObjectType::Upvalue => {
                    let upvalue = unsafe { object.upvalue };
                    self.mark(unsafe { (*upvalue).closed });
                }
            }
        }
    }

    pub fn sweep(&mut self) {
        // TODO: benchmark against `drain_filter`
        for idx in (0..self.objects.len()).rev() {
            let object = self.objects[idx];
            if unsafe { (*object.common).is_marked } {
                unsafe { (*object.common).is_marked = false };
            } else {
                self.objects.swap_remove(idx);
                object.free();
            }
        }

        for (_, string) in self
            .strings
            .drain_filter(|_, &mut string| !unsafe { (*string).is_marked })
        {
            unsafe { Box::from_raw(string) };
        }
    }
}

impl Drop for Gc {
    fn drop(&mut self) {
        for object in &self.objects {
            object.free();
        }
        for &string in self.strings.values() {
            unsafe { Box::from_raw(string) };
        }
    }
}

pub trait GcAlloc<T> {
    fn alloc(self, gc: &mut Gc) -> T;
}

impl<T> GcAlloc<*mut T> for T
where
    *mut T: Into<Object>,
{
    fn alloc(self, gc: &mut Gc) -> *mut T {
        let object = Box::into_raw(Box::new(self));
        gc.objects.push(object.into());
        object
    }
}

impl<S> GcAlloc<*mut ObjectString> for S
where
    S: AsRef<str> + Into<String>,
{
    fn alloc(self, gc: &mut Gc) -> *mut ObjectString {
        match gc.strings.raw_entry_mut().from_key(self.as_ref()) {
            RawEntryMut::Occupied(entry) => *entry.get(),
            RawEntryMut::Vacant(entry) => {
                let string = self.into();
                let object =
                    Box::into_raw(Box::new(ObjectString::new(unsafe {
                        mem::transmute(string.as_str())
                    })));
                entry.insert(string, object);
                object
            }
        }
    }
}

pub trait GcMark {
    fn mark(self, gc: &mut Gc);
}

impl GcMark for Value {
    fn mark(self, gc: &mut Gc) {
        if let Value::Object(object) = self {
            object.mark(gc);
        }
    }
}

impl<T: Into<Object>> GcMark for T {
    fn mark(self, gc: &mut Gc) {
        let object = self.into();
        if unsafe { (*object.common).is_marked } {
            unsafe { (*object.common).is_marked = true };
            gc.gray_objects.push(object);
        }
    }
}