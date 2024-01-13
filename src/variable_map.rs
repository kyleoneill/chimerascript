use std::rc::Rc;
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::ops::Deref;
use crate::literal::Literal;
use crate::err_handle::ChimeraRuntimeFailure;
use crate::frontend::Context;

/*
Variables are stored in an Rc<RefCell<Literal>>

Rc is the ownership layer of the variable. It allows us to have multiple references to a thing.
This is needed when we do something like:
var foo = LIST NEW [1,2,3]
var bar = (foo.1)
Here, when bar is being assigned we will make sure that foo is a list with an index of 1 and then clone it.
The clone, rather than cloning all of the data at that index, will increase the reference count.
The same will occur for setting a variable equal to an access within a Literal(Object)

RefCell is used to mutate data in multiple places, it acts like Rust's borrow checker except the checks are done
at runtime. This will allow mutation of variables, which is needed to modify lists and objects

-------------------------
RefCell vs Cell
Cell has restrictions. It does not impl Send/Sync and is not thread safe. Data operates on a transaction level. To
read or write, the data must be removed from the cell, read/modified, and then returned into the cell.

RefCell has overhead as it tracks active references to it (multiple immutable borrows or just a single mutable borrow,
like the borrow checker). The data in a RefCell, unlike a Cell, can be read/modified in place and does not need to be
removed and replaced.
*/

pub struct VariableMap {
    map: HashMap<String, Rc<RefCell<Literal>>>
}

impl VariableMap {
    pub fn new() -> Self {
        Self { map: HashMap::new() }
    }
    pub fn get(&self, context: &Context, key: &str) -> Result<&Literal, ChimeraRuntimeFailure> {
        // Rc is "transparent", we can call RefCell's methods directly on var_value despite it being an &Rc
        // All Rc methods must be called like Rc::Foo() so they do not conflict with methods of the inner value
        match self.map.get(key) {
            Some(var_value) => {
                match var_value.try_borrow() {
                    Ok(borrowed_value) => Ok(borrowed_value.deref()),
                    // TODO: Should this be returning an InternalError? Is there a better err to be returning here?
                    Err(_) => ChimeraRuntimeFailure::InternalError("Trying to immutably borrow a reference".to_owned())
                }
            }
            None => Err(ChimeraRuntimeFailure::VarNotFound(key.to_owned(), context.current_line))
        }
    }
    pub fn get_mut(&self, context: &Context, key: &str) -> Result<RefMut<Literal>, ChimeraRuntimeFailure> {
        match self.map.get(key) {
            Some(var_value) => {
                match var_value.try_borrow_mut() {
                    Ok(borrowed_value) => Ok(borrowed_value),
                    // TODO: Should this be returning an InternalError? Is there a better err to be returning here?
                    Err(_) => ChimeraRuntimeFailure::InternalError("Trying to immutably borrow a reference".to_owned())
                }
            }
            None => Err(ChimeraRuntimeFailure::VarNotFound(key.to_owned(), context.current_line))
        }
    }
    pub fn insert(&mut self, key: String, value: Rc<RefCell<Literal>>) {
        self.map.insert(key, value);
    }
}
