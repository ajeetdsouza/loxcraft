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

    #[remain::check]
    pub fn trace(&mut self) {
        while let Some(object) = self.gray_objects.pop() {
            if cfg!(feature = "gc-trace") {
                eprintln!("blacken {}: {object}", object.type_());
            }
            #[remain::sorted]
            match unsafe { (*object.common).type_ } {
                ObjectType::BoundMethod => {
                    let method = unsafe { object.bound_method };
                    self.mark(unsafe { (*method).this });
                    self.mark(unsafe { (*method).closure });
                }
                ObjectType::Class => {
                    let class = unsafe { object.class };
                    self.mark(unsafe { (*class).name });
                    for (&name, &method) in unsafe { &(*class).methods } {
                        self.mark(name);
                        self.mark(method);
                    }
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
                ObjectType::Instance => {
                    self.mark(unsafe { (*object.instance).class });
                    for (&name, &value) in unsafe { &(*object.instance).fields } {
                        self.mark(name);
                        self.mark(value);
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
        for idx in (0..self.objects.len()).rev() {
            let object = *unsafe { self.objects.get_unchecked(idx) };
            if unsafe { (*object.common).is_marked } {
                unsafe { (*object.common).is_marked = false };
            } else {
                self.objects.swap_remove(idx);
                object.free();
            }
        }

        self.strings.drain_filter(|_, &mut string| {
            if unsafe { (*string).is_marked } {
                unsafe { (*string).is_marked = false };
                false
            } else {
                unsafe { Box::from_raw(string) };
                true
            }
        });
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
        let object_ptr = Box::into_raw(Box::new(self));
        let object = object_ptr.into();

        if cfg!(feature = "gc-trace") {
            eprintln!("allocate {}: {object}", object.type_());
        }

        gc.objects.push(object);
        object_ptr
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
                if cfg!(feature = "gc-trace") {
                    eprintln!("allocate string: {string}");
                }
                let object = Box::into_raw(Box::new(ObjectString::new(unsafe {
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
        if !unsafe { (*object.common).is_marked } {
            if cfg!(feature = "gc-trace") {
                eprintln!("mark {}: {object}", object.type_());
            }
            unsafe { (*object.common).is_marked = true };
            gc.gray_objects.push(object);
        }
    }
}
