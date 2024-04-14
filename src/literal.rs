use crate::abstract_syntax_tree::{Statement, Value};
use crate::err_handle::{ChimeraRuntimeFailure, VarTypes};
use crate::frontend::Context;
use serde::de::{Error, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::cell::{Ref, RefCell, RefMut};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::rc::Rc;

#[derive(Debug, Copy, Clone)]
pub enum NumberKind {
    // Do I want to support more number types? There are functions in the deserialize impl
    // for visit_u128() and visit_i128() for larger int numbers. There are also numbers that
    // cannot be represented in i128, u128, or f64 because JSON numbers are strings of
    // arbitrary size
    I64(i64),
    U64(u64),
    F64(f64),
}

// TODO: Is there a way to handle floats here besides a lossy `as f64` cast? How can I convert them in such a way
//       where an error is returned if the conversion is lossy or fails. From<i64> and From<u64> are not implemented
//       for f64
impl PartialEq for NumberKind {
    fn eq(&self, other: &Self) -> bool {
        match self {
            NumberKind::I64(signed) => match other {
                NumberKind::I64(other_signed) => signed == other_signed,
                NumberKind::U64(other_unsigned) => match i64::try_from(other_unsigned.clone()) {
                    Ok(r) => signed == &r,
                    Err(_) => return false,
                },
                NumberKind::F64(other_float) => {
                    let self_as_float = signed.clone() as f64;
                    return self_as_float.eq(other_float);
                }
            },
            NumberKind::U64(unsigned) => match other {
                NumberKind::I64(other_signed) => match u64::try_from(other_signed.clone()) {
                    Ok(r) => unsigned == &r,
                    Err(_) => return false,
                },
                NumberKind::U64(other_unsigned) => unsigned == other_unsigned,
                NumberKind::F64(other_float) => {
                    let self_as_float = unsigned.clone() as f64;
                    return self_as_float.eq(other_float);
                }
            },
            NumberKind::F64(float) => match other {
                NumberKind::I64(other_signed) => float == &(other_signed.clone() as f64),
                NumberKind::U64(other_unsigned) => float == &(other_unsigned.clone() as f64),
                NumberKind::F64(other_float) => float == other_float,
            },
        }
    }
}

impl PartialOrd for NumberKind {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self {
            NumberKind::I64(signed) => {
                match other {
                    NumberKind::I64(other_signed) => return Some(signed.cmp(other_signed)),
                    NumberKind::U64(other_unsigned) => {
                        match i64::try_from(other_unsigned.clone()) {
                            Ok(v) => Some(signed.cmp(&v)),
                            // If a u64 cannot be converted into an i64 then it must be greater than
                            // i64::MAX. The i64 self is less than the u64 other
                            Err(_) => return Some(Ordering::Less),
                        }
                    }
                    NumberKind::F64(other_float) => {
                        let self_as_float = signed.clone() as f64;
                        self_as_float.partial_cmp(other_float)
                    }
                }
            }
            NumberKind::U64(unsigned) => {
                match other {
                    NumberKind::I64(other_signed) => match u64::try_from(other_signed.clone()) {
                        Ok(v) => Some(unsigned.cmp(&v)),
                        // If an i64 cannot be converted into a u64 then it must be less than 0.
                        // The u64 self, being positive, must be greater than the i64 other
                        Err(_) => return Some(Ordering::Greater),
                    },
                    NumberKind::U64(other_unsigned) => Some(unsigned.cmp(other_unsigned)),
                    NumberKind::F64(other_float) => {
                        let self_as_float = unsigned.clone() as f64;
                        self_as_float.partial_cmp(other_float)
                    }
                }
            }
            NumberKind::F64(float) => {
                let resolved_other: f64 = match other {
                    NumberKind::I64(other_signed) => other_signed.clone() as f64,
                    NumberKind::U64(other_unsigned) => other_unsigned.clone() as f64,
                    NumberKind::F64(other_float) => return float.partial_cmp(other_float),
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
            NumberKind::I64(signed) => write!(f, "{}", signed),
        }
    }
}

impl NumberKind {
    pub fn to_usize(&self) -> Option<usize> {
        match self {
            NumberKind::I64(signed) => usize::try_from(signed.clone()).ok(),
            NumberKind::U64(unsigned) => usize::try_from(unsigned.clone()).ok(),
            NumberKind::F64(_) => None,
        }
    }
    pub fn try_into_usize(
        &self,
        came_from: &Value,
        context: &Context,
    ) -> Result<usize, ChimeraRuntimeFailure> {
        Ok(self.to_usize().ok_or_else(|| {
            return ChimeraRuntimeFailure::VarWrongType(
                came_from.error_print(),
                VarTypes::Unsigned,
                context.current_line,
            );
        })?)
    }
}

// TODO: https://pest.rs/book/examples/json.html?highlight=optional#writing-the-grammar
//       If I want to support a full JSON value being stored here, like `var foo = LITERAL {"my_json":{"key":"val"}}

#[derive(Debug)]
pub struct Data {
    handle: Rc<RefCell<DataKind>>,
}

impl Clone for Data {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
        }
    }
}

