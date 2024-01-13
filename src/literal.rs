use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use serde::{Deserialize, Deserializer};
use serde::de::{Error, MapAccess, SeqAccess, Visitor};
use crate::abstract_syntax_tree::{Value, Statement};
use crate::err_handle::{ChimeraRuntimeFailure, VarTypes};
use crate::frontend::Context;

#[derive(Debug, Copy, Clone)]
pub enum NumberKind {
    // Do I want to support more number types? There are functions in the deserialize impl
    // for visit_u128() and visit_i128() for larger int numbers. There are also numbers that
    // cannot be represented in i128, u128, or f64 because JSON numbers are strings of
    // arbitrary size
    I64(i64),
    U64(u64),
    F64(f64)
}

// TODO: Is there a way to handle floats here besides a lossy `as f64` cast? How can I convert them in such a way
//       where an error is returned if the conversion is lossy or fails. From<i64> and From<u64> are not implemented
//       for f64
impl PartialEq for NumberKind {
    fn eq(&self, other: &Self) -> bool {
        match self {
            NumberKind::I64(signed) => {
                match other {
                    NumberKind::I64(other_signed) => signed == other_signed,
                    NumberKind::U64(other_unsigned) => { match i64::try_from(other_unsigned.clone()) { Ok(r) => signed == &r, Err(_) => return false } },
                    NumberKind::F64(other_float) => {
                        let self_as_float = signed.clone() as f64;
                        return self_as_float.eq(other_float)
                    }
                }
            },
            NumberKind::U64(unsigned) => {
                match other {
                    NumberKind::I64(other_signed) => { match u64::try_from(other_signed.clone()) { Ok(r) => unsigned == &r, Err(_) => return false } },
                    NumberKind::U64(other_unsigned) => unsigned == other_unsigned,
                    NumberKind::F64(other_float) => {
                        let self_as_float = unsigned.clone() as f64;
                        return self_as_float.eq(other_float)
                    }
                }
            },
            NumberKind::F64(float) => {
                match other {
                    NumberKind::I64(other_signed) => float == &(other_signed.clone() as f64),
                    NumberKind::U64(other_unsigned) => float == &(other_unsigned.clone() as f64),
                    NumberKind::F64(other_float) => float == other_float
                }
            }
        }
    }
}

impl PartialOrd for NumberKind {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self {
            NumberKind::I64(signed) => {
                match other {
                    NumberKind::I64(other_signed) => return Some(signed.cmp(other_signed)),
                    NumberKind::U64(other_unsigned) => match i64::try_from(other_unsigned.clone()) {
                        Ok(v) => Some(signed.cmp(&v)),
                        // If a u64 cannot be converted into an i64 then it must be greater than
                        // i64::MAX. The i64 self is less than the u64 other
                        Err(_) => return Some(Ordering::Less)
                    },
                    NumberKind::F64(other_float) => {
                        let self_as_float = signed.clone() as f64;
                        self_as_float.partial_cmp(other_float)
                    }
                }
            },
            NumberKind::U64(unsigned) => {
                match other {
                    NumberKind::I64(other_signed) => match u64::try_from(other_signed.clone()) {
                        Ok(v) => Some(unsigned.cmp(&v)),
                        // If an i64 cannot be converted into a u64 then it must be less than 0.
                        // The u64 self, being positive, must be greater than the i64 other
                        Err(_) => return Some(Ordering::Greater)
                    },
                    NumberKind::U64(other_unsigned) => Some(unsigned.cmp(other_unsigned)),
                    NumberKind::F64(other_float) => {
                        let self_as_float = unsigned.clone() as f64;
                        self_as_float.partial_cmp(other_float)
                    }
                }
            },
            NumberKind::F64(float) => {
                let resolved_other: f64 = match other {
                    NumberKind::I64(other_signed) => other_signed.clone() as f64,
                    NumberKind::U64(other_unsigned) => other_unsigned.clone() as f64,
                    NumberKind::F64(other_float) => return float.partial_cmp(other_float)
                };
                float.partial_cmp(&resolved_other)
            }
        }
    }
}

impl Display for NumberKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NumberKind::F64(float) => write!(f, "{}", float),
            NumberKind::U64(unsigned) => write!(f, "{}", unsigned),
            NumberKind::I64(signed) => write!(f, "{}", signed)
        }
    }
}

impl NumberKind {
    pub fn to_usize(&self) -> Option<usize> {
        match self {
            NumberKind::I64(signed) => usize::try_from(signed.clone()).ok(),
            NumberKind::U64(unsigned) => usize::try_from(unsigned.clone()).ok(),
            NumberKind::F64(_) => None
        }
    }
    pub fn try_into_usize(&self, came_from: &Value, context: &Context) -> Result<usize, ChimeraRuntimeFailure> {
        Ok(self.to_usize().ok_or_else(|| return ChimeraRuntimeFailure::VarWrongType(came_from.error_print(), VarTypes::Unsigned, context.current_line))?)
    }
}

