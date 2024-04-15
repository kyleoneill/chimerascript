use crate::abstract_syntax_tree::{AssertCommand, AssertSubCommand, Value};
use crate::err_handle::{ChimeraRuntimeFailure, VarTypes};
use crate::frontend::Context;
use crate::literal::{Collection, DataKind};
use crate::variable_map::VariableMap;
use std::ops::Deref;

pub fn assert_command(
    context: &Context,
    assert_command: &AssertCommand,
    variable_map: &VariableMap,
) -> Result<(), ChimeraRuntimeFailure> {
    let left_binding = assert_command.left_value.resolve(context, variable_map)?;
    let left_data = left_binding.borrow(context)?;
    let right_binding = assert_command.right_value.resolve(context, variable_map)?;
    let right_data = right_binding.borrow(context)?;
    let assertion_passed = match assert_command.subcommand {
        AssertSubCommand::Length => {
            let assert_len = right_data.try_into_usize(&assert_command.right_value, context)?;
            let vec = left_data.try_into_list(assert_command.left_value.error_print(), context)?;
            vec.len() == assert_len
        }
        AssertSubCommand::Equals => left_data.deref() == right_data.deref(),
        AssertSubCommand::Status => {
            let res = match left_data.deref() {
                DataKind::Collection(Collection::Object(obj)) => match obj.get("status_code") {
                    Some(status_code) => {
                        let expected_code =
                            right_data.try_into_u64(&assert_command.right_value, context)?;
                        let status_as_num = status_code
                            .borrow(context)?
                            .deref()
                            .try_into_u64(&assert_command.left_value, context)?;
                        Some(expected_code == status_as_num)
                    }
                    None => None,
                },
                _ => None,
            };
            match res {
                Some(b) => b,
                None => {
                    return Err(ChimeraRuntimeFailure::VarWrongType(
                        assert_command.left_value.error_print(),
                        VarTypes::HttpResponse,
                        context.current_line,
                    ))
                }
            }
        }
        AssertSubCommand::Contains => match left_data.deref() {
            DataKind::Collection(c) => c.contains(right_data, context)?,
            _ => {
                return Err(ChimeraRuntimeFailure::VarWrongType(
                    assert_command.left_value.error_print(),
                    VarTypes::Containable,
                    context.current_line,
                ))
            }
        },
        _ => {
            // The remaining matches are the four relational operators, left and right must both be numbers
            let numeric_left =
                left_data.try_into_number_kind(&assert_command.left_value, context)?;
            let numeric_right =
                right_data.try_into_number_kind(&assert_command.right_value, context)?;
            match assert_command.subcommand {
                AssertSubCommand::GTE => numeric_left >= numeric_right,
                AssertSubCommand::GT => numeric_left > numeric_right,
                AssertSubCommand::LTE => numeric_left <= numeric_right,
                AssertSubCommand::LT => numeric_left < numeric_right,
                _ => panic!("Failed to handle an ASSERT subcommand case"),
            }
        }
    };
    let left_val_error_message = match &assert_command.left_value {
        Value::Literal(literal_val) => format!("value {}", literal_val),
        Value::Variable(var_name) => {
            format!("var '{}' with value '{}'", var_name, left_data.deref())
        }
    };
    if assert_command.negate_assertion && assertion_passed {
        // Assertion was true but expected to be false
        return Err(ChimeraRuntimeFailure::TestFailure(
            format!(
                "Expected {} to not {} {}",
                left_val_error_message,
                assert_command.subcommand,
                assert_command.right_value.error_print()
            ),
            context.current_line,
        ));
    } else if !assert_command.negate_assertion && !assertion_passed {
        // Assertion was false but expected to be true
        return Err(ChimeraRuntimeFailure::TestFailure(
            format!(
                "Expected {} to {} {}",
                left_val_error_message,
                assert_command.subcommand,
                assert_command.right_value.error_print()
            ),
            context.current_line,
        ));
    }
    Ok(())
}
