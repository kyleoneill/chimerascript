use std::collections::HashMap;
use crate::abstract_syntax_tree::{AssertCommand, AssertSubCommand, AssignmentValue};
use crate::err_handle::{ChimeraRuntimeFailure, VarTypes};
use crate::frontend::Context;
use serde_json::Value;

pub fn assert_command(context: &Context, assert_command: AssertCommand, variable_map: &HashMap<String, AssignmentValue>) -> Result<(), ChimeraRuntimeFailure> {
    let left_value = AssignmentValue::resolve_value(&assert_command.left_value, variable_map, context)?;
    let right_value = AssignmentValue::resolve_value(&assert_command.right_value, variable_map, context)?;
    let assertion_passed = match assert_command.subcommand {
        AssertSubCommand::LENGTH => {
            if !right_value.is_numeric() { return Err(ChimeraRuntimeFailure::VarWrongType(assert_command.right_value.error_print(), VarTypes::Int, context.current_line)) }
            let assert_len = right_value.to_int() as usize;
            match &left_value {
                AssignmentValue::JsonValue(json_value) => {
                    match json_value {
                        Value::Array(json_array) => {
                            json_array.len() == assert_len
                        },
                        _ => return Err(ChimeraRuntimeFailure::VarWrongType(assert_command.left_value.error_print(), VarTypes::List, context.current_line))
                    }
                },
                AssignmentValue::List(list) => {
                    list.len() == assert_len
                },
                _ => return Err(ChimeraRuntimeFailure::VarWrongType(assert_command.left_value.error_print(), VarTypes::List, context.current_line))
            }
        },
        AssertSubCommand::EQUALS => { left_value == right_value },
        AssertSubCommand::STATUS => {
            match left_value {
                AssignmentValue::HttpResponse(ref http_response) => {
                    if !right_value.is_numeric() { return Err(ChimeraRuntimeFailure::VarWrongType(assert_command.right_value.error_print(), VarTypes::Int, context.current_line)) }
                    http_response.status_code as i64 == right_value.to_int()
                },
                _ => return Err(ChimeraRuntimeFailure::VarWrongType(assert_command.left_value.error_print(), VarTypes::HttpResponse, context.current_line))
            }
        },
        AssertSubCommand::CONTAINS => {
            match &left_value {
                AssignmentValue::List(list) => {
                    match right_value.resolve_to_literal() {
                        Some(right_literal) => {
                            list.contains(right_literal)
                        },
                        None => return Err(ChimeraRuntimeFailure::VarWrongType(assert_command.right_value.error_print(), VarTypes::Primitive, context.current_line))
                    }
                },
                // TODO: Support ASSERT CONTAINS for Json<Object> and Json<Array>
                //       Headache: Json<Object> must be indexed here by a string, and a Json<Array<Json>>
                //       must be indexed by a number but can return something that MIGHT be convertable to a
                //       LITERAL but might also be an Object?
                //       This is probably another knot that must be resolved by converting a JSON object immediately to
                //       some custom AssignmentValue variant to strip every non-Object into a Literal
                // If left is a serde_json::Value<Array> then right must be ?, depends on how ^ is resolved
                // If left is a serde_json::Value<Object> then right must be a string
                AssignmentValue::JsonValue(_json_value) => {
                    return Err(ChimeraRuntimeFailure::UnsupportedOperation("Using CONTAINS on a non-list".to_owned(), context.current_line))
                }
                _ => return Err(ChimeraRuntimeFailure::VarWrongType(assert_command.left_value.error_print(), VarTypes::Containable, context.current_line))
            }
        },
        _ => {
            // The remaining matches are the four relational operators, left_value and
            // right_value must be ints for all four
            if !left_value.is_numeric() { return Err(ChimeraRuntimeFailure::VarWrongType(assert_command.left_value.error_print(), VarTypes::Int, context.current_line)) }
            if !right_value.is_numeric() { return Err(ChimeraRuntimeFailure::VarWrongType(assert_command.right_value.error_print(), VarTypes::Int, context.current_line)) }
            match assert_command.subcommand {
                AssertSubCommand::GTE => {
                    left_value.to_int() >= right_value.to_int()
                },
                AssertSubCommand::GT => {
                    left_value.to_int() > right_value.to_int()
                },
                AssertSubCommand::LTE => {
                    left_value.to_int() <= right_value.to_int()
                },
                AssertSubCommand::LT => {
                    left_value.to_int() < right_value.to_int()
                }
                _ => panic!("Failed to handle an ASSERT subcommand case")
            }
        }
    };
    if assert_command.negate_assertion && assertion_passed {
        // Assertion was true but expected to be false
        return Err(ChimeraRuntimeFailure::TestFailure(format!("Expected {} to not {} {}", left_value, assert_command.subcommand, right_value), context.current_line))
    }
    else if !assert_command.negate_assertion && !assertion_passed {
        // Assertion was false but expected to be true
        return Err(ChimeraRuntimeFailure::TestFailure(format!("Expected {} to {} {}", left_value, assert_command.subcommand, right_value), context.current_line))
    }
    Ok(())
}
