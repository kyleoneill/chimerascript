use serde_json::Value;
use crate::abstract_syntax_tree::{AssignmentValue, Literal};
use crate::err_handle::ChimeraRuntimeFailure;
use crate::frontend::Context;

pub fn serde_json_to_string(json_object: &Value) -> String {
    let mut str_build = String::new();
    match json_object {
        Value::Null => str_build.push_str("null"),
        Value::Bool(b) => {
            if *b {
                str_build.push_str("true")
            }
            else {
                str_build.push_str("false")
            }
        },
        Value::Number(n) => {
            match n.as_i64() {
                Some(i) => str_build.push_str(i.to_string().as_str()),
                // TODO: Need error handling here
                None => str_build.push_str(i64::MAX.to_string().as_str())
            }
        },
        Value::String(s) => str_build.push_str(s.as_str()),
        Value::Array(a) => {
            let mut b_string = "[".to_owned();
            for val in a {
                let formatted = format!("{}, ", serde_json_to_string(val));
                b_string.push_str(formatted.as_str());
            }
            // We want to remove the last two chars to get rid of the last ", "
            let slice: &str = &b_string[0..b_string.len() - 2];
            str_build.push_str(slice);
            str_build.push(']');
        },
        Value::Object(json_obj) => {
            for (key, val) in json_obj.iter() {
                let val_string = serde_json_to_string(val);
                let b_string = format!("{{\"{}}}\":\"{{{}}}\"", key, val_string);
                str_build.push_str(b_string.as_str());
            }
        }
    }
    str_build
}

pub fn access_json(json_value: &Value, accessors: &[&str], context: &Context) -> Result<AssignmentValue, ChimeraRuntimeFailure> {
    // TODO: Will probably be hard but there SHOULD be a way to do this without cloning
    //       cloning web requests might be extremely expensive
    //       Repeated usage of large requests is going to result in a lot of expensive cloning
    match access_json_by_key(json_value, accessors, context)? {
        Value::Null => Ok(AssignmentValue::Literal(Literal::Null)),
        Value::Bool(b) => Ok(AssignmentValue::Literal(Literal::Bool(*b))),
        Value::Number(n) => {
            // TODO: What if we have a number outside the i32 range, or a float? Make this a smarter check, maybe add more Literals to represent ints vs floats
            if !n.is_i64() {
                return Err(ChimeraRuntimeFailure::JsonBadNumberRead(context.current_line))
            }
            Ok(AssignmentValue::Literal(Literal::Int(n.as_i64().unwrap())))
        },
        // TODO: If the web resource returns a stringified num or bool, should we check to convert that here?
        //       Ex, if we are accessing some (web_res.body.foo) where "foo" has been set to `"1"`, should
        //       we check to see if the value can be converted into an int and return it if so?
        Value::String(s) => Ok(AssignmentValue::Literal(Literal::Str(s.clone()))),
        // TODO: When Literal::List is added this should check to see if the Value::Array contains all
        //       primitives and, if so, convert it to a Literal::List. This might not be the case, like if
        //       the Value::Array contains a Value::Object
        Value::Array(a) => Ok(AssignmentValue::JsonValue(Value::Array(a.clone()))),
        Value::Object(o) => Ok(AssignmentValue::JsonValue(Value::Object(o.clone())))
    }
}

pub fn access_json_by_key<'a>(mut json_value: &'a Value, accessors: &[&str], context: &Context) -> Result<&'a Value, ChimeraRuntimeFailure> {
    if accessors.len() == 0 {
        return Err(ChimeraRuntimeFailure::InternalError("accessing a JSON object by key".to_owned()))
    }
    let last_index = accessors.len() - 1;
    for (i, &accessor) in accessors.iter().enumerate() {
        match json_value {
            Value::Array(json_array) => {
                match accessor.parse::<usize>() {
                    Ok(numerical_accessor) => {
                        if numerical_accessor >= json_array.len() {
                            return Err(ChimeraRuntimeFailure::OutOfBounds(context.current_line))
                        }
                        json_value = &json_array[numerical_accessor];
                    },
                    Err(_) => return Err(ChimeraRuntimeFailure::TriedToIndexWithNonNumber(context.current_line))
                }
            },
            Value::Object(json_object) => {
                match json_object.get(accessor) {
                    Some(read_value) => {
                        json_value = read_value;
                    },
                    None => return Err(ChimeraRuntimeFailure::BadSubfieldAccess(None, accessor.to_owned(), context.current_line))
                }
            },
            _ => return Err(ChimeraRuntimeFailure::BadSubfieldAccess(None, accessor.to_owned(), context.current_line))
        }
        if i == last_index {
            return Ok(json_value);
        }
    }
    Err(ChimeraRuntimeFailure::InternalError("accessing a JSON object by key".to_owned()))
}
