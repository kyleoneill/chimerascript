use std::collections::HashMap;
use crate::abstract_syntax_tree::{AssignmentValue, Value};
use crate::err_handle::ChimeraRuntimeFailure;
use crate::frontend::{Context, TestCase};

pub fn print_command(context: &Context, print_cmd: Value, variable_map: &HashMap<String, AssignmentValue>, depth: u32) -> Result <(), ChimeraRuntimeFailure> {
    let resolved = print_cmd.resolve(context, variable_map)?;
    TestCase::print_in_test(&format!("{}", resolved), depth);
    Ok(())
}
