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

pub trait GcAlloc<T, U> {
    fn alloc(&mut self, object: T) -> U;
}

impl<T> GcAlloc<T, *mut T> for Gc
where
    *mut T: Into<Object>,
{
    fn alloc(&mut self, object: T) -> *mut T {
        let object = Box::into_raw(Box::new(object));
        self.objects.push(object.into());
        object
    }
}

impl<S> GcAlloc<S, *mut ObjectString> for Gc
where
    S: AsRef<str> + Into<String>,
{
    fn alloc(&mut self, str: S) -> *mut ObjectString {
        match self.strings.raw_entry_mut().from_key(str.as_ref()) {
            RawEntryMut::Occupied(entry) => *entry.get(),
            RawEntryMut::Vacant(entry) => {
                let string = str.into();
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

pub trait GcMark<T> {
    fn mark(&mut self, object: T);
}

impl GcMark<Value> for Gc {
    fn mark(&mut self, value: Value) {
        if let Value::Object(object) = value {
            self.mark(object);
        }
    }
}

impl<T: Into<Object>> GcMark<T> for Gc {
    fn mark(&mut self, object: T) {
        let object = object.into();
        if unsafe { (*object.common).is_marked } {
            unsafe { (*object.common).is_marked = true };
            self.gray_objects.push(object);
        }
    }
}
