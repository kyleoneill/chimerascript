use std::ops::Deref;
use crate::literal::Literal;
use crate::abstract_syntax_tree::{AssertCommand, AssertSubCommand};
use crate::err_handle::{ChimeraRuntimeFailure, VarTypes};
use crate::frontend::Context;
use crate::variable_map::VariableMap;

pub fn assert_command(context: &Context, assert_command: AssertCommand, variable_map: &VariableMap) -> Result<(), ChimeraRuntimeFailure> {
    // TODO: The right value should be resolved in each assertion command
    //       It does not make sense to just return false always on an assert equals, for example, and the current logic
    //       will cause an `ASSERT EQUALS (some_response) (some_response)` to return `false`
    let left_value = assert_command.left_value.resolve(context, variable_map)?;
    let right_value = assert_command.right_value.resolve(context, variable_map)?;
    let assertion_passed = match assert_command.subcommand {
        AssertSubCommand::LENGTH => {
            let assert_len = right_value.try_into_usize(&assert_command.right_value, context)?;
            let vec = left_value.try_into_list(&assert_command.left_value, context)?;
            vec.len() == assert_len
        },
        AssertSubCommand::EQUALS => left_value == right_value,
        AssertSubCommand::STATUS => {
            match &left_value {
                Literal::Object(obj) => {
                    match obj.get("status_code") {
                        Some(status_code) => {
                            let expected_code = right_value.try_into_u64(&assert_command.right_value, context)?;
                            let status_as_num = status_code.try_into_u64(&assert_command.left_value, context)?;
                            expected_code == status_as_num
                        },
                        None => return Err(ChimeraRuntimeFailure::VarWrongType(assert_command.left_value.error_print(), VarTypes::HttpResponse, context.current_line))
                    }
                },
                _ => return Err(ChimeraRuntimeFailure::VarWrongType(assert_command.left_value.error_print(), VarTypes::HttpResponse, context.current_line))
            }
        },
        AssertSubCommand::CONTAINS => {
            match left_value {
                Literal::List(list) => list.iter().any(|list_member| list_member.deref() == right_value),
                Literal::Object(map) => {
                    let key = right_value.try_into_string(&assert_command.right_value, context)?;
                    map.contains_key(key)
                },
                _ => return Err(ChimeraRuntimeFailure::VarWrongType(assert_command.left_value.error_print(), VarTypes::Containable, context.current_line))
            }
        },
        _ => {
            // The remaining matches are the four relational operators, left and right must both be numbers
            let numeric_left = left_value.try_into_number_kind(&assert_command.left_value, context)?;
            let numeric_right = right_value.try_into_number_kind(&assert_command.right_value, context)?;
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
