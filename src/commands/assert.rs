use std::collections::HashMap;
use crate::abstract_syntax_tree::{AssertCommand, AssertSubCommand, AssignmentValue, Literal};
use crate::err_handle::{ChimeraRuntimeFailure, VarTypes};
use crate::frontend::Context;

pub fn assert_command(context: &Context, assert_command: AssertCommand, variable_map: &HashMap<String, AssignmentValue>) -> Result<(), ChimeraRuntimeFailure> {
    let left_value = assert_command.left_value.resolve(context, variable_map)?;
    let right_value = assert_command.right_value.resolve_to_literal(context, variable_map)?;
    let assertion_passed = match assert_command.subcommand {
        AssertSubCommand::LENGTH => {
            let assert_len = right_value.try_into_number(&assert_command.right_value, context)? as usize;
            match &left_value {
                AssignmentValue::Literal(literal) => {
                    let vec = literal.try_into_list(&assert_command.left_value, context)?;
                    vec.len() == assert_len
                },
                _ => return Err(ChimeraRuntimeFailure::VarWrongType(assert_command.left_value.error_print(), VarTypes::Literal, context.current_line))
            }
        },
        AssertSubCommand::EQUALS => {
            match &left_value {
                AssignmentValue::Literal(left_literal) => left_literal == &right_value,
                AssignmentValue::HttpResponse(_) => false
            }
        },
        AssertSubCommand::STATUS => {
            match &left_value {
                AssignmentValue::HttpResponse(ref http_response) => {
                    let expected_code = right_value.try_into_number(&assert_command.right_value, context)?;
                    http_response.status_code as i64 == expected_code
                },
                _ => return Err(ChimeraRuntimeFailure::VarWrongType(assert_command.left_value.error_print(), VarTypes::HttpResponse, context.current_line))
            }
        },
        AssertSubCommand::CONTAINS => {
            match &left_value {
                AssignmentValue::Literal(literal) => {
                    match literal {
                        Literal::List(list) => {
                            list.contains(&right_value)
                        },
                        Literal::Object(map) => {
                            let key = right_value.try_into_string(&assert_command.right_value, context)?;
                            map.contains_key(key)
                        },
                        _ => return Err(ChimeraRuntimeFailure::VarWrongType(assert_command.left_value.error_print(), VarTypes::Containable, context.current_line))
                    }
                },
                AssignmentValue::HttpResponse(_) => {
                    // Resolving the left value should return a Literal, unless _just_ the http_response variable was
                    // used and in that case there is nothing to check a contains on
                    return Err(ChimeraRuntimeFailure::VarWrongType(assert_command.left_value.error_print(), VarTypes::Containable, context.current_line))
                }
            }
        },
        _ => {
            // The remaining matches are the four relational operators, left_value and
            // right_value must be ints for all four
            let literal_left = match left_value.to_literal() {
                Some(literal) => literal,
                None => return Err(ChimeraRuntimeFailure::VarWrongType(assert_command.left_value.error_print(), VarTypes::Int, context.current_line))
            };
            let numeric_left = literal_left.try_into_number(&assert_command.left_value, context)?;
            let numeric_right = right_value.try_into_number(&assert_command.right_value, context)?;
            match assert_command.subcommand {
                AssertSubCommand::GTE => {
                    numeric_left >= numeric_right
                },
                AssertSubCommand::GT => {
                    numeric_left > numeric_right
                },
                AssertSubCommand::LTE => {
                    numeric_left <= numeric_right
                },
                AssertSubCommand::LT => {
                    numeric_left < numeric_right
                }
                _ => panic!("Failed to handle an ASSERT subcommand case")
            }
        }
    };
    if assert_command.negate_assertion && assertion_passed {
        // Assertion was true but expected to be false
        return Err(ChimeraRuntimeFailure::TestFailure(format!("Expected '{}' to not {} '{}'", left_value, assert_command.subcommand, right_value), context.current_line))
    }
    else if !assert_command.negate_assertion && !assertion_passed {
        // Assertion was false but expected to be true
        return Err(ChimeraRuntimeFailure::TestFailure(format!("Expected '{}' to {} '{}'", left_value, assert_command.subcommand, right_value), context.current_line))
    }
    Ok(())
}
