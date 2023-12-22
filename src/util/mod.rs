use std::collections::HashMap;
use serde_json::Value;
use crate::abstract_syntax_tree::Literal;
use crate::err_handle::ChimeraRuntimeFailure;
use crate::frontend::Context;

// TODO: This approach is better than the previous but is still going to cause a performance hit if data from a json
//       is repeatedly accessed which itself contains deeply nested data. The two paths forward are to maybe implement some kind
//       of caching when grabbing an array/object or to just deserialize a web request JSON body right into a Literal
pub fn access_json(json_value: &Value, accessors: &[&str], context: &Context) -> Result<Literal, ChimeraRuntimeFailure> {
    // TODO: Will probably be hard but there SHOULD be a way to do this without cloning
    //       cloning web requests might be extremely expensive
    //       Repeated usage of large requests is going to result in a lot of potentially expensive cloning
    match access_json_by_key(json_value, accessors, context)? {
        Value::Null => Ok(Literal::Null),
        Value::Bool(b) => Ok(Literal::Bool(*b)),
        // TODO: Need better number support here. Should be f64 if able or u64 if f64 and i64 both fail
        Value::Number(n) => {
            if !n.is_i64() {
                return Err(ChimeraRuntimeFailure::JsonBadNumberRead(context.current_line))
            }
            Ok(Literal::Number(n.as_i64().unwrap()))
        },
        Value::String(s) => Ok(Literal::String(s.clone())),
        Value::Array(a) => {
            let mut new_list: Vec<Literal> = Vec::new();
            for val in a {
                new_list.push(access_json(val, &[], context)?);
            }
            Ok(Literal::List(new_list))
        },
        Value::Object(o) => {
            let mut new_map: HashMap<String, Literal> = HashMap::new();
            for (k, v) in o.iter() {
                new_map.insert(k.to_owned(), access_json(v, &[], context)?);
            }
            Ok(Literal::Object(new_map))
        }
    }
}

// TODO: Replace the recursive walk for Object (and i guess Array?) in access_json to the following. This reduces the possibility
//       of a stack overflow. Not sure if that is a problem that can actually be reached, but web requests can be
//       pretty large and pretty nested
/*
traverse(Node node) {
  while (node) {
    if (node->current <= MAX_CHILD) {
      Node prev = node;
      if (node->child[node->current]) {
        node = node->child[node->current];
      }
      prev->current++;
    } else {
      // Do your thing with the node.
      node->current = 0; // Reset counter for next traversal.
      node = node->parent;
    }
  }
}

or

traverse(Node node)
{
  List<Node> nodes = [node];

  while (nodes.notEmpty) {
    Node n = nodes.shift();

    for (Node child in n.getChildren()) {
      nodes.add(child);
    }

    // do stuff with n, maybe
  }
}
 */

pub fn access_json_by_key<'a>(mut json_value: &'a Value, accessors: &[&str], context: &Context) -> Result<&'a Value, ChimeraRuntimeFailure> {
    if accessors.len() == 0 {
        return Ok(json_value);
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
