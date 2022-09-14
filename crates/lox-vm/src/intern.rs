use crate::value::Object;
use hashbrown::hash_map::{Entry, RawEntryMut};
use hashbrown::HashMap;
use rustc_hash::FxHasher;
use std::hash::BuildHasherDefault;
use std::mem;

/// Any interned string will immediately be invalidated once [`Intern`] is
/// dropped.
#[derive(Default)]
pub struct Intern {
    strings: HashMap<String, *mut Object, BuildHasherDefault<FxHasher>>,
}

impl Intern {
    pub fn insert_str(&mut self, str: &str) -> (*mut Object, bool) {
        match self.strings.raw_entry_mut().from_key(str) {
            RawEntryMut::Occupied(entry) => (*entry.get(), false),
            RawEntryMut::Vacant(entry) => {
                let string = str.to_string();
                let str: &'static str = unsafe { mem::transmute(string.as_str()) };
                let object = Box::into_raw(Box::new(str.into()));
                entry.insert(string, object);
                (object, true)
            }
        }
    }

    pub fn insert_string(&mut self, string: String) -> (*mut Object, bool) {
        match self.strings.entry(string) {
            Entry::Occupied(entry) => (*entry.get(), false),
            Entry::Vacant(entry) => {
                let string: &'static str = unsafe { mem::transmute(entry.key().as_str()) };
                let object = Box::into_raw(Box::new(string.into()));
                entry.insert(object);
                (object, true)
            }
        }
    }
}

impl Drop for Intern {
    fn drop(&mut self) {
        for object in self.strings.values() {
            unsafe { Box::from_raw(*object) };
        }
    }
}