impl Data {
    pub fn new(data_kind: DataKind) -> Self {
        Self {
            handle: Rc::new(RefCell::new(data_kind)),
        }
    }
    pub fn from_literal(lit: Literal) -> Self {
        Self {
            handle: Rc::new(RefCell::new(DataKind::Literal(lit))),
        }
    }
    pub fn from_vec(v: Vec<Data>) -> Self {
        Self {
            handle: Rc::new(RefCell::new(DataKind::Collection(Collection::List(v)))),
        }
    }
    pub fn borrow(&self, context: &Context) -> Result<Ref<DataKind>, ChimeraRuntimeFailure> {
        match self.handle.try_borrow() {
            // Must return a Ref<T> here, returning a Ref<T>::deref() will error.
            // This happens because RefCell<T>::try_borrow returns a Ref<T> with the lifetime of the &self passed into
            // this method. Calling deref on that Ref<T> will be a borrow of a borrow, where the second borrow will
            // go out of scope when this function ends. The first borrow has the lifetime of &self and can be
            // returned, because the caller gave us &self and knows what the lifetime is.
            Ok(d) => Ok(d),
            Err(_) => Err(ChimeraRuntimeFailure::BorrowError(
                context.current_line,
                "Cannot borrow a variable when it has a mutable reference in use".to_owned(),
            )),
        }
    }
    pub fn borrow_mut(&self, context: &Context) -> Result<RefMut<DataKind>, ChimeraRuntimeFailure> {
        match self.handle.try_borrow_mut() {
            Ok(d) => Ok(d),
            Err(_) => Err(ChimeraRuntimeFailure::BorrowError(
                context.current_line,
                "Cannot borrow a variable mutably when it already has a reference in use"
                    .to_owned(),
            )),
        }
    }
    pub fn resolve_access(
        &self,
        mut accessors: Vec<&str>,
        context: &Context,
    ) -> Result<Self, ChimeraRuntimeFailure> {
        accessors.reverse();
        let var_name = match accessors.len() {
            0 => {
                return Err(ChimeraRuntimeFailure::InternalError(
                    "resolving the access of a Literal".to_string(),
                ))
            }
            _ => accessors.pop().unwrap().to_owned(),
        };
        self.recursive_access(&mut accessors, context, var_name)
    }
    fn recursive_access(
        &self,
        accessors: &mut Vec<&str>,
        context: &Context,
        var_name: String,
    ) -> Result<Self, ChimeraRuntimeFailure> {
        let accessor = match accessors.pop() {
            Some(a) => a,
            None => return Ok(self.clone()),
        };
        let borrow = self.borrow(context)?;
        match borrow.deref() {
            DataKind::Collection(c) => match c {
                Collection::Object(obj) => match obj.get(accessor) {
                    Some(val) => val.recursive_access(accessors, context, var_name),
                    None => {
                        return Err(ChimeraRuntimeFailure::BadSubfieldAccess(
                            Some(var_name),
                            accessor.to_string(),
                            context.current_line,
                        ))
                    }
                },
                Collection::List(list) => {
                    let index: usize = match accessor.parse() {
                        Ok(i) => i,
                        Err(_) => {
                            return Err(ChimeraRuntimeFailure::TriedToIndexWithNonNumber(
                                context.current_line,
                            ))
                        }
                    };
                    match list.get(index) {
                        Some(val) => val.recursive_access(accessors, context, var_name),
                        None => {
                            return Err(ChimeraRuntimeFailure::OutOfBounds(context.current_line))
                        }
                    }
                }
            },
            DataKind::Literal(_) => {
                return Err(ChimeraRuntimeFailure::BadSubfieldAccess(
                    Some(var_name),
                    accessor.to_string(),
                    context.current_line,
                ))
            }
        }
    }
}

