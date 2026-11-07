//! The patch queue sent to the page. Region updates are diffed by content hash so
//! re-rendering a region that did not change sends nothing.

use serde_json::{json, Value};
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

#[derive(Default)]
pub struct Ui {
    /// Ops that must run before region updates (in push order).
    pub ops: Vec<Value>,
    /// Effects that need the regions in place first (focus, reveal, live editor…).
    pub late: Vec<Value>,
    cache: HashMap<String, u64>,
}

pub fn hash<T: Hash + ?Sized>(v: &T) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

impl Ui {
    fn changed(&mut self, key: String, value: u64) -> bool {
        if self.cache.get(&key) == Some(&value) {
            return false;
        }
        self.cache.insert(key, value);
        true
    }
    pub fn html(&mut self, id: &str, h: String) {
        if self.changed(format!("h#{id}"), hash(&h)) {
            self.ops.push(json!({"op": "html", "id": id, "h": h}));
        }
    }
    /// Forget what we last sent for a region (the page changed it behind our back).
    pub fn forget(&mut self, id: &str) {
        self.cache.remove(&format!("h#{id}"));
    }
    pub fn attr(&mut self, sel: &str, k: &str, v: Option<&str>) {
        if self.changed(format!("a#{sel}#{k}"), hash(&v)) {
            self.ops.push(json!({"op": "attr", "sel": sel, "k": k, "v": v}));
        }
    }
    pub fn cls(&mut self, sel: &str, c: &str, on: bool) {
        if self.changed(format!("c#{sel}#{c}"), on as u64) {
            self.ops.push(json!({"op": "cls", "sel": sel, "c": c, "on": on}));
        }
    }
    pub fn css(&mut self, k: &str, v: String) {
        if self.changed(format!("s#{k}"), hash(&v)) {
            self.ops.push(json!({"op": "css", "k": k, "v": v}));
        }
    }
    pub fn now(&mut self, op: Value) {
        self.ops.push(op);
    }
    pub fn later(&mut self, op: Value) {
        self.late.push(op);
    }
    pub fn reset(&mut self) {
        self.cache.clear();
    }
    pub fn take(&mut self) -> Vec<Value> {
        let mut out = std::mem::take(&mut self.ops);
        out.append(&mut self.late);
        out
    }
}
