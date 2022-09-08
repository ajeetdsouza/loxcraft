use hashbrown::hash_map::{Entry, EntryRef, OccupiedEntry};
use hashbrown::{HashMap, HashSet};
use rustc_hash::FxHasher;
use std::hash::BuildHasherDefault;
use std::mem;

use crate::value::Object;

/// This is safe because we never modify any string that has been interned.
///
/// Any interned string will immediately be invalidated once [`Intern`] is
/// dropped.
///
#[derive(Default)]
pub struct Intern {
    strings: HashMap<String, *mut Object, BuildHasherDefault<FxHasher>>,
}

impl Intern {
    pub fn insert_str(&mut self, str: &str) -> (*mut Object, bool) {
        match self.strings.get(str) {
            Some(object) => (*object, false),
            None => {
                let string = str.to_string();
                let str: &'static str = unsafe { mem::transmute(string.as_str()) };
                let object = Box::into_raw(Box::new(str.into()));
                self.strings.insert_unique_unchecked(string, object);
                (object, true)
            }
        }
    }

    pub fn insert_string(&mut self, string: String) -> (*mut Object, bool) {
        match self.strings.get(&string) {
            Some(object) => (*object, false),
            None => {
                let str: &'static str = unsafe { mem::transmute(string.as_str()) };
                let object = Box::into_raw(Box::new(str.into()));
                self.strings.insert_unique_unchecked(string, object);
                (object, true)
            }
        }
    }
}

#[derive(Eq, PartialEq)]
pub enum InternState {
    /// The string has already been interned.
    Borrowed,
    /// The string has freshly been interned.
    Owned,
}