#[derive(Debug)]
pub enum DataKind {
    Literal(Literal),
    Collection(Collection),
}

impl Display for DataKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DataKind::Collection(c) => write!(f, "{}", c.to_string()),
            DataKind::Literal(l) => write!(f, "{}", l.to_string()),
        }
    }
}

impl PartialEq for DataKind {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Collection(self_c) => match other {
                Self::Collection(other_c) => self_c == other_c,
                _ => false,
            },
            Self::Literal(self_l) => match other {
                Self::Literal(other_l) => self_l == other_l,
                _ => false,
            },
        }
    }
}

impl DataKind {
    pub fn to_number(&self) -> Option<NumberKind> {
        match self {
            Self::Collection(_) => None,
            Self::Literal(literal) => literal.to_number(),
        }
    }
    fn to_list(&self) -> Option<&Vec<Data>> {
        match self {
            Self::Collection(c) => c.to_list(),
            _ => None,
        }
    }
    // TODO: Some of these try_into's take a Value and some take a String for came_from,
    //       this should be made consistent
    //       https://github.com/kyleoneill/chimerascript/issues/33
    pub fn try_into_literal(
        &self,
        came_from: &Value,
        context: &Context,
    ) -> Result<&Literal, ChimeraRuntimeFailure> {
        match self {
            Self::Literal(literal) => Ok(literal),
            Self::Collection(_) => Err(ChimeraRuntimeFailure::VarWrongType(
                came_from.error_print(),
                VarTypes::Literal,
                context.current_line,
            )),
        }
    }
    pub fn try_into_number_kind(
        &self,
        came_from: &Value,
        context: &Context,
    ) -> Result<NumberKind, ChimeraRuntimeFailure> {
        Ok(self.to_number().ok_or_else(|| {
            return ChimeraRuntimeFailure::VarWrongType(
                came_from.error_print(),
                VarTypes::Number,
                context.current_line,
            );
        })?)
    }
    pub fn try_into_usize(
        &self,
        came_from: &Value,
        context: &Context,
    ) -> Result<usize, ChimeraRuntimeFailure> {
        let number_kind = self.try_into_number_kind(came_from, context)?;
        number_kind.try_into_usize(came_from, context)
    }
    pub fn try_into_u64(
        &self,
        came_from: &Value,
        context: &Context,
    ) -> Result<u64, ChimeraRuntimeFailure> {
        if let Some(number) = self.to_number() {
            if let NumberKind::U64(unsigned) = number {
                return Ok(unsigned);
            }
        };
        return Err(ChimeraRuntimeFailure::VarWrongType(
            came_from.error_print(),
            VarTypes::Unsigned,
            context.current_line,
        ));
    }
    pub fn try_into_list(
        &self,
        came_from: String,
        context: &Context,
    ) -> Result<&Vec<Data>, ChimeraRuntimeFailure> {
        Ok(self.to_list().ok_or_else(|| {
            return ChimeraRuntimeFailure::VarWrongType(
                came_from,
                VarTypes::List,
                context.current_line,
            );
        })?)
    }
    pub fn try_into_string(
        &self,
        came_from: String,
        context: &Context,
    ) -> Result<&str, ChimeraRuntimeFailure> {
        let attempt = match self {
            Self::Literal(literal) => literal.to_str(),
            Self::Collection(_) => None,
        };
        Ok(attempt.ok_or_else(|| {
            return ChimeraRuntimeFailure::VarWrongType(
                came_from,
                VarTypes::String,
                context.current_line,
            );
        })?)
    }
}