// TODO: https://pest.rs/book/examples/json.html?highlight=optional#writing-the-grammar
//       If I want to support a full JSON value being stored here, like `var foo = LITERAL {"my_json":{"key":"val"}}

#[derive(Debug)]
pub struct Data {
    handle: Rc<RefCell<Literal>>
}

impl Clone for Data {
    fn clone(&self) -> Self {
        Self { handle: self.handle.clone() }
    }
}

impl PartialEq for Data {
    fn eq(&self, other: &Self) -> bool {
        // TODO: This will panic if the handle has a mutable borrow out
        //       I don't _think_ this is currently possible, but this must be changed
        //       to guard against future changes.
        // Realistically, the fix here is to probably break Literal down further to have some schema that looks like
        // Handle: Rc<RefCell<Data>>
        // Data: enum of Literal and Collectible
        // Literal: enum of String, Number, Bool, Null
        // Collectible: enum of List and Object
        // Will then need to swap over the derive de-serialize to go into Data
        // This will be another major refactor but likely necessary
        self.handle.borrow().deref() == other.handle.borrow().deref()
    }
}

impl Data {
    pub fn from_literal(lit: Literal) -> Self {
        Self { handle: Rc::new(RefCell::new(lit)) }
    }
    pub fn borrow(&self, context: &Context) -> Result<Ref<Literal>, ChimeraRuntimeFailure> {
        match self.handle.try_borrow() {
            // Must return a Ref<T> here, returning a Ref<T>::deref() will error.
            // This happens because RefCell<T>::try_borrow returns a Ref<T> with the lifetime of the &self passed into
            // this method. Calling deref on that Ref<T> will be a borrow of a borrow, where the second borrow will
            // go out of scope when this function ends. The first borrow has the lifetime of &self and can be
            // returned, because the caller gave us &self and knows what the lifetime is.
            Ok(d) => Ok(d),
            Err(_) => Err(ChimeraRuntimeFailure::BorrowError(context.current_line, "Cannot borrow a variable when it has a mutable reference in use".to_owned()))
        }
    }
    pub fn borrow_mut(&self, context: &Context) -> Result<RefMut<Literal>, ChimeraRuntimeFailure> {
        match self.handle.try_borrow_mut() {
            Ok(d) => Ok(d),
            Err(_) => Err(ChimeraRuntimeFailure::BorrowError(context.current_line, "Cannot borrow a variable mutably when it already has a reference in use".to_owned()))
        }
    }
    pub fn resolve_access(&self, mut accessors: Vec<&str>, context: &Context) -> Result<Self, ChimeraRuntimeFailure> {
        accessors.reverse();
        let var_name = match accessors.len() {
            0 => return Err(ChimeraRuntimeFailure::InternalError("resolving the access of a Literal".to_string())),
            _ => accessors.pop().unwrap().to_owned()
        };
        self.recursive_access(&mut accessors, context, var_name)
    }
    fn recursive_access(&self, accessors: &mut Vec<&str>, context: &Context, var_name: String) -> Result<Self, ChimeraRuntimeFailure> {
        let accessor = match accessors.pop() {
            Some(a) => a,
            None => return Ok(self.clone())
        };
        let borrow = self.borrow(context)?;
        match borrow.deref() {
            Literal::Object(obj) => {
                match obj.get(accessor) {
                    Some(val) => val.recursive_access(accessors, context, var_name),
                    None => return Err(ChimeraRuntimeFailure::BadSubfieldAccess(Some(var_name), accessor.to_string(), context.current_line))
                }
            },
            Literal::List(list) => {
                let index: usize = match accessor.parse() {
                    Ok(i) => i,
                    Err(_) => return Err(ChimeraRuntimeFailure::TriedToIndexWithNonNumber(context.current_line))
                };
                match list.get(index) {
                    Some(val) => val.recursive_access(accessors, context, var_name),
                    None => return Err(ChimeraRuntimeFailure::OutOfBounds(context.current_line))
                }
            },
            _ => return Err(ChimeraRuntimeFailure::BadSubfieldAccess(Some(var_name), accessor.to_string(), context.current_line))
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
    String(String),
    Number(NumberKind),
    Bool(bool),
    Null,
    Object(HashMap<String, Data>),
    List(Vec<Data>)
}

impl Display for Literal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // TODO: Need to resolve this, this is a temporary hack
        let fake_context = Context { current_line: 0 };
        match self {
            Literal::String(str) => write!(f, "{}", str),
            Literal::Number(num) => write!(f, "{}", num),
            Literal::Bool(bool) => write!(f, "{}", bool),
            Literal::Null => write!(f, "<null>"),
            Literal::Object(object) => {
                for (key, val) in object.iter() {
                    let val_string = match val.borrow(&fake_context) {
                        Ok(v) => v.to_string(),
                        Err(_) => return Err(std::fmt::Error)
                    };
                    write!(f, "{{\"{}\"}}\":\"{{{}}}\"", key, val_string)?;
                }
                Ok(())
            },
            Literal::List(list) => {
                // TODO: Should NOT be using an unwrap, this has to be fixed
                let list_as_str = list.into_iter().map(|c| c.borrow(&fake_context).unwrap().to_string()).collect::<Vec<String>>().join(", ");
                write!(f, "[{}]", list_as_str)
            }
        }
    }
}

