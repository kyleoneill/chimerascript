use std::ops::Deref;
use crate::literal::{Collection, DataKind};
use crate::abstract_syntax_tree::{AssertCommand, AssertSubCommand};
use crate::err_handle::{ChimeraRuntimeFailure, VarTypes};
use crate::frontend::Context;
use crate::variable_map::VariableMap;

pub fn assert_command(context: &Context, assert_command: &AssertCommand, variable_map: &VariableMap) -> Result<(), ChimeraRuntimeFailure> {
    let left_binding = assert_command.left_value.resolve(context, variable_map)?;
    let left_data = left_binding.borrow(context)?;
    let right_binding = assert_command.right_value.resolve(context, variable_map)?;
    let right_data = right_binding.borrow(context)?;
    let assertion_passed = match assert_command.subcommand {
        AssertSubCommand::LENGTH => {
            let assert_len = right_data.try_into_usize(&assert_command.right_value, context)?;
            let vec = left_data.try_into_list(assert_command.left_value.error_print(), context)?;
            vec.len() == assert_len
        },
        AssertSubCommand::EQUALS => {
            // TODO: Should support checking equality for collection types as well
            let left_lit = left_data.try_into_literal(&assert_command.left_value, context)?;
            let right_lit = right_data.try_into_literal(&assert_command.right_value, context)?;
            left_lit == right_lit
        },
        AssertSubCommand::STATUS => {
            let res = match left_data.deref() {
                DataKind::Collection(c) => match c {
                    Collection::Object(obj) => {
                        match obj.get("status_code") {
                            Some(status_code) => {
                                let expected_code = right_data.try_into_u64(&assert_command.right_value, context)?;
                                let status_as_num = status_code.borrow(context)?.deref().try_into_u64(&assert_command.left_value, context)?;
                                Some(expected_code == status_as_num)
                            },
                            None => None
                        }
                    },
                    _ => None
                },
                _ =>  None
            };
            match res {
                Some(b) => b,
                None => return Err(ChimeraRuntimeFailure::VarWrongType(assert_command.left_value.error_print(), VarTypes::HttpResponse, context.current_line))
            }
        },
        AssertSubCommand::CONTAINS => {
            match left_data.deref() {
                DataKind::Collection(c) => c.contains(right_data, context)?,
                _ => return Err(ChimeraRuntimeFailure::VarWrongType(assert_command.left_value.error_print(), VarTypes::Containable, context.current_line))
            }
        },
        _ => {
            // The remaining matches are the four relational operators, left and right must both be numbers
            let numeric_left = left_data.try_into_number_kind(&assert_command.left_value, context)?;
            let numeric_right = right_data.try_into_number_kind(&assert_command.right_value, context)?;
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
        return Err(ChimeraRuntimeFailure::TestFailure(format!("Expected '{}' to not {} '{}'", assert_command.left_value.error_print(), assert_command.subcommand, assert_command.right_value.error_print()), context.current_line))
    }
    else if !assert_command.negate_assertion && !assertion_passed {
        // Assertion was false but expected to be true
        return Err(ChimeraRuntimeFailure::TestFailure(format!("Expected '{}' to {} '{}'", assert_command.left_value.error_print(), assert_command.subcommand, assert_command.right_value.error_print()), context.current_line))
    }
    Ok(())
}