#[derive(Debug)]
pub enum Collection {
    Object(HashMap<String, Data>),
    List(Vec<Data>),
}

impl PartialEq for Collection {
    fn eq(&self, other: &Self) -> bool {
        // TODO: https://github.com/kyleoneill/chimerascript/issues/33
        // borrow() should not need to take context
        let fake_context = Context { current_line: 0 };
        match self {
            Self::Object(self_obj) => match other {
                Self::Object(other_obj) => {
                    if self_obj.len() != other_obj.len() {
                        return false;
                    };
                    self_obj.iter().all(|(key, value)| {
                        other_obj.get(key).map_or(false, |v| {
                            v.borrow(&fake_context)
                                .expect("Failed to borrow object member")
                                .deref()
                                == value
                                    .borrow(&fake_context)
                                    .expect("Failed to borrow object member")
                                    .deref()
                        })
                    })
                }
                _ => false,
            },
            Self::List(self_list) => match other {
                Self::List(other_list) => {
                    if self_list.len() != other_list.len() {
                        return false;
                    };
                    (0..self_list.len()).into_iter().all(|i| {
                        self_list[i]
                            .borrow(&fake_context)
                            .expect("Failed to borrow list member")
                            .deref()
                            == other_list[i]
                                .borrow(&fake_context)
                                .expect("Failed to borrow list member")
                                .deref()
                    })
                }
                _ => false,
            },
        }
    }
}

impl Display for Collection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // TODO: https://github.com/kyleoneill/chimerascript/issues/33
        // borrow() should not need to take context
        let fake_context = Context { current_line: 0 };
        match self {
            Collection::Object(object) => {
                for (key, val) in object.iter() {
                    let val_string = match val.borrow(&fake_context) {
                        Ok(borrowed) => borrowed.to_string(),
                        Err(_) => return Err(std::fmt::Error),
                    };
                    write!(f, "{{\"{}\"}}\":\"{{{}}}\"", key, val_string)?;
                }
                Ok(())
            }
            Collection::List(list) => {
                // TODO: Should not be doing unwrap here, get rid of fake_context
                let list_as_str = list
                    .into_iter()
                    .map(|c| c.borrow(&fake_context).unwrap().to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(f, "[{}]", list_as_str)
            }
        }
    }
}

impl Collection {
    fn to_list(&self) -> Option<&Vec<Data>> {
        match self {
            Self::List(list) => Some(list),
            _ => None,
        }
    }
    pub fn contains(
        &self,
        contains_data: Ref<DataKind>,
        context: &Context,
    ) -> Result<bool, ChimeraRuntimeFailure> {
        match self {
            Collection::List(list) => {
                let borrowed_list_values: Result<Vec<_>, ChimeraRuntimeFailure> =
                    list.iter().map(|x| x.borrow(context)).collect();
                let rhs = contains_data.deref();
                let res = borrowed_list_values?
                    .into_iter()
                    .any(|member| member.deref() == rhs);
                Ok(res)
            }
            Collection::Object(map) => {
                // TODO: https://github.com/kyleoneill/chimerascript/issues/33
                // "key".to_string() is a stopgap hack, replace asap when resolving ^
                let key = contains_data.try_into_string("key".to_string(), context)?;
                Ok(map.contains_key(key))
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
    String(String),
    Number(NumberKind),
    Bool(bool),
    Null,
}

impl Display for Literal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::String(str) => write!(f, "{}", str),
            Literal::Number(num) => write!(f, "{}", num),
            Literal::Bool(bool) => write!(f, "{}", bool),
            Literal::Null => write!(f, "null"),
        }
    }
}