impl From<Statement> for Literal {
    fn from(statement: Statement) -> Self {
        match statement {
            Statement::Expression(expr) => {
                match expr {
                    crate::abstract_syntax_tree::Expression::LiteralExpression(literal) => {
                        literal
                    },
                    _ => panic!("Tried to convert a statement to a Literal but it was not one")
                }
            },
            _ => panic!("Tried to convert a Statement to a Literal but it was not even an Expression")
        }
    }
}

impl Literal {
    pub fn to_number(&self) -> Option<NumberKind> {
        match self {
            Self::Number(i) => Some(*i),
            _ => None
        }
    }
    fn to_list(&self) -> Option<&Vec<Data>> {
        match self {
            Self::List(list) => Some(list),
            _ => None
        }
    }
    fn internal_to_string(&self) -> Option<&str> {
        match self {
            Self::String(string) => Some(string.as_str()),
            _ => None
        }
    }
    pub fn try_into_number_kind(&self, came_from: &Value, context: &Context) -> Result<NumberKind, ChimeraRuntimeFailure> {
        Ok(self.to_number().ok_or_else(|| return ChimeraRuntimeFailure::VarWrongType(came_from.error_print(), VarTypes::Number, context.current_line))?)
    }
    pub fn try_into_usize(&self, came_from: &Value, context: &Context) -> Result<usize, ChimeraRuntimeFailure> {
        let number_kind = self.try_into_number_kind(came_from, context)?;
        number_kind.try_into_usize(came_from, context)
    }
    pub fn try_into_u64(&self, came_from: &Value, context: &Context) -> Result<u64, ChimeraRuntimeFailure> {
        if let Some(number) = self.to_number() {
            if let NumberKind::U64(unsigned) = number {
                return Ok(unsigned)
            }
        };
        return Err(ChimeraRuntimeFailure::VarWrongType(came_from.error_print(), VarTypes::Unsigned, context.current_line))
    }
    pub fn try_into_list(&self, came_from: &Value, context: &Context) -> Result<&Vec<Data>, ChimeraRuntimeFailure> {
        Ok(self.to_list().ok_or_else(|| return ChimeraRuntimeFailure::VarWrongType(came_from.error_print(), VarTypes::List, context.current_line))?)
    }
    pub fn try_into_string(&self, came_from: &Value, context: &Context) -> Result<&str, ChimeraRuntimeFailure> {
        Ok(self.internal_to_string().ok_or_else(|| return ChimeraRuntimeFailure::VarWrongType(came_from.error_print(), VarTypes::String, context.current_line))?)
    }
}

impl <'de> Deserialize<'de> for Literal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        struct LiteralVisitor;
        impl<'de> Visitor<'de> for LiteralVisitor {
            type Value = Literal;
            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("any valid JSON value")
            }
            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> where E: Error {
                Ok(Literal::Bool(v))
            }
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> where E: Error {
                Ok(Literal::Number(NumberKind::I64(v)))
            }
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> where E: Error {
                Ok(Literal::Number(NumberKind::U64(v)))
            }
            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> where E: Error {
                Ok(Literal::Number(NumberKind::F64(v)))
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: Error {
                self.visit_string(String::from(v))
            }
            fn visit_string<E>(self, v: String) -> Result<Self::Value, E> where E: Error {
                Ok(Literal::String(v))
            }
            fn visit_none<E>(self) -> Result<Self::Value, E> where E: Error {
                Ok(Literal::Null)
            }
            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error> where D: Deserializer<'de> {
                Deserialize::deserialize(deserializer)
            }
            fn visit_unit<E>(self) -> Result<Self::Value, E> where E: Error {
                Ok(Literal::Null)
            }
            fn visit_seq<A>(self, mut visitor: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de> {
                let mut vec = Vec::new();
                while let Some(member) = visitor.next_element::<Literal>()? {
                    vec.push(Data::from_literal(member))
                }
                Ok(Literal::List(vec))
            }
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error> where A: MapAccess<'de> {
                match map.next_key()? {
                    Some(first_key) => {
                        let mut values: HashMap<String, Data> = HashMap::new();
                        let first_value = map.next_value::<Literal>()?;
                        values.insert(first_key, Data::from_literal(first_value));
                        while let Some((key, value)) = map.next_entry::<String, Literal>()? {
                            values.insert(key, Data::from_literal(value));
                        }
                        Ok(Literal::Object(values))
                    },
                    None => Ok(Literal::Object(HashMap::new()))
                }
            }
        }
        deserializer.deserialize_any(LiteralVisitor)
    }
}
