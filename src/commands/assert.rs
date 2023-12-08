use std::collections::HashMap;
use crate::abstract_syntax_tree::{AssertCommand, AssertSubCommand, AssignmentValue};
use crate::err_handle::{ChimeraRuntimeFailure, VarTypes};
use crate::frontend::Context;

pub fn assert_command(context: &Context, assert_command: AssertCommand, variable_map: &HashMap<String, AssignmentValue>) -> Result<(), ChimeraRuntimeFailure> {
    let left_value = AssignmentValue::resolve_value(&assert_command.left_value, variable_map, context)?;
    let right_value = AssignmentValue::resolve_value(&assert_command.right_value, variable_map, context)?;
    let assertion_passed = match assert_command.subcommand {
        AssertSubCommand::EQUALS => { left_value == right_value },
        AssertSubCommand::STATUS => {
            // left needs to be a web response variable
            if !right_value.is_numeric() { return Err(ChimeraRuntimeFailure::VarWrongType(assert_command.right_value.error_print(), VarTypes::Int, context.current_line)) }
            todo!()
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