impl From<Statement> for Literal {
    fn from(statement: Statement) -> Self {
        match statement {
            Statement::Expression(expr) => match expr {
                crate::abstract_syntax_tree::Expression::LiteralExpression(literal) => literal,
                _ => panic!("Tried to convert a statement to a Literal but it was not one"),
            },
            _ => panic!(
                "Tried to convert a Statement to a Literal but it was not even an Expression"
            ),
        }
    }
}

impl Literal {
    pub fn to_number(&self) -> Option<NumberKind> {
        match self {
            Self::Number(i) => Some(*i),
            _ => None,
        }
    }
    pub fn to_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for DataKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DatakindVisitor;
        impl<'de> Visitor<'de> for DatakindVisitor {
            type Value = DataKind;
            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("any valid JSON value")
            }
            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(DataKind::Literal(Literal::Bool(v)))
            }
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(DataKind::Literal(Literal::Number(NumberKind::I64(v))))
            }
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(DataKind::Literal(Literal::Number(NumberKind::U64(v))))
            }
            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(DataKind::Literal(Literal::Number(NumberKind::F64(v))))
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                self.visit_string(String::from(v))
            }
            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: Error,
            {
                // Serde often interprets non-string values as strings for non-structured data like this interpreters
                // so we need to check if we have a non-string type
                // TODO: This fixes an issue I ran into, but this will cause a new problem of silently converting user
                //       input if a user _wants_ to use a stringified number. If a user expects the value "5" here they
                //       might not understand why they keep getting u64::5. This is not a permanent fix
                match v.parse::<u64>() {
                    Ok(unsigned_int) => {
                        return Ok(DataKind::Literal(Literal::Number(NumberKind::U64(
                            unsigned_int,
                        ))))
                    }
                    Err(_) => match v.parse::<i64>() {
                        Ok(signed_int) => {
                            return Ok(DataKind::Literal(Literal::Number(NumberKind::I64(
                                signed_int,
                            ))))
                        }
                        Err(_) => match v.parse::<f64>() {
                            Ok(float) => {
                                return Ok(DataKind::Literal(Literal::Number(NumberKind::F64(
                                    float,
                                ))))
                            }
                            Err(_) => (),
                        },
                    },
                }
                match v.parse::<bool>() {
                    Ok(boolean) => return Ok(DataKind::Literal(Literal::Bool(boolean))),
                    Err(_) => (),
                }
                Ok(DataKind::Literal(Literal::String(v)))
            }
            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(DataKind::Literal(Literal::Null))
            }
            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                Deserialize::deserialize(deserializer)
            }
            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(DataKind::Literal(Literal::Null))
            }
            fn visit_seq<A>(self, mut visitor: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut vec = Vec::new();
                while let Some(member) = visitor.next_element::<DataKind>()? {
                    vec.push(Data::new(member))
                }
                Ok(DataKind::Collection(Collection::List(vec)))
            }
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                match map.next_key()? {
                    Some(first_key) => {
                        let mut values: HashMap<String, Data> = HashMap::new();
                        let first_value = map.next_value::<DataKind>()?;
                        values.insert(first_key, Data::new(first_value));
                        while let Some((key, value)) = map.next_entry::<String, DataKind>()? {
                            values.insert(key, Data::new(value));
                        }
                        Ok(DataKind::Collection(Collection::Object(values)))
                    }
                    None => Ok(DataKind::Collection(Collection::Object(HashMap::new()))),
                }
            }
        }
        deserializer.deserialize_any(DatakindVisitor)
    }
}
